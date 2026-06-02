# Hornet

A four-player chess engine built from a single foundational primitive: **per-piece BFS line
projection** feeding a query engine that returns a per-player utility vector
**V = ⟨U₁, U₂, U₃, U₄⟩** to a **Max^n** search. The evaluation contract is a vector, never a scalar —
search backs up per-player components without ever collapsing them.

> **Status: early.** The specification is at **v0.2**. Implemented so far: the board types and native
> **FEN4 / PGN4** I/O (FEN4 round-trips the start position byte-identically; PGN4 structurally
> round-trips all 16 corpus games), **legal move generation** (perft `20 / 395 / 7800 / 152050`,
> matching the reference engine), **per-piece line projection**, the **query engine** + **evaluation**
> (the `V` vector), and a **Max^n search** (transposition table + beam + MVV-LVA ordering). The full
> position → move pipeline runs end-to-end. NNUE and the UCI-style protocol are the remaining pieces.

## Layout

| Path | What |
|------|------|
| `PITCH-for-new-agents.md` | **Start here** — what Hornet is, where things live, the hard rules. |
| `HORNET-BUILD-SPEC.md` | The build specification (v0.2) — source of truth for what to build. |
| `TECHNIQUES-and-REFERENCES.md`, `SOURCES-and-CITATIONS.md` | Academic techniques and citations. |
| `hornet-engine/` | The Rust engine crate. |
| `baselines/` | 16 real chess.com 4PC games (PGN4) + a 25-position tactical fixture suite. |
| `Playbook/` | The project-agnostic operations framework Hornet runs under. |

## Build & test

```sh
cd hornet-engine
cargo test     # 59 unit + 3 integration tests
cargo run      # prints a skeleton banner (protocol not yet wired)
```

## License

See [LICENSE](LICENSE).
