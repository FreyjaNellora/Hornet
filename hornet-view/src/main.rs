//! Hornet viewer — a NATIVE graphical observer for the 4-player-chess engine.
//!
//! **The viewer never decides anything.** It spawns the `hornet-engine` binary and drives it over
//! the protocol; the engine owns all game state and decisions (legality, the move it plays, the DKW
//! lifecycle, draws). The viewer only: deserializes the position the engine reports (`fen4`) to draw
//! it, highlights the engine-provided legal set (`legal`) for a clicked piece, relays the human's
//! click as a `move`, and renders the engine's thinking (`info`/MultiPV) in collapsible panels.
//!
//! Run: `cargo run -p hornet-view --release` (builds and spawns `hornet-engine` from the same dir).
//!
//! (No `windows_subsystem = "windows"` yet — during bring-up we want a console so startup errors
//! are visible. Add it once stable for a clean double-click launch.)

use eframe::egui;
use hornet_engine::board::types::{PieceType, Player};
use hornet_engine::board::{Board, Square, fen4};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::thread;

// Theme: teal panels, reddish-orange borders, light text (per the user's spec).
// Darker teal = stronger contrast so ALL the light text reads clearly (not just a few elements).
const TEAL: egui::Color32 = egui::Color32::from_rgb(8, 52, 56);
const TEAL_CARD: egui::Color32 = egui::Color32::from_rgb(5, 38, 42);
const TEAL_ACTIVE: egui::Color32 = egui::Color32::from_rgb(13, 78, 84);
const ORANGE: egui::Color32 = egui::Color32::from_rgb(236, 112, 64);
const INK: egui::Color32 = egui::Color32::from_rgb(247, 248, 243); // near-white primary text
const SOFT: egui::Color32 = egui::Color32::from_rgb(214, 230, 224); // bright secondary text

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([1100.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Hornet 4PC — viewer",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

/// Load a font that contains the Unicode chess glyphs (U+2654–265F) and add it as a fallback in the
/// proportional family, so the board can draw real piece shapes (♚♛♜♝♞♟) instead of letters. egui's
/// bundled fonts don't include them; we borrow a system font (Segoe UI Symbol / DejaVu). Returns
/// whether a glyph font was found.
fn setup_fonts(ctx: &egui::Context) -> bool {
    let candidates = [
        "C:/Windows/Fonts/seguisym.ttf",   // Segoe UI Symbol (Windows)
        "C:/Windows/Fonts/SEGUISYM.TTF",
        "C:/Windows/Fonts/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/Library/Fonts/Arial Unicode.ttf",
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts
                .font_data
                .insert("chess".to_owned(), Arc::new(egui::FontData::from_owned(bytes)));
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("chess".to_owned());
            ctx.set_fonts(fonts);
            return true;
        }
    }
    false
}

/// A spawned engine process + a channel of its stdout lines (read on a background thread).
struct Engine {
    _child: Child,
    stdin: ChildStdin,
    rx: Receiver<String>,
}

impl Engine {
    fn spawn() -> std::io::Result<Engine> {
        // The engine binary is a sibling of this viewer in the same target dir.
        let exe = std::env::current_exe()?
            .parent()
            .unwrap()
            .join(format!("hornet-engine{}", std::env::consts::EXE_SUFFIX));
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(l) = line else { break };
                if tx.send(l).is_err() {
                    break;
                }
            }
        });
        Ok(Engine {
            _child: child,
            stdin,
            rx,
        })
    }

    fn send(&mut self, cmd: &str) {
        let _ = writeln!(self.stdin, "{cmd}");
        let _ = self.stdin.flush();
    }
}

/// One ranked root candidate from the engine's MultiPV telemetry.
struct Cand {
    piece: String, // the moving piece's letter (P/N/B/R/Q/K)
    mv: String,
    score: [i16; 4],
}

