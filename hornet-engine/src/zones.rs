//! Secondary-zone board control (PITCH-secondary-zones.md).
//!
//! Nine 2×2 zones on the 14×14 cross board. Control = friendly vs enemy reachers per zone.
//! This is a **measurement module** — it computes zone control but does NOT weight it into
//! eval yet. The pitch says: measure correlation with game outcome first, then decide.
//!
//! Hard Rule #4: if validated, zone control folds into Pᵢ (positional) as an internal
//! sub-readout, not a 5th component.

use crate::board::types::{Player, Square};
use crate::lines::LineMap;
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// Zone geometry — nine 2×2 blocks
// ---------------------------------------------------------------------------

/// A 2×2 zone defined by its top-left square.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zone {
    pub name: &'static str,
    pub top_left: Square,
}

impl Zone {
    /// The four squares in this 2×2 zone.
    pub fn squares(&self) -> [Square; 4] {
        let tl = self.top_left;
        [
            tl,
            Square::from_rank_file(tl.rank(), tl.file() + 1),
            Square::from_rank_file(tl.rank() + 1, tl.file()),
            Square::from_rank_file(tl.rank() + 1, tl.file() + 1),
        ]
    }
}

/// The nine secondary zones (established geometry from PITCH-secondary-zones.md).
/// Square indices: rank * 14 + file.
pub const ZONES: [Zone; 9] = [
    // Center: g7 h7 g8 h8 → top-left = g7 = (rank 6, file 6) → idx = 6*14+6 = 90
    Zone {
        name: "Center",
        top_left: Square::new(90),
    },
    // Gate W: c7 d7 c8 d8 → top-left = c7 = (rank 6, file 2) → idx = 6*14+2 = 86
    Zone {
        name: "GateW",
        top_left: Square::new(86),
    },
    // Gate E: k7 l7 k8 l8 → top-left = k7 = (rank 6, file 10) → idx = 6*14+10 = 94
    Zone {
        name: "GateE",
        top_left: Square::new(94),
    },
    // Gate S: g3 h3 g4 h4 → top-left = g3 = (rank 2, file 6) → idx = 2*14+6 = 34
    Zone {
        name: "GateS",
        top_left: Square::new(34),
    },
    // Gate N: g11 h11 g12 h12 → top-left = g11 = (rank 10, file 6) → idx = 10*14+6 = 146
    Zone {
        name: "GateN",
        top_left: Square::new(146),
    },
    // Quadrant SW: e5 f5 e6 f6 → top-left = e5 = (rank 4, file 4) → idx = 4*14+4 = 60
    Zone {
        name: "QuadSW",
        top_left: Square::new(60),
    },
    // Quadrant SE: i5 j5 i6 j6 → top-left = i5 = (rank 4, file 8) → idx = 4*14+8 = 64
    Zone {
        name: "QuadSE",
        top_left: Square::new(64),
    },
    // Quadrant NW: e9 f9 e10 f10 → top-left = e9 = (rank 8, file 4) → idx = 8*14+4 = 116
    Zone {
        name: "QuadNW",
        top_left: Square::new(116),
    },
    // Quadrant NE: i9 j9 i10 j10 → top-left = i9 = (rank 8, file 8) → idx = 8*14+8 = 120
    Zone {
        name: "QuadNE",
        top_left: Square::new(120),
    },
];

// ---------------------------------------------------------------------------
// Zone control
// ---------------------------------------------------------------------------

/// Control of one zone: friendly vs enemy reachers summed over the zone's 4 squares.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZoneControl {
    pub zone: Zone,
    /// Net control per player: positive = more friendly reachers than enemies.
    pub net: [i16; 4],
    /// Friendly reacher count per player.
    pub friendly: [u8; 4],
    /// Enemy reacher count per player (sum of all opponents).
    pub enemy: [u8; 4],
}

