"""Ant-trails: aggregate real piece movement across all human games and HONESTLY test whether the zone
families (Center / Gates / Quadrants) actually carry the board's traffic — no confirmation bias.

For each move we count (a) the DESTINATION square (where a piece lands) and (b) the TRANSIT squares it
crosses (for sliding moves — the actual "lane"). Then we compare each family's share of traffic to its
share of the board (a "lift"): lift > 1 = a real hub/lane, lift ≈ 1 = nothing special, lift < 1 = cold.

Reads every unique .pgn4 in baselines/ + collected_games/ (deduped by move-content).
Run: py tools/zone_trails.py
"""
import os, re, glob, hashlib
from collections import Counter

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DIRS = [os.path.join(ROOT, "baselines"), os.path.join(ROOT, "collected_games")]
MOVE = re.compile(r"([a-n])(\d{1,2})-([a-n])(\d{1,2})")

FAMILIES = {
    "Center":    ["g7", "h7", "g8", "h8"],
    "Gates":     ["c7", "d7", "c8", "d8", "k7", "l7", "k8", "l8",
                  "g3", "h3", "g4", "h4", "g11", "h11", "g12", "h12"],
    "Quadrants": ["e5", "f5", "e6", "f6", "i5", "j5", "i6", "j6",
                  "e9", "f9", "e10", "f10", "i9", "j9", "i10", "j10"],
}

def to_idx(fc, rn):
    return (rn - 1) * 14 + (ord(fc) - ord("a"))

def coord_idx(s):
    m = re.match(r"([a-n])(\d{1,2})", s)
    return to_idx(m.group(1), int(m.group(2)))

FAM_IDX = {fam: {coord_idx(s) for s in sqs} for fam, sqs in FAMILIES.items()}
ALL_FAM = set().union(*FAM_IDX.values())

def playable(i):
    r, f = divmod(i, 14)
    return not ((r < 3 or r > 10) and (f < 3 or f > 10))  # 3x3 corners off-board

def name(i):
    r, f = divmod(i, 14)
    return chr(ord("a") + f) + str(r + 1)

def transit_squares(ff, fr, tf, tr):
    """Intermediate squares a sliding move crosses (excludes from & to). Empty for knight/single steps."""
    df, dr = (ord(tf) - ord(ff)), (tr - fr)
    if (df == 0) ^ (dr == 0) or abs(df) == abs(dr):  # straight or pure diagonal
        steps = max(abs(df), abs(dr))
        if steps <= 1:
            return []
        sf, sr = (df // steps if df else 0), (dr // steps if dr else 0)
        return [to_idx(chr(ord(ff) + sf * k), fr + sr * k) for k in range(1, steps)]
    return []  # knight (or non-line) — no lane

# --- dedup games by move-content across both dirs ---
seen, games = set(), []
for d in DIRS:
    for f in glob.glob(os.path.join(d, "*.pgn4")):
        moves = MOVE.findall(open(f, encoding="utf-8", errors="ignore").read())
        if not moves:
            continue
        sig = hashlib.md5(",".join("".join(m) for m in moves).encode()).hexdigest()
        if sig not in seen:
            seen.add(sig)
            games.append(moves)

land, transit = Counter(), Counter()
for moves in games:
    for ff, fr, tf, tr in moves:
        fr, tr = int(fr), int(tr)
        land[to_idx(tf, tr)] += 1
        for s in transit_squares(ff, fr, tf, tr):
            transit[s] += 1

n_play = sum(1 for i in range(196) if playable(i))
print(f"{len(games)} unique games | {sum(land.values())} moves | {sum(transit.values())} transit-crossings\n")

for label, counts in [("LANDING (destinations)", land), ("TRANSIT (lanes crossed)", transit)]:
    total = sum(counts.values())
    print(f"== {label} ==  (lift = traffic-share / board-area-share; >1 = real hub/lane)")
    for fam, idxs in FAM_IDX.items():
        traf = sum(counts[i] for i in idxs)
        area = len(idxs) / n_play
        lift = (traf / total) / area if total else 0
        print(f"  {fam:10}: {100*traf/total:5.1f}% of traffic  | {len(idxs)} sq ({100*area:4.1f}% of board)  -> {lift:.2f}x")
    af = sum(counts[i] for i in ALL_FAM)
    print(f"  {'ALL 9':10}: {100*af/total:5.1f}% of traffic  | {len(ALL_FAM)} sq ({100*len(ALL_FAM)/n_play:4.1f}% of board)  -> {(af/total)/(len(ALL_FAM)/n_play):.2f}x")
    print("  top 12 squares (in-family *):")
    for i, c in counts.most_common(12):
        fam = next((fa for fa, s in FAM_IDX.items() if i in s), "")
        print(f"     {name(i):4} {c:5}  {'* ' + fam if fam else ''}")
    print()

# transit heatmap (the lanes), rank 14 at top, F-marks family squares
mx = max(transit.values()) if transit else 1
chars = ".:-+*#@"
print("TRANSIT heatmap (lanes; intensity .=low @=peak; F = family square):")
for r in range(13, -1, -1):
    row = ""
    for f in range(14):
        i = r * 14 + f
        if not playable(i):
            row += "  "
            continue
        v = transit[i] / mx
        ch = chars[min(len(chars) - 1, int(v * (len(chars) - 1) + 0.999))] if v > 0 else "."
        row += ch + ("F" if i in ALL_FAM else " ")
    print(" " + row)