struct App {
    engine: Option<Engine>,
    err: Option<String>,
    // Authoritative game state (all set from engine output — never computed here).
    board: Board,
    legal: Vec<(Square, Square)>,
    side: Player,
    points: [u16; 4],
    dead: [bool; 4],
    dkw: [bool; 4],
    state: String,
    // Telemetry (the engine's thinking).
    candidates: Vec<Cand>,
    pv: Vec<String>,
    depth: u32,
    nodes: u64,
    nps: u64,
    time_ms: u64,
    eval_rows: Vec<(String, [i16; 4])>,
    king_rows: Vec<String>,
    log: Vec<String>,
    // Interaction.
    human: Player,
    selected: Option<Square>,
    awaiting: bool, // a command is in flight; wait for the next `status` before acting
    depth_setting: u32,
    glyphs: bool, // a chess-glyph font loaded → draw piece shapes, else fall back to letters
    move_history: Vec<(Player, String)>, // the played-move record (mover, from-to)
    paused: bool,    // engine auto-play halted, so you can examine/report mid-game
    step_once: bool, // let exactly one engine move through while paused (the Step button)
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> App {
        let glyphs = setup_fonts(&cc.egui_ctx);
        setup_theme(&cc.egui_ctx);
        let (engine, err) = match Engine::spawn() {
            Ok(mut e) => {
                e.send("newgame");
                (Some(e), None)
            }
            Err(e) => (
                None,
                Some(format!(
                    "Could not start the engine. Build both binaries first:  cargo build --release  \
                     — then relaunch with  cargo run -p hornet-view --release.  ({e})"
                )),
            ),
        };
        App {
            engine,
            err,
            board: fen4::parse(fen4::START_FEN4).unwrap(),
            legal: Vec::new(),
            side: Player::Red,
            points: [0; 4],
            dead: [false; 4],
            dkw: [false; 4],
            state: "ongoing".into(),
            candidates: Vec::new(),
            pv: Vec::new(),
            depth: 0,
            nodes: 0,
            nps: 0,
            time_ms: 0,
            eval_rows: Vec::new(),
            king_rows: Vec::new(),
            log: Vec::new(),
            human: Player::Red,
            selected: None,
            awaiting: false,
            depth_setting: 8,
            glyphs,
            move_history: Vec::new(),
            paused: false,
            step_once: false,
        }
    }

    /// Drain engine output, update state, then drive the next engine move if it's an engine seat.
    fn pump(&mut self) {
        let mut lines = Vec::new();
        if let Some(e) = &self.engine {
            while let Ok(l) = e.rx.try_recv() {
                lines.push(l);
            }
        }
        for l in lines {
            self.handle_line(&l);
            self.log.push(l);
        }
        if self.log.len() > 400 {
            let drop = self.log.len() - 400;
            self.log.drain(0..drop);
        }
        // Drive: if it's an engine seat's turn and nothing is in flight, ask it to move — unless
        // paused (then only a Step lets one move through, so you can freeze and report).
        if self.state == "ongoing" && !self.awaiting {
            let human_turn = self.side == self.human && !self.dkw[self.human.index()];
            if !human_turn && (!self.paused || self.step_once) {
                self.step_once = false;
                let d = self.depth_setting;
                self.candidates.clear();
                self.send(&format!("go depth {d}"));
                self.awaiting = true;
            }
        }
    }

    fn send(&mut self, cmd: &str) {
        if let Some(e) = &mut self.engine {
            e.send(cmd);
        }
    }

