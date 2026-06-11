"""
collect_games.py — clipboard watcher for collecting chess.com 4-player (4PC) games.

Workflow (human-in-the-loop, no automated requests to chess.com):
  1. Run this script. It polls your clipboard.
  2. Browse chess.com 4PC and copy a game's PGN4 (button, Ctrl+C, or copy-on-select).
  3. Each new, valid, not-already-seen 4PC PGN is saved as its own .pgn4 file.

Because every game lands on the clipboard through your own manual action, this stays
within chess.com's rules: no scripted page requests, no bot traffic, human-paced. It
only captures data you can already see and copy yourself.

Copy-on-select on Windows: the OS has no native copy-on-select. Install a browser
extension (e.g. "Auto Copy" / "AutocopySelectionText" for Chrome) so merely selecting
the PGN text copies it. This script captures it the same way regardless.

Stop with Ctrl+C.
"""

import hashlib
import re
import sys
import time
from pathlib import Path

import pyperclip

# --- config -----------------------------------------------------------------
OUT_DIR = Path(__file__).parent / "collected_games"
PREFIX = "cc_game_"
EXT = ".pgn4"
POLL_SECONDS = 0.4
SEEN_FILE = OUT_DIR / ".seen_hashes"  # persisted dedupe set, survives restarts

# Save ONLY real chess.com FFA games: require [Variant "FFA"] AND a [GameNr]. This excludes other
# variants (Chaturaji, Teams, etc.) and metadata-less / self-play captures that would contaminate the
# corpus — the corpus stays clean at capture time, not just at fit time.
_FFA_RE = re.compile(r'\[\s*Variant\s+"FFA"', re.IGNORECASE)
_GAMENR_RE = re.compile(r'\[\s*GameNr\s+"', re.IGNORECASE)


def looks_like_ffa(text: str) -> bool:
    if not text or len(text) < 20:
        return False
    return bool(_FFA_RE.search(text)) and bool(_GAMENR_RE.search(text))


def normalize(text: str) -> str:
    """Whitespace-insensitive key so trivial reformatting doesn't dupe."""
    return re.sub(r'\s+', ' ', text.strip())


def fingerprint(text: str) -> str:
    return hashlib.sha1(normalize(text).encode("utf-8")).hexdigest()


def load_seen() -> set:
    if SEEN_FILE.exists():
        return set(SEEN_FILE.read_text(encoding="utf-8").split())
    # Backfill from any .pgn4 already in OUT_DIR so a fresh .seen_hashes doesn't re-save them.
    seen = set()
    for f in OUT_DIR.glob(f"*{EXT}"):
        seen.add(fingerprint(f.read_text(encoding="utf-8")))
    return seen


def next_index() -> int:
    existing = sorted(OUT_DIR.glob(f"{PREFIX}*{EXT}"))
    n = -1
    for f in existing:
        m = re.search(r'(\d+)', f.stem)
        if m:
            n = max(n, int(m.group(1)))
    return n + 1


def main():
    OUT_DIR.mkdir(exist_ok=True)
    seen = load_seen()
    idx = next_index()
    saved = 0
    last_raw = None

    print(f"Watching clipboard -> {OUT_DIR}")
    print(f"Already have {len(seen)} game(s). Copy a chess.com FFA game's PGN to capture it (FFA only — Chaturaji/Teams are skipped). Ctrl+C to stop.\n")

    try:
        while True:
            try:
                raw = pyperclip.paste()
            except Exception as e:  # transient clipboard lock (common on Windows)
                time.sleep(POLL_SECONDS)
                continue

            if raw != last_raw:
                last_raw = raw
                if looks_like_ffa(raw):
                    fp = fingerprint(raw)
                    if fp in seen:
                        print("  - already collected (skipped)")
                    else:
                        path = OUT_DIR / f"{PREFIX}{idx:04d}{EXT}"
                        path.write_text(raw.strip() + "\n", encoding="utf-8")
                        with SEEN_FILE.open("a", encoding="utf-8") as sf:
                            sf.write(fp + "\n")
                        seen.add(fp)
                        idx += 1
                        saved += 1
                        print(f"  [+] saved {path.name}  (this session: {saved})")
            time.sleep(POLL_SECONDS)
    except KeyboardInterrupt:
        print(f"\nStopped. Saved {saved} new game(s) this session; {len(seen)} total.")


if __name__ == "__main__":
    main()
