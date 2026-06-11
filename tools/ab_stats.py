"""Significance for the self-play A/B gate. The runs use few games, so eyeballing a win-rate is
misleading — this gives the 95% CI and the p-value vs 50%, and how many games you'd need.

Run:  py tools/ab_stats.py WINS GAMES [A_POINTS B_POINTS]
e.g.  py tools/ab_stats.py 2 6 214 282
"""
import sys
from scipy import stats


def main():
    if len(sys.argv) < 3:
        sys.exit("usage: py tools/ab_stats.py WINS GAMES [A_POINTS B_POINTS]")
    wins, games = int(sys.argv[1]), int(sys.argv[2])
    p = wins / games
    # Jeffreys interval (good for small n); exact binomial test against 50%.
    lo, hi = stats.beta.ppf([0.025, 0.975], wins + 0.5, games - wins + 0.5)
    pval = stats.binomtest(wins, games, 0.5).pvalue
    print(f"A win-rate: {wins}/{games} = {100*p:.0f}%   95% CI [{100*lo:.0f}%, {100*hi:.0f}%]   p(vs 50%) = {pval:.3f}")
    verdict = "CONCLUSIVE" if pval < 0.05 else "not significant yet"
    print(f"  -> {verdict} at this sample size.")

    if len(sys.argv) >= 5:
        a, b = float(sys.argv[3]), float(sys.argv[4])
        leader = "A" if a > b else "B"
        rel = 100 * abs(a - b) / max(1.0, (a + b) / 2)
        print(f"points: A {a:.0f} vs B {b:.0f}  ({leader} leads by {abs(a-b):.0f}, {rel:.0f}% relative)")

    # crude power: games needed to detect the observed effect at 80% power (normal approx)
    if 0 < p < 1 and p != 0.5:
        import math
        eff = abs(p - 0.5)
        need = math.ceil((1.96 * 0.5 + 0.84 * math.sqrt(p * (1 - p))) ** 2 / eff ** 2)
        print(f"  ~{need} games would be needed to call this effect at 80% power.")
    print("rule of thumb: <~20 games is directional; ~50-100 to call a small edge.")


if __name__ == "__main__":
    main()