    fn handle_line(&mut self, line: &str) {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("fen4") => {
                let rest = line.strip_prefix("fen4 ").unwrap_or("").trim();
                if let Ok(b) = fen4::parse(rest) {
                    self.board = b;
                }
            }
            Some("legal") => {
                self.legal = it.filter_map(parse_pair).collect();
            }
            Some("status") => {
                self.parse_status(line);
                self.awaiting = false;
                // Refresh the render data from the authoritative new position.
                self.send("board");
                self.send("legal");
                self.send("eval");
            }
            Some("info") => self.parse_info(line),
            Some("illegal") => self.selected = None,
            // The mover is the current `side` (the `status` that advances it comes next in the batch).
            Some("bestmove") | Some("moved") => {
                if let Some(mv) = it.next()
                    && mv != "(none)"
                {
                    self.move_history.push((self.side, mv.to_string()));
                }
            }
            _ => {}
        }
    }

    fn parse_status(&mut self, line: &str) {
        let t: Vec<&str> = line.split_whitespace().collect();
        let after = |key: &str| t.iter().position(|&x| x == key).map(|i| i + 1);
        if let Some(i) = after("side") {
            self.side = parse_player(t[i]).unwrap_or(self.side);
        }
        if let Some(i) = after("points") {
            for s in 0..4 {
                if let Some(v) = t.get(i + 1 + s * 2).and_then(|x| x.parse().ok()) {
                    self.points[s] = v;
                }
            }
        }
        if let Some(i) = after("dead") {
            for s in 0..4 {
                self.dead[s] = t.get(i + s).map(|&x| x == "1").unwrap_or(false);
            }
        }
        if let Some(i) = after("dkw") {
            for s in 0..4 {
                self.dkw[s] = t.get(i + s).map(|&x| x == "1").unwrap_or(false);
            }
        }
        if let Some(i) = after("state") {
            self.state = t.get(i).unwrap_or(&"ongoing").to_string();
        }
    }

    fn parse_info(&mut self, line: &str) {
        let t: Vec<&str> = line.split_whitespace().collect();
        let after = |key: &str| t.iter().position(|&x| x == key).map(|i| i + 1);
        let score_at = |i: usize| -> [i16; 4] {
            let g = |k: usize| t.get(i + k).and_then(|x| x.parse().ok()).unwrap_or(0);
            [g(1), g(3), g(5), g(7)] // "R r B b Y y G g"
        };
        match t.get(1).copied() {
            Some("depth") if t.contains(&"multipv") => {
                let rank = after("multipv").and_then(|i| t[i].parse::<usize>().ok()).unwrap_or(0);
                let score = after("score").map(score_at).unwrap_or([0; 4]);
                let mv = after("move").map(|i| t[i].to_string()).unwrap_or_default();
                let piece = after("piece").map(|i| t[i].to_string()).unwrap_or_default();
                if rank == 1 {
                    self.candidates.clear();
                    self.depth = after("depth").and_then(|i| t[i].parse().ok()).unwrap_or(0);
                    self.nodes = after("nodes").and_then(|i| t[i].parse().ok()).unwrap_or(0);
                    self.nps = after("nps").and_then(|i| t[i].parse().ok()).unwrap_or(0);
                    self.time_ms = after("time").and_then(|i| t[i].parse().ok()).unwrap_or(0);
                    self.pv = after("pv").map(|i| t[i..].iter().map(|s| s.to_string()).collect()).unwrap_or_default();
                }
                self.candidates.push(Cand { piece, mv, score });
            }
            Some("eval") => {
                if let (Some(name), Some(i)) = (t.get(2), after("R")) {
                    self.eval_rows.push((name.to_string(), score_at(i - 1)));
                    if self.eval_rows.len() > 4 {
                        let n = self.eval_rows.len();
                        self.eval_rows.drain(0..n - 4); // keep the latest 4 component rows
                    }
                }
            }
            Some("kingsafety") => {
                if self.king_rows.len() >= 4 {
                    self.king_rows.clear();
                }
                self.king_rows.push(line.trim_start_matches("info ").to_string());
            }
            _ => {}
        }
    }

    fn on_click(&mut self, sq: Square) {
        let human_turn =
            self.state == "ongoing" && self.side == self.human && !self.dkw[self.human.index()];
        if self.awaiting || !human_turn {
            return;
        }
        if let Some(from) = self.selected {
            if self.legal.iter().any(|&(f, t)| f == from && t == sq) {
                self.send(&format!(
                    "move {}-{}",
                    from.to_algebraic(),
                    sq.to_algebraic()
                ));
                self.selected = None;
                self.candidates.clear();
                self.awaiting = true;
                return;
            }
        }
        self.selected = self
            .board
            .piece_at(sq)
            .filter(|p| p.player == self.human)
            .map(|_| sq);
    }

    /// A copy-pasteable debug snapshot: the exact position (FEN4), whose turn, the full legal-move
    /// set, the engine's candidates, and the recent protocol log — everything needed to diagnose
    /// "why can't I make this move?" off-line.
    fn debug_snapshot(&self) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        let _ = writeln!(s, "fen4 {}", fen4::serialize(&self.board));
        let _ = writeln!(
            s,
            "side {:?}  state {}  points {:?}  paused {}",
            self.side, self.state, self.points, self.paused
        );
        if let Some(sel) = self.selected {
            let _ = writeln!(s, "selected {}", sel.to_algebraic());
        }
        s.push_str("legal:");
        for (f, t) in &self.legal {
            let _ = write!(s, " {}-{}", f.to_algebraic(), t.to_algebraic());
        }
        s.push_str("\ncandidates:\n");
        for c in self.candidates.iter().take(20) {
            let _ = writeln!(s, "  {} {:?}", c.mv, c.score);
        }
        s.push_str("recent log:\n");
        let start = self.log.len().saturating_sub(30);
        for l in &self.log[start..] {
            let _ = writeln!(s, "  {l}");
        }
        s
    }

    fn draw_board(&mut self, ui: &mut egui::Ui) {
        const LBL: f32 = 18.0; // gutter for rank (left) and file (bottom) labels
        // The board SCALES to the space left between the panels, clamped to a readable range — so it
        // never overflows or gets clipped at any window size.
        let avail = ui.available_size();
        let cell = ((avail.x.min(avail.y) - LBL - 6.0) / 14.0).clamp(26.0, 46.0);
        let board = cell * 14.0;
        let (resp, painter) =
            ui.allocate_painter(egui::Vec2::new(board + LBL, board + LBL), egui::Sense::click());
        let o = resp.rect.min;
        let (bx, by) = (o.x + LBL, o.y); // board origin (after the left rank-label gutter)
        for rank in 0..14u8 {
            for file in 0..14u8 {
                let sq = Square::from_rank_file(rank, file);
                if !sq.is_valid() {
                    continue;
                }
                let rect = egui::Rect::from_min_size(
                    egui::pos2(bx + file as f32 * cell, by + (13 - rank) as f32 * cell),
                    egui::Vec2::splat(cell),
                );
                let mut fill = if (rank + file) % 2 == 0 {
                    egui::Color32::from_gray(170)
                } else {
                    egui::Color32::from_gray(135)
                };
                if self.selected == Some(sq) {
                    fill = egui::Color32::from_rgb(116, 156, 208);
                }
                painter.rect_filled(rect, egui::CornerRadius::ZERO, fill);
                if self.selected.is_some_and(|f| self.legal.iter().any(|&(a, b)| a == f && b == sq)) {
                    painter.circle_filled(
                        rect.center(),
                        cell * 0.16,
                        egui::Color32::from_rgba_unmultiplied(20, 110, 20, 200),
                    );
                }
                if let Some(p) = self.board.piece_at(sq) {
                    let (text, size) = if self.glyphs {
                        (piece_glyph(p.piece_type), cell * 0.82)
                    } else {
                        (piece_letter(p.piece_type), cell * 0.62)
                    };
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        text,
                        egui::FontId::proportional(size),
                        seat_color(p.player),
                    );
                }
            }
        }
        // Coordinate labels: files a–n along the bottom, ranks 1–14 down the left gutter.
        let lf = egui::FontId::proportional(13.0);
        for file in 0..14u8 {
            painter.text(
                egui::pos2(bx + file as f32 * cell + cell / 2.0, by + board + LBL / 2.0),
                egui::Align2::CENTER_CENTER,
                ((b'a' + file) as char).to_string(),
                lf.clone(),
                SOFT,
            );
        }
        for rank in 0..14u8 {
            painter.text(
                egui::pos2(o.x + LBL / 2.0, by + (13 - rank) as f32 * cell + cell / 2.0),
                egui::Align2::CENTER_CENTER,
                (rank + 1).to_string(),
                lf.clone(),
                SOFT,
            );
        }
        if resp.clicked()
            && let Some(pos) = resp.interact_pointer_pos()
        {
            let file = ((pos.x - bx) / cell).floor() as i32;
            let rank = 13 - ((pos.y - by) / cell).floor() as i32;
            if (0..14).contains(&file) && (0..14).contains(&rank) {
                let sq = Square::from_rank_file(rank as u8, file as u8);
                if sq.is_valid() {
                    self.on_click(sq);
                }
            }
        }
    }

    /// LEFT column — PLAY: controls, whose turn, points per player, and the move list.
    fn draw_play_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading(egui::RichText::new("Hornet 4PC").color(ORANGE).strong());
        if let Some(e) = &self.err {
            ui.colored_label(egui::Color32::from_rgb(255, 140, 120), e);
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.label("You:");
            for (p, name) in SEATS {
                if ui
                    .selectable_label(self.human == p, egui::RichText::new(name).color(seat_color(p)))
                    .clicked()
                {
                    self.human = p;
                    self.selected = None;
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Depth:").size(15.0).strong().color(ORANGE));
            // Big, bold, bright digits; the selected one is near-white over the orange highlight,
            // the rest soft — so which depth is active reads at a glance.
            for d in [4u32, 8, 12, 16] {
                let sel = self.depth_setting == d;
                ui.selectable_value(
                    &mut self.depth_setting,
                    d,
                    egui::RichText::new(d.to_string())
                        .size(16.0)
                        .strong()
                        .color(if sel { INK } else { SOFT }),
                );
            }
            if ui.button("New game").clicked() {
                self.send("newgame");
                self.selected = None;
                self.move_history.clear();
                self.paused = false;
            }
        });
        ui.horizontal_wrapped(|ui| {
            // Pause freezes the engine's auto-play so you can examine / send feedback in real time.
            if ui
                .button(if self.paused { "Resume" } else { "Pause" })
                .clicked()
            {
                self.paused = !self.paused;
            }
            if ui.button("Step").on_hover_text("advance the engine one move").clicked() {
                self.step_once = true;
            }
            if ui
                .button("Refresh")
                .on_hover_text("re-sync the board, status, and engine data")
                .clicked()
            {
                self.send("board");
                self.send("legal");
                self.send("status");
                self.send("eval");
            }
            if ui
                .button("Copy report")
                .on_hover_text("copy a debug snapshot (position, legal moves, candidates, log) — paste it to the dev")
                .clicked()
            {
                let snap = self.debug_snapshot();
                ui.ctx().copy_text(snap);
            }
        });
        let turn = if self.state != "ongoing" {
            format!("game {}", self.state)
        } else if self.side == self.human {
            "your move".into()
        } else if self.paused {
            format!("{:?} (engine) — paused", self.side)
        } else {
            format!("{:?} (engine) thinking…", self.side)
        };
        ui.label(egui::RichText::new(turn).italics().color(SOFT));

        section(ui, "POINTS");
        desc(ui, "the score — won by points (captures + eliminations), not checkmate.");
        for (i, (p, name)) in SEATS.iter().enumerate() {
            let dead = self.dead[i];
            let active = self.side == *p && !dead && self.state == "ongoing";
            egui::Frame::new()
                .fill(if active { TEAL_ACTIVE } else { TEAL_CARD })
                .stroke(egui::Stroke::new(1.0, seat_color(*p)))
                .inner_margin(egui::Margin::symmetric(8, 3))
                .corner_radius(egui::CornerRadius::same(3))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let mut label = egui::RichText::new(*name).color(seat_color(*p)).strong();
                        if dead {
                            label = label.strikethrough();
                        }
                        ui.label(label);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.monospace(
                                egui::RichText::new(self.points[i].to_string())
                                    .strong()
                                    .color(if active { ORANGE } else { INK }),
                            );
                        });
                    });
                });
            ui.add_space(2.0);
        }

        section(ui, "MOVES");
        desc(ui, "the game record — each row is one round: Red · Blue · Yellow · Green.");
        egui::ScrollArea::vertical()
            .id_salt("moves")
            .auto_shrink([false, false])
            .stick_to_bottom(true) // keep the newest round in view; wheel-scroll up to review
            .drag_to_scroll(false) // drag selects text; scroll with the wheel/scrollbar
            .show(ui, |ui| {
                if self.move_history.is_empty() {
                    ui.label(egui::RichText::new("(no moves yet)").color(SOFT));
                }
                for (r, chunk) in self.move_history.chunks(4).enumerate() {
                    let moves: Vec<&str> = chunk.iter().map(|(_, m)| m.as_str()).collect();
                    ui.monospace(format!("{:>2}. {}", r + 1, moves.join("   ")));
                }
            });
    }

    /// RIGHT column — the engine's mind: STUDY (its reasoning) on top, DEBUG (the mechanics) below.
    fn draw_engine_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading(egui::RichText::new("Engine").color(ORANGE).strong());
        egui::ScrollArea::vertical()
            .id_salt("engine")
            .auto_shrink([false, false])
            .drag_to_scroll(false) // drag selects text; scroll with the wheel/scrollbar
            .show(ui, |ui| {
                section(ui, "STUDY — what it's thinking");
                egui::CollapsingHeader::new(head(&format!("Candidates ({})", self.candidates.len())))
                    .default_open(true)
                    .show(ui, |ui| {
                        desc(ui, "every move it weighed, best first. The R/B/Y/G columns are each player's score if that move is played; it picks the one with ITS own score highest. Row 1 = the move it made.");
                        egui::Grid::new("candidates_grid")
                            .striped(true)
                            .num_columns(7)
                            .min_col_width(30.0)
                            .show(ui, |ui| {
                                ui.label(head("#"));
                                ui.label(head("pc"));
                                ui.label(head("move"));
                                for (p, l) in [
                                    (Player::Red, "R"),
                                    (Player::Blue, "B"),
                                    (Player::Yellow, "Y"),
                                    (Player::Green, "G"),
                                ] {
                                    ui.label(egui::RichText::new(l).color(seat_color(p)).strong());
                                }
                                ui.end_row();
                                for (i, c) in self.candidates.iter().enumerate().take(20) {
                                    ui.monospace(format!("{}", i + 1));
                                    ui.monospace(c.piece.as_str());
                                    ui.monospace(c.mv.as_str());
                                    for s in c.score {
                                        ui.monospace(s.to_string());
                                    }
                                    ui.end_row();
                                }
                            });
                    });
                egui::CollapsingHeader::new(head("Expected line"))
                    .default_open(true)
                    .show(ui, |ui| {
                        desc(ui, "what it expects to follow if everyone plays their best (the \"principal variation\").");
                        ui.monospace(if self.pv.is_empty() {
                            "—".to_string()
                        } else {
                            self.pv.join("  ")
                        });
                    });
                egui::CollapsingHeader::new(head("Why this score"))
                    .show(ui, |ui| {
                        desc(ui, "the position's score per player: material = piece value, positional = squares controlled + measured piece-square tables, safety = king danger (penalty when attacked), crossfire = enemies ganging up. All four are active in the engine you're playing.");
                        ui.monospace(format!("    {:<11} {:>6} {:>6} {:>6} {:>6}", "", "R", "B", "Y", "G"));
                        for (name, v) in &self.eval_rows {
                            ui.monospace(format!("    {:<11} {:>6} {:>6} {:>6} {:>6}", name, v[0], v[1], v[2], v[3]));
                        }
                        for k in &self.king_rows {
                            ui.monospace(k);
                        }
                    });

                ui.add_space(10.0);
                ui.separator();
                // Debug tucked away in one closed section — open it only when tuning / investigating.
                egui::CollapsingHeader::new(head("Debug — how it's working"))
                    .default_open(false)
                    .show(ui, |ui| {
                        desc(ui, "search internals + the raw engine messages; open when tuning or if something looks off.");
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Search effort").size(14.0).strong().color(SOFT));
                        ui.monospace(format!(
                            "depth {}   nodes {}   nps {}   time {} ms",
                            self.depth, self.nodes, self.nps, self.time_ms
                        ));
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("Engine messages").size(14.0).strong().color(SOFT));
                        for l in self.log.iter().rev().take(60) {
                            ui.monospace(l);
                        }
                    });
            });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.pump();
        ui.ctx().request_repaint(); // keep polling the engine channel
        // 3-column: PLAY (left) · board (center) · ENGINE study+debug (right).
        egui::SidePanel::left("play")
            .frame(panel_frame())
            .exact_width(280.0)
            .show_inside(ui, |ui| self.draw_play_panel(ui));
        egui::SidePanel::right("engine")
            .frame(panel_frame())
            .default_width(400.0)
            .show_inside(ui, |ui| self.draw_engine_panel(ui));
        egui::CentralPanel::default()
            .frame(panel_frame())
            .show_inside(ui, |ui| self.draw_board(ui));
    }
}

