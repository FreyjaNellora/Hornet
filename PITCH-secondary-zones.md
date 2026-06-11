# Pitch — Secondary-zone board control

**Status:** the geometry is established. The *usage structure* of these zones is now measured on
the 16-game human corpus (see "What the corpus showed"). What's still unproven — and what the
ablation must decide — is whether *encoding* zone control actually improves engine strength. Don't
ship it default-on until the ablation shows a gain.

## Hypothesis

Control of nine specific 2×2 zones gives a player the most control over the board. The corpus
study below confirms the zones are real, structured, and used differently by role — but "used a
lot" is not "improves the evaluator." The open question is strength, settled by ablation.

## Board (established)

14×14 with four 3×3 corners dead → a playable **cross of 160 squares** with four 8×3 arms.
`is_valid` at [types.rs:52](hornet-engine/src/board/types.rs#L52). Geometry runs on the playable
cross; the dead corners are counted only when they constrain — slider-ray termination at the core
boundary, arm depth (3), and graph adjacency for flow.

## The nine secondary zones (established geometry)

2×2 blocks. "Cardinal" / "diagonal" are positional descriptors only — no attached meaning.

| Zone | center (rank,file idx) | squares |
|---|---|---|
| Center | (6.5, 6.5) | g7 h7 g8 h8 |
| Gate W | (6.5, 2.5) | c7 d7 c8 d8 |
| Gate E | (6.5, 10.5) | k7 l7 k8 l8 |
| Gate S | (2.5, 6.5) | g3 h3 g4 h4 |
| Gate N | (10.5, 6.5) | g11 h11 g12 h12 |
| Quadrant SW | (4.5, 4.5) | e5 f5 e6 f6 |
| Quadrant SE | (4.5, 8.5) | i5 j5 i6 j6 |
| Quadrant NW | (8.5, 4.5) | e9 f9 e10 f10 |
| Quadrant NE | (8.5, 8.5) | i9 j9 i10 j10 |

Indices 0-based: file 0=a…13=n; rank idx r = rank r+1.

## Coverage property (established)

Orthogonal-lane occupancy (the ranks/files each group sits on):

- Center + gates: ranks/files {2,3,6,7,10,11}.
- Quadrant zones: ranks/files {4,5,8,9}.
- Union → ranks 2–11 and files 2–11, i.e. every playable square lies on the rank- or file-lane of
  some zone (only the dead corners, rank and file both in {0,1,12,13}, are off all lanes).

Both groups are required. The gates alone leave the {4,5,8,9} ranks/files uncovered, and those gaps
fall in the wings; the quadrant zones' orthogonal lanes cover those wing cells. Each 8-long arm
(ranks/files 3–10) is split: gates cover {3,6,7,10}, quadrant zones cover {4,5,8,9}.

## What the corpus showed (measured, descriptive)

Replayed 2532 positions across the 16 games
([examples/zone_stats.rs](hornet-engine/examples/zone_stats.rs)). The three families behave
differently — this is descriptive usage, not a strength result:

- **Gates = held anchors.** Most-occupied (each ~20–29% of plies vs center/quads ~5–13%) and best
  defended (~2 friendly defenders on an occupied gate square).
- **Center = contested transit.** Lowest occupancy (7.7%) but the most moves landing in it and the
  most captures (32 = all four quads combined), fewest defenders (0.82). High churn, low tenure.
- **Quadrants = gate-fed.** Lower occupancy; the dominant inter-zone reach is gate → adjacent quad
  (gates supply the quads; quads project little back).
- **Per-seat asymmetry:** Green favors center, Red favors quads+gates, Yellow is low everywhere —
  confounded by elimination timing, and the corpus is not seat-balanced.

Caveats: coverage is the replayed opening/midgame prefix; gate "control" is partly geometric (the
cross-axis carries rook/queen lines regardless of intent); 16-game sample. These say the zones are
worth *encoding and testing* — they do not say encoding will help.

## How "control" is measured in-engine

The substrate already exists: line projection ([lines.rs:174](hornet-engine/src/lines.rs#L174))
plus the per-square inverse index ([lines.rs:105–162](hornet-engine/src/lines.rs#L105-L162),
"which pieces reach this square"). Zone control = friendly vs. enemy reachers summed over a zone's
squares. Computable now; nothing new in the substrate.

## Ablation (how to decide if it helps)

Two engine configs, identical except for the zone-control term: **OFF (default) vs ON**. Encode the
term using the corpus priors (weight gates as the anchors), then measure the strength delta
head-to-head with **seat rotation / counterbalancing** so per-seat priors don't bias the result
(the spec's seat-fair design). Keep the term only if ON beats OFF; drop it if neutral or worse.
Pre-search, the tactical-fixture solve rate
([baselines/tactical_samples.json](baselines/tactical_samples.json)) is the cheap proxy. Do not
ship an unvalidated weighting.

## If it validates — where it lands

- Positional Pᵢ is currently flat centrality-weighted empty-square control
  ([queries.rs:84–94](hornet-engine/src/queries.rs#L84-L94)). Zone control is a discrete,
  structured alternative or refinement to that flat weighting — compare the two, don't assume.
- Hard Rule #4: no 5th V component. It folds into Pᵢ or becomes an NNUE input feature.
- Default-off + ablation arm.
- This is positional (`eval_value` side); keep distinct from `ffa_points` bounty scoring.

## Verify

```
cd hornet-engine
cargo test                  # 66 green today
```
