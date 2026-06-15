# Hornet

A four-player chess engine for **chess.com-style 4-player Free-For-All**, with a native viewer so you
can play against it and watch it think.

Under the hood it's built from one primitive — **per-piece line projection** feeding a query engine
that returns a per-player utility vector **V = ⟨U₁, U₂, U₃, U₄⟩** to a **Max^n** search. The
evaluation is a vector, never a scalar: search backs up each player's components without ever
collapsing them.

> **Status: pre-alpha.** It plays a full game and puts up a fight, but it's early and rough edges are
> expected. Feedback welcome.

---

## Play it

You need **Rust** (latest stable). Install it in one step from **<https://rustup.rs>** — that gives
you `cargo`, used below. Works on Windows, macOS, and Linux.

**1. Get the code**

```sh
git clone https://github.com/FreyjaNellora/Hornet.git
cd Hornet
```

(Or download the ZIP from the GitHub page and unzip it.)

**2. Build it** — compiles both the engine and the viewer (first build takes a few minutes):

```sh
cargo build --release
```

**3. Play:**

```sh
cargo run -p hornet-view --release
```

A window opens and the engine starts automatically. That's it.

> **Tip:** always run step 2 before step 3 — the viewer launches the engine binary that the build
> produces. If you skip it, the viewer will tell you to `cargo build --release` first.

### How to play

- In the left panel, under **You:**, click the color you want to play (**R**ed, **B**lue,
  **Y**ellow, or **G**reen). Hornet plays the other three seats.
- Click **New game**.
- On your turn, click one of your pieces, then click where it should go (legal destinations show as
  dots).
- **Pause / Step** freeze the engine so you can study a position; the right panel shows its candidate
  moves, the expected line, and the score breakdown (what it's thinking).
- Set search depth with **4 / 8 / 12 / 16** — higher is stronger but slower.

> **Linux note:** the viewer needs the usual desktop graphics libraries. If the window fails to open,
> install your distro's standard X11/Wayland + OpenGL dev packages (e.g. on Debian/Ubuntu:
> `libxcb1 libxkbcommon0 libgl1`).

---

## Develop & test

```sh
cargo test --release                 # the engine test suite
cargo run -p hornet-view --release   # the viewer (after a build)
```

The engine speaks a simple line protocol over stdin/stdout (`newgame`, `go depth N`, `move e1-f3`,
`board`, `status`, `eval`, …), so you can also drive it directly without the viewer.

## Layout

| Path | What |
|------|------|
| `hornet-engine/` | The engine crate (zero runtime dependencies — std only). |
| `hornet-view/` | The native viewer (egui). A pure display layer: it spawns the engine and relays moves, but never makes a game decision itself. |

## License

See [LICENSE](LICENSE).