const SEATS: [(Player, &str); 4] = [
    (Player::Red, "Red"),
    (Player::Blue, "Blue"),
    (Player::Yellow, "Yellow"),
    (Player::Green, "Green"),
];

/// Teal panels, reddish-orange borders, light ink text (the user's color scheme).
fn setup_theme(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = TEAL;
    v.window_fill = TEAL_CARD;
    v.faint_bg_color = TEAL_CARD;
    v.extreme_bg_color = TEAL_CARD;
    v.override_text_color = Some(INK);
    v.window_stroke = egui::Stroke::new(2.0, ORANGE);
    // Bright foreground (text) on every widget state so headers/buttons read clearly on teal.
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, INK);
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, INK);
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    v.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, ORANGE);
    v.widgets.inactive.bg_fill = TEAL_CARD;
    v.widgets.inactive.weak_bg_fill = TEAL_CARD;
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.5, ORANGE);
    v.widgets.active.bg_stroke = egui::Stroke::new(1.5, ORANGE);
    v.selection.bg_fill = ORANGE.gamma_multiply(0.6); // stronger highlight so a selected chip is unmistakable
    v.selection.stroke = egui::Stroke::new(1.0, ORANGE);
    ctx.set_visuals(v);
    ctx.style_mut(|s| {
        // Let any text be drag-highlighted and copied (Ctrl+C).
        s.interaction.selectable_labels = true;
        // One readable type scale for ALL text — so nothing defaults to dim/tiny.
        use egui::{FontFamily, FontId, TextStyle};
        s.text_styles = [
            (TextStyle::Heading, FontId::new(18.0, FontFamily::Proportional)),
            (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(14.0, FontFamily::Proportional)),
            (TextStyle::Small, FontId::new(13.0, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace)),
        ]
        .into();
    });
}

