"""Proper eval-weight fitting (Texel's method, done right) with BOOTSTRAP CONFIDENCE INTERVALS.

Why this exists: the Rust `texel_tune` hill-climb gives a point estimate of the weights but cannot tell
you whether a weight is *really* non-zero or just noise. This fits the same sigmoid-MSE objective with a
real optimizer (scipy), then bootstraps to put a 95% CI on each weight. A weight whose CI excludes 0 is
a genuine signal; one whose CI straddles 0 is noise — that is the question we keep hitting.

Input: tools/texel_positions.csv  (generate it with:
    HORNET_DUMP_CSV=1 cargo run --release --example texel_tune   # writes per-(position,player) rows
)
Columns: dM,dP,dS,dO,target  — mean-relative components + placement outcome in [0,1].

Run:  py tools/fit_weights.py [n_bootstrap]   (default 200)

Math: eval_i = wM·dM + wP·dP + wS·dS − wO·dO ;  winprob = sigmoid(K·eval) ;  minimize mean((target−winprob)^2).
K (the sigmoid scale) is fit once on baseline weights then held fixed, so the weights are identifiable.
"""
import os, sys
import numpy as np
import pandas as pd
from scipy.optimize import minimize, minimize_scalar

HERE = os.path.dirname(os.path.abspath(__file__))
CSV = os.path.join(HERE, "texel_positions.csv")
SIGN = np.array([1.0, 1.0, 1.0, -1.0])  # crossfire (O) is subtracted in the eval
NAMES = ["W_MATERIAL", "W_POSITIONAL", "W_SAFETY", "W_CROSSFIRE"]


def winprob(w, K, X):
    return 1.0 / (1.0 + np.exp(-K * ((X * SIGN) @ w)))


def mse(w, K, X, y):
    return float(np.mean((y - winprob(w, K, X)) ** 2))


def fit_w(X, y, K, x0):
    res = minimize(mse, x0, args=(K, X, y), method="Nelder-Mead",
                   options={"xatol": 1e-3, "fatol": 1e-8, "maxiter": 20000})
    return res.x


def main():
    if not os.path.exists(CSV):
        sys.exit(f"missing {CSV} — run: HORNET_DUMP_CSV=1 cargo run --release --example texel_tune")
    df = pd.read_csv(CSV)
    X = df[["dM", "dP", "dS", "dO"]].to_numpy()
    y = df["target"].to_numpy()
    n = len(y)

    base_w = np.array([4.0, 1.0, 1.0, 1.0])
    K = minimize_scalar(lambda k: mse(base_w, k, X, y), bounds=(1e-5, 1e-2), method="bounded").x
    w = fit_w(X, y, K, base_w)
    print(f"rows: {n}   fitted K = {K:.5f}")
    print(f"baseline (4,1,1,1) MSE = {mse(base_w, K, X, y):.5f}")
    print(f"fitted weights: " + "  ".join(f"{nm}={v:.2f}" for nm, v in zip(NAMES, w)) +
          f"   MSE = {mse(w, K, X, y):.5f}")

    nboot = int(sys.argv[1]) if len(sys.argv) > 1 else 200
    rng = np.random.default_rng(0)
    boots = np.array([fit_w(X[i], y[i], K, w) for i in (rng.integers(0, n, n) for _ in range(nboot))])
    print(f"\nbootstrap x{nboot} - 95% CI per weight (CI excluding 0 => real signal):")
    for j, nm in enumerate(NAMES):
        lo, hi = np.percentile(boots[:, j], [2.5, 97.5])
        flag = "" if lo <= 0 <= hi else "   <-- significant"
        print(f"  {nm:13} {w[j]:7.3f}   [{lo:7.3f}, {hi:7.3f}]{flag}")
    print("\nnote: rows within a game aren't independent (block-bootstrap by game would be stricter),")
    print("so treat a borderline 'significant' as suggestive until the corpus is larger.")


if __name__ == "__main__":
    main()
