# Hornet Strength Gate (Hard Rule #7)

**Status:** Defined 2026-06-06 · **Owner:** Kimi · **Gatekeeper:** Texel MSE + blunder-rate

The exact-move match rate against human fixtures is dead as a metric (0–2/13 noise — a different good move scores as a miss). The strength gate is outcome-based, matching how classical engines are tuned.

---

## Gate criteria (ALL must pass)

### 1. Texel MSE < 0.10

The eval must predict corpus game outcomes better than the current v0 baseline.

- **Current v0 baseline:** MSE = 0.11453 (16-game corpus, 855 positions, weights [4,1,1,1])
- **Chance level:** MSE = 0.14 (uniform random placement)
- **Gate:** MSE < 0.10
- **Measurement:** `cargo run --release --example texel_tune`

Rationale: 0.11453 → 0.10 is a ~13% relative improvement. This requires real eval feature gains (pawn structure, mobility refinement, etc.), not just weight tuning. The current weights are already optimal; further MSE drops come from feature quality.

### 2. Blunder rate < 5%

The engine must rarely capture into a losing exchange.

- **Measurement:** `cargo run --release --example gate_ablation` — the "ENGINE-LOSES-MATERIAL" count
- **Current baseline:** ~1% capture-into-loss (avg 12 cp newly-hung over 150 corpus positions)
- **Gate:** < 5% blunder rate

Rationale: A blunder rate > 5% means the eval systematically misevaluates exchanges, which is fatal in FFA where three opponents exploit errors. The current ~1% is already gate-passing; this criterion ensures regressions are caught.

### 3. Quiet-move stability < 200 cp avg swing

A single quiet move should not swing the eval by thousands.

- **Measurement:** `cargo run --release --example gate_ablation` — CALIBRATION line, quiet-move avg
- **Current baseline:** ~hundreds (post-recalibration; was ~1300 before)
- **Gate:** quiet-move avg swing < 200 cp

Rationale: This catches scale bugs. The pre-recalibration eval swung by 1294 avg / 3506 max due to crossfire `value×count`. Post-recalibration it's ~hundreds. The 200 cp bound gives slack while preventing scale regressions.

---

## Measurement procedure

```bash
cd hornet-engine

# 1. Texel MSE
cargo run --release --example texel_tune
# Look for: "baseline weights [4.0, 1.0, 1.0, 1.0] MSE=..."
# Gate: MSE < 0.10

# 2. Blunder rate + quiet-move stability
cargo run --release --example gate_ablation
# Look for: "ENGINE-LOSES-MATERIAL" count and CALIBRATION line
# Gate: blunders < 5% of tested, quiet avg < 200
```

---

## Gate passing → NNUE (P7)

Once all three criteria pass, NNUE training begins. The NNUE replaces the hand-tuned v0 eval, using the same 4-component decomposition (M/P/S/O) as input features. The gate ensures the feature set is rich enough to train from.

---

## History

- **2026-06-06:** Gate defined. v0 baseline: MSE 0.11453, blunder ~1%, quiet swing ~hundreds.
- **Pre-recalibration:** MSE untuned, blunder untracked, quiet swing 1294 avg / 3506 max.