/// A teal panel with a reddish-orange border.
fn panel_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(TEAL)
        .stroke(egui::Stroke::new(2.0, ORANGE))
        .inner_margin(egui::Margin::same(10))
}

/// A small reddish-orange section heading.
fn section(ui: &mut egui::Ui, text: &str) {
    ui.add_space(6.0);
    ui.label(egui::RichText::new(text).size(15.0).strong().color(ORANGE));
    ui.add_space(2.0);
}

fn parse_pair(tok: &str) -> Option<(Square, Square)> {
    let (a, b) = tok.split_once('-')?;
    // a `from-to=PROMO` token: drop the promo suffix for the square pair.
    let b = b.split('=').next().unwrap_or(b);
    Some((Square::from_algebraic(a)?, Square::from_algebraic(b)?))
}

/// A readable one-line explanation shown under a panel header (plain language, not jargon).
fn desc(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(13.0).color(SOFT));
}

fn parse_player(s: &str) -> Option<Player> {
    match s {
        "Red" => Some(Player::Red),
        "Blue" => Some(Player::Blue),
        "Yellow" => Some(Player::Yellow),
        "Green" => Some(Player::Green),
        _ => None,
    }
}

fn piece_letter(t: PieceType) -> &'static str {
    match t {
        PieceType::Pawn => "P",
        PieceType::Knight => "N",
        PieceType::Bishop => "B",
        PieceType::Rook => "R",
        PieceType::Queen | PieceType::PromotedQueen => "Q",
        PieceType::King => "K",
    }
}

/// Filled Unicode chess glyphs (the "black" set), tinted per seat — recognizable piece shapes,
/// the same approach the web UIs use. Rendered when a glyph-capable font was found.
fn piece_glyph(t: PieceType) -> &'static str {
    match t {
        PieceType::Pawn => "\u{265F}",
        PieceType::Knight => "\u{265E}",
        PieceType::Bishop => "\u{265D}",
        PieceType::Rook => "\u{265C}",
        PieceType::Queen | PieceType::PromotedQueen => "\u{265B}",
        PieceType::King => "\u{265A}",
    }
}

/// Bright seat colors chosen to read on BOTH the teal panels and the gray board squares.
fn seat_color(p: Player) -> egui::Color32 {
    match p {
        Player::Red => egui::Color32::from_rgb(255, 110, 100),
        Player::Blue => egui::Color32::from_rgb(110, 170, 255),
        Player::Yellow => egui::Color32::from_rgb(244, 214, 92),
        Player::Green => egui::Color32::from_rgb(96, 224, 124),
    }
}

/// A bright, bold panel-header label (collapsible headers were washing out on teal).
fn head(t: &str) -> egui::RichText {
    egui::RichText::new(t).color(INK).strong()
}