/// Compute control for all nine zones.
pub fn compute_zone_control(lines: &LineMap) -> [ZoneControl; 9] {
    let mut controls = [ZoneControl {
        zone: ZONES[0],
        net: [0; 4],
        friendly: [0; 4],
        enemy: [0; 4],
    }; 9];

    for (zi, zone) in ZONES.iter().enumerate() {
        controls[zi].zone = *zone;

        for sq in zone.squares() {
            let sr = lines.reachers_at(sq);
            for i in 0..sr.count {
                let pi = sr.piece_indices[i as usize] as usize;
                if pi >= lines.piece_count {
                    continue;
                }
                let pl = &lines.pieces[pi];
                let pidx = pl.player.index();
                controls[zi].friendly[pidx] += 1;
                for opp in pl.player.opponents() {
                    controls[zi].enemy[opp.index()] += 1;
                }
            }
        }

        for player in Player::ALL {
            let i = player.index();
            controls[zi].net[i] = controls[zi].friendly[i] as i16 - controls[zi].enemy[i] as i16;
        }
    }

    controls
}

/// Aggregate zone control per player: sum of net control across all zones.
/// Positive = player controls more zones than they lose.
pub fn aggregate_zone_control(lines: &LineMap) -> [i16; 4] {
    let controls = compute_zone_control(lines);
    let mut total = [0i16; 4];
    for zc in controls.iter() {
        for player in Player::ALL {
            total[player.index()] += zc.net[player.index()];
        }
    }
    total
}

// ---------------------------------------------------------------------------
// Geometry-flow control (the board's intrinsic lanes; tools/board_flow.py)
// ---------------------------------------------------------------------------

/// Structural slider-flow per square — **board geometry, zero games**: how many empty-board slider
/// moves cross each square (`2 rooks + 2 bishops + 1 queen` = `3·(rook+bishop)` crossings). This is the
/// board's intrinsic "lane" map; real human transit follows it at r≈0.80 (`tools/board_flow.py`). High
/// flow = a square pieces must cross to travel — a derived alternative to the hand-picked zones.
pub static FLOW: LazyLock<[i16; 196]> = LazyLock::new(|| {
    let playable = |r: i32, f: i32| {
        (0..14).contains(&r) && (0..14).contains(&f) && !((r < 3 || r > 10) && (f < 3 || f > 10))
    };
    let run = |r: i32, f: i32, dr: i32, df: i32| {
        let (mut n, mut rr, mut ff) = (0i32, r + dr, f + df);
        while playable(rr, ff) {
            n += 1;
            rr += dr;
            ff += df;
        }
        n
    };
    let mut flow = [0i16; 196];
    for r in 0..14 {
        for f in 0..14 {
            if !playable(r, f) {
                continue;
            }
            let (e, w, n, s) = (
                run(r, f, 0, 1),
                run(r, f, 0, -1),
                run(r, f, 1, 0),
                run(r, f, -1, 0),
            );
            let (ne, sw, nw, se) = (
                run(r, f, 1, 1),
                run(r, f, -1, -1),
                run(r, f, 1, -1),
                run(r, f, -1, 1),
            );
            let rook = e * w + n * s; // horizontal + vertical crossings
            let bishop = ne * sw + nw * se; // the two diagonals
            flow[(r * 14 + f) as usize] = (3 * (rook + bishop)) as i16;
        }
    }
    flow
});

/// Flow-control per player: each player's flow-weighted reach over the board —
/// `Σ_sq FLOW[sq] × (that player's reachers of sq)`. Values controlling the board's lanes. A
/// geometry-derived spatial prior (continuous, principled) vs the hand-picked zones. Gate on self-play.
pub fn query_flow_control(lines: &LineMap) -> [i16; 4] {
    let mut ctrl = [0i64; 4];
    for idx in 0..196u8 {
        let flow = FLOW[idx as usize] as i64;
        if flow == 0 {
            continue;
        }
        let sr = lines.reachers_at(Square::new(idx));
        for k in 0..sr.count {
            let pi = sr.piece_indices[k as usize] as usize;
            if pi >= lines.piece_count {
                continue;
            }
            ctrl[lines.pieces[pi].player.index()] += flow;
        }
    }
    [
        (ctrl[0] / 256) as i16,
        (ctrl[1] / 256) as i16,
        (ctrl[2] / 256) as i16,
        (ctrl[3] / 256) as i16,
    ]
}

// ---------------------------------------------------------------------------
// Comparison with flat centrality
// ---------------------------------------------------------------------------

