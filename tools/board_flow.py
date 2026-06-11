"""Board-flow map (structural) + human-game overlay — a SIDE analysis tool, not engine code.

Premise (the user's framing): don't model the player's *mind* (incomplete, Gödel); read flow off the
*board* (complete, formal). The lanes are a property of the geometry — computable from the movement
rules with zero games. For each square we count how many empty-board *slider* moves CROSS it (its
"transit flow"), per piece type, purely from the 14×14 cross geometry. Then we map the human games
across it and measure whether real play flows along the structural lanes (Pearson r over squares).

Run: py tools/board_flow.py
"""
import os, re, glob, hashlib
import numpy as np
from scipy import stats

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DIRS = [os.path.join(ROOT, "baselines"), os.path.join(ROOT, "collected_games")]
MOVE = re.compile(r"([a-n])(\d{1,2})-([a-n])(\d{1,2})")


def playable(r, f):
    return 0 <= r < 14 and 0 <= f < 14 and not ((r < 3 or r > 10) and (f < 3 or f > 10))


def run(r, f, dr, df):
    """Contiguous playable squares from (r,f) in direction (dr,df), exclusive of the start."""
    n, rr, ff = 0, r + dr, f + df
    while playable(rr, ff):
        n += 1
        rr += dr
        ff += df
    return n


# --- structural flow: empty-board slider moves crossing each square ---
rook = np.zeros(196)
bishop = np.zeros(196)
for r in range(14):
    for f in range(14):
        if not playable(r, f):
            continue
        E, W, N, S = run(r, f, 0, 1), run(r, f, 0, -1), run(r, f, 1, 0), run(r, f, -1, 0)
        NE, SW, NW, SE = run(r, f, 1, 1), run(r, f, -1, -1), run(r, f, 1, -1), run(r, f, -1, 1)
        i = r * 14 + f
        rook[i] = E * W + N * S          # horizontal + vertical crossings
        bishop[i] = NE * SW + NW * SE    # the two diagonal crossings
queen = rook + bishop
# per-player slider mix (2 rooks, 2 bishops, 1 queen)
struct = 2 * rook + 2 * bishop + 1 * queen

# --- human-game transit (the ant-trails), deduped ---
def transit_squares(ff, fr, tf, tr):
    df, dr = (ord(tf) - ord(ff)), (tr - fr)
    if (df == 0) ^ (dr == 0) or abs(df) == abs(dr):
        steps = max(abs(df), abs(dr))
        if steps <= 1:
            return []
        sf, sr = (df // steps if df else 0), (dr // steps if dr else 0)
        return [(fr + sr * k - 1) * 14 + (ord(ff) - ord("a") + sf * k) for k in range(1, steps)]
    return []

seen = set()
human = np.zeros(196)
ngames = 0
for d in DIRS:
    for path in glob.glob(os.path.join(d, "*.pgn4")):
        moves = MOVE.findall(open(path, encoding="utf-8", errors="ignore").read())
        if not moves:
            continue
        sig = hashlib.md5(",".join("".join(m) for m in moves).encode()).hexdigest()
        if sig in seen:
            continue
        seen.add(sig)
        ngames += 1
        for ff, fr, tf, tr in moves:
            for s in transit_squares(ff, int(fr), tf, int(tr)):
                if 0 <= s < 196:
                    human[s] += 1

# --- correlate over playable squares ---
mask = np.array([playable(i // 14, i % 14) for i in range(196)])
def corr(a):
    return stats.pearsonr(a[mask], human[mask])[0]

print(f"{ngames} unique human games | board flow vs human transit, Pearson r over {mask.sum()} squares:\n")
print(f"  rook-flow   vs human transit : r = {corr(rook):+.3f}")
print(f"  bishop-flow vs human transit : r = {corr(bishop):+.3f}")
print(f"  queen-flow  vs human transit : r = {corr(queen):+.3f}")
print(f"  aggregate   vs human transit : r = {corr(struct):+.3f}   <- does real play flow along the geometry?")

def name(i):
    r, f = divmod(i, 14)
    return chr(ord("a") + f) + str(r + 1)

order = np.argsort(-struct)
print("\ntop 12 structural-flow squares (geometry) — and their human-transit rank:")
hrank = {idx: rk for rk, idx in enumerate(np.argsort(-human))}
for i in order[:12]:
    print(f"  {name(i):4} struct {int(struct[i]):5}  | human-transit rank #{hrank[i]+1}")

def heat(a, title):
    mx = a[mask].max() or 1
    ch = ".:-+*#@"
    print(f"\n{title} (rank 14 top):")
    for r in range(13, -1, -1):
        row = ""
        for f in range(14):
            i = r * 14 + f
            if not playable(r, f):
                row += "  "
                continue
            v = a[i] / mx
            row += (ch[min(len(ch) - 1, int(v * (len(ch) - 1) + 0.999))] if v > 0 else ".") + " "
        print(" " + row)

heat(struct, "STRUCTURAL flow (board geometry, slider mix)")
heat(human, "HUMAN transit (real games)")
