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

**No coding required.** You'll install one free tool (Rust), download Hornet, then build and run it —
about 10–15 minutes, most of it just waiting on the build to finish. The steps below are written for
**Windows**; **macOS / Linux** notes are called out where they differ. If you've never done any of
this, that's fine — follow it line by line.

**Step 1 — Open a terminal** (a terminal is just a window where you type commands)

- **Windows:** click Start, type `PowerShell`, and open it.
- **macOS:** press ⌘ + Space, type `Terminal`, press Enter.
- **Linux:** open your Terminal app.

**Step 2 — Install Rust** (the free toolkit that builds Hornet)

Open **<https://rustup.rs>** in your web browser, then:

- **Windows:** download `rustup-init.exe` and double-click it. A black window opens — just press
  **Enter** to accept the default option (`1`) and wait for it to finish. If it says it needs the
  **Visual C++ Build Tools** ("Desktop development with C++"), say **yes** and let it install — that's
  the piece that turns code into a program your computer can actually run.
- **macOS / Linux:** copy the one-line command the page shows you, paste it into your terminal, press
  Enter, and accept the defaults.

When it's done, **close the terminal window and open a brand-new one** (it only "sees" Rust in a fresh
window). To check it worked, type this and press Enter:

```sh
cargo --version
```

You should see a version number (something like `cargo 1.9x.x`). If you instead see "command not
found", close the terminal, open a fresh one, and try again.

**Step 3 — Download Hornet** (the easy way — no extra tools needed)

1. Go to **<https://github.com/FreyjaNellora/Hornet>**.
2. Click the green **`Code`** button, then **Download ZIP**.
3. Unzip the file (**Windows:** right-click it → **Extract All**). Note where it lands — for example
   `Downloads\Hornet-main`.

*(Already have `git`? You can instead run `git clone https://github.com/FreyjaNellora/Hornet.git`.)*

**Step 4 — Point your terminal at that folder**

In the terminal, type `cd ` (the letters `c`, `d`, then a **space**), then **drag the unzipped Hornet
folder onto the terminal window** — it fills in the path for you. Press Enter. (Or just type the path
yourself, e.g. `cd Downloads\Hornet-main`.)

**Step 5 — Build it** — this turns the code into a program. The first build takes a few minutes;
that's normal:

```sh
cargo build --release
```

Lots of lines scroll past. When you see **`Finished`** near the end, it worked.

**Step 6 — Play:**

```sh
cargo run -p hornet-view --release
```

A window opens with the board and the engine starts on its own. That's it — pick your color, click
**New game**, and play (see **How to play** below).

### If something goes wrong

- **`cargo: command not found`** — close the terminal and open a new one (Rust only appears in
  terminals you open *after* installing it). Still nothing? Re-run the installer from
  <https://rustup.rs>.
- **A build error mentioning a "linker" or `link.exe`** (Windows) — the Visual C++ Build Tools were
  skipped during Rust's install. Install the **"Desktop development with C++"** workload (through the
  Visual Studio Installer), open a new terminal, and run `cargo build --release` again.
- **A build error saying your Rust is too old, or something about an "edition"** — run `rustup update`,
  then build again (Hornet needs Rust **1.85 or newer**).
- **Linux: the window won't open** — install your distro's desktop graphics libraries (on
  Debian/Ubuntu: `libxcb1 libxkbcommon0 libgl1`).

### How to play

- In the left panel, under **You:**, click the color you want to play (**R**ed, **B**lue,
  **Y**ellow, or **G**reen). Hornet plays the other three seats.
- Click **New game**.
- On your turn, click one of your pieces, then click where it should go (legal destinations show as
  dots).
- **Pause / Step** freeze the engine so you can study a position; the right panel shows its candidate
  moves, the expected line, and the score breakdown (what it's thinking).
- Set search depth with **4 / 8 / 12 / 16** — higher is stronger but slower.

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
