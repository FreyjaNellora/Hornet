"""Opening patterns in the human games: do strong 4PC players CONVERGE on openings (= opening theory),
and what are the common lines per seat? The most convergent/robust pattern in the corpus — the stable
counterpart to the flipping midgame eval weights. A high top-move share = real theory = bookable.

Reads human_games/. Run: py tools/opening_patterns.py
"""
import os, re, glob
from collections import Counter

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
HG = os.path.join(ROOT, "human_games")
MOVE = re.compile(r"[a-n]\d{1,2}-[a-n]\d{1,2}")
ROUND = re.compile(r"^\s*\d+\.\s")
SEATS = ["Red", "Blue", "Yellow", "Green"]

games = []
for f in glob.glob(os.path.join(HG, "*.pgn4")):
    txt = open(f, encoding="utf-8", errors="ignore").read()
    seat_moves = [[], [], [], []]
    for line in txt.splitlines():
        if not ROUND.match(line):
            continue
        segs = line.split("..")
        for si in range(min(4, len(segs))):
            m = MOVE.search(segs[si])
            if m:
                seat_moves[si].append(m.group(0))
    if all(len(sm) >= 3 for sm in seat_moves):
        games.append(seat_moves)

n = len(games)
print(f"{n} games with full 3-move openings per seat\n")

conv = []
for si, seat in enumerate(SEATS):
    first = Counter(g[si][0] for g in games)
    second = Counter(g[si][1] for g in games)
    seq2 = Counter(tuple(g[si][:2]) for g in games)
    seq3 = Counter(tuple(g[si][:3]) for g in games)
    top1, top1n = first.most_common(1)[0]
    conv.append(top1n / n)
    print(f"=== {seat} ===  1st-move convergence: '{top1}' in {100*top1n/n:.0f}% of games "
          f"| {len(first)} distinct 1st moves, {len(seq3)} distinct 3-move lines")
    print("  1st: " + ", ".join(f"{m} {100*c/n:.0f}%" for m, c in first.most_common(4)))
    print("  2nd: " + ", ".join(f"{m} {100*c/n:.0f}%" for m, c in second.most_common(4)))
    print("  top opening lines:")
    for seq, c in seq3.most_common(3):
        print(f"     {' '.join(seq)}  ({100*c/n:.0f}%)")
    print()

avg = sum(conv) / 4
print(f"Average 1st-move convergence across seats: {100*avg:.0f}%")
print("  >~50% = real opening theory (bookable);  ~25% (1 of ~4) = weak/diverse;  spread = no theory.")
