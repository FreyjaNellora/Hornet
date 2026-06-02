# Hornet

A four-player chess engine built from a single foundational primitive: **per-piece BFS line
projection** feeding a query engine that returns a per-player utility vector
**V = ⟨U₁, U₂, U₃, U₄⟩** to a **Max^n** search. The evaluation contract is a vector, never a scalar —
search backs up per-player components without ever collapsing them.

> **Status: early.** The specification is at **v0.2**. The engine currently implements the board
> types and the native **FEN4 / PGN4** I/O layer: the FEN4 parser round-trips the canonical starting
> position byte-identically, and the PGN4 parser structurally round-trips all 16 real-game corpus
> files. Move generation, line projection, the query engine, evaluation, search, and NNUE are not
> yet built.

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
cargo test     # 19 unit + 1 integration test (round-trips all 16 corpus games)
cargo run      # prints a skeleton banner (protocol not yet wired)
```

## License

See [LICENSE](LICENSE).
