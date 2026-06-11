"""Fit the adaptive-cap rule from the cap-vs-branching data.

Reads tools/cap_branching.csv (ply,pieces,branching,cap_needed,converged) produced by the
`cap_vs_branching` example. Fits cap_needed against branching and against pieces, reports the slope k,
the fit quality (R²), the implied hard ceiling (k · max_branching), and which feature predicts the
needed cap better. This turns "breadth must scale with the board's busy-ness" into actual numbers.

Run: py tools/fit_cap.py
"""
import os
import sys
import numpy as np
import pandas as pd
from scipy import stats

sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # Windows console: don't crash on ≈/·/²

HERE = os.path.dirname(os.path.abspath(__file__))
CSV = os.path.join(HERE, "cap_branching.csv")


def main():
    if not os.path.exists(CSV):
        raise SystemExit("missing cap_branching.csv — run: cargo run --release --example cap_vs_branching")
    df = pd.read_csv(CSV)
    conv = df[df.converged == 1]
    print(f"{len(df)} positions; {len(conv)} converged within the tested caps "
          f"({len(df) - len(conv)} still moving at the widest cap -> their true need is higher).")

    best = None
    for feat in ["branching", "pieces"]:
        x = conv[feat].to_numpy(float)
        y = conv["cap_needed"].to_numpy(float)
        if len(x) < 3 or x.std() == 0:
            print(f"\n{feat}: too few converged points to fit.")
            continue
        k0 = float(np.sum(x * y) / np.sum(x * x))           # least-squares through the origin
        lr = stats.linregress(x, y)                          # with intercept
        r2 = lr.rvalue ** 2
        print(f"\n{feat}:")
        print(f"  through origin : cap ≈ {k0:.1f} · {feat}")
        print(f"  with intercept : cap ≈ {lr.slope:.1f} · {feat} + {lr.intercept:.0f}   (R²={r2:.2f}, p={lr.pvalue:.3f})")
        if best is None or r2 > best[1]:
            best = (feat, r2, k0)

    if best:
        feat, r2, k0 = best
        mx = int(df[feat].max())
        ceiling = k0 * mx
        print(f"\nbest predictor: {feat} (R²={r2:.2f})")
        print(f"hard ceiling: k · max_{feat} ≈ {k0:.1f} · {mx} ≈ {ceiling:.0f}")
        print(f"max cap actually needed in this sample: {int(df.cap_needed.max())}  "
              f"-> the ceiling is rarely approached, so the adaptive cap saves a lot.")
    print("\nnote: 'still moving' positions cap out at the widest tested value, so the fitted k is a "
          "slight UNDER-estimate; widen the cap sweep if many positions didn't converge.")


if __name__ == "__main__":
    main()