/// Correlation between zone control and flat centrality for a position.
/// Returns Pearson-like correlation (not exact Pearson, just dot-product normalized).
/// This is for measurement, not eval.
pub fn zone_vs_centrality_correlation(zone_ctrl: &[i16; 4], centrality: &[i16; 4]) -> f32 {
    let z_sum: i32 = zone_ctrl.iter().map(|&x| x as i32).sum();
    let c_sum: i32 = centrality.iter().map(|&x| x as i32).sum();
    let z_mean = z_sum as f32 / 4.0;
    let c_mean = c_sum as f32 / 4.0;

    let mut num = 0.0;
    let mut z_den = 0.0;
    let mut c_den = 0.0;

    for i in 0..4 {
        let zd = zone_ctrl[i] as f32 - z_mean;
        let cd = centrality[i] as f32 - c_mean;
        num += zd * cd;
        z_den += zd * zd;
        c_den += cd * cd;
    }

    let den = (z_den * c_den).sqrt();
    if den == 0.0 { 0.0 } else { num / den }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::fen4;
    use crate::lines::{LineMap, compute_lines};

    fn start() -> crate::board::Board {
        fen4::parse(fen4::START_FEN4).unwrap()
    }

    #[test]
    fn zones_have_four_squares_each() {
        for zone in ZONES.iter() {
            let sqs = zone.squares();
            assert_eq!(sqs.len(), 4, "zone {} should have 4 squares", zone.name);
            // All squares should be valid (not in dead corners)
            for sq in sqs {
                assert!(
                    sq.is_valid(),
                    "zone {} square {} should be valid",
                    zone.name,
                    sq
                );
            }
        }
    }

    #[test]
    fn flow_center_high_and_control_symmetric_at_start() {
        // Geometry: the center carries the most slider-flow, an edge square far less.
        let center = 6 * 14 + 6; // g7
        let edge = 3; // d1 (rank 0, file 3) — short rays
        assert!(
            FLOW[center] > FLOW[edge] * 2,
            "center flow {} should dwarf edge {}",
            FLOW[center],
            FLOW[edge]
        );
        // The 4 players are rotations of each other at the start → ~symmetric flow-control.
        let b = start();
        let mut lm = LineMap::new();
        compute_lines(&b, &mut lm);
        let fc = query_flow_control(&lm);
        let avg = fc.iter().map(|&x| x as i32).sum::<i32>() as f32 / 4.0;
        for (i, &c) in fc.iter().enumerate() {
            assert!(
                (c as f32 - avg).abs() < avg.abs() * 0.2 + 50.0,
                "player {i} flow-control {c} deviates too far from {avg}"
            );
        }
    }

    #[test]
    fn start_position_symmetric_zone_control() {
        let b = start();
        let mut lm = LineMap::new();
        compute_lines(&b, &mut lm);
        let agg = aggregate_zone_control(&lm);

        // At start, all players should have roughly equal zone control
        let avg = agg.iter().map(|&x| x as i32).sum::<i32>() as f32 / 4.0;
        for (i, &ctrl) in agg.iter().enumerate() {
            let diff = (ctrl as f32 - avg).abs();
            assert!(
                diff < 20.0,
                "player {i} zone control {ctrl} deviates {diff} from avg {avg}"
            );
        }
    }

    #[test]
    fn center_zone_exists() {
        let center = &ZONES[0];
        assert_eq!(center.name, "Center");
        let sqs = center.squares();
        assert_eq!(sqs[0].to_algebraic(), "g7");
        assert_eq!(sqs[1].to_algebraic(), "h7");
        assert_eq!(sqs[2].to_algebraic(), "g8");
        assert_eq!(sqs[3].to_algebraic(), "h8");
    }

    #[test]
    fn zone_control_counts_reachers() {
        let b = start();
        let mut lm = LineMap::new();
        compute_lines(&b, &mut lm);
        let controls = compute_zone_control(&lm);

        // Center zone should have many reachers (all pieces influence center)
        let center = &controls[0];
        let total_reachers: u8 = center.friendly.iter().sum();
        assert!(total_reachers > 0, "center zone should have reachers");
    }
}
