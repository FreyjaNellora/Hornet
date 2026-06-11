# Where the engine is now — and an independent ask (for Kimi)

Since we last synced, the search/eval picture moved a lot. Here's the caught-up state (findings only),
then a request: **research and plan your own next move independently** — we have ours, and we want a
genuinely independent read to compare against, so please don't anchor to us.

## What we found
1. **The eval was the bottleneck, and we found *why*.** Deeper search wasn't winning. Root cause: the
   eval optimizes the wrong objective — it's points-blind centipawns (material + the converging-enemy
   term), but 4PC is won on FFA points / eliminations / placement, and the game isn't zero-sum. More
   search just optimized the wrong thing harder. Depth was a probe telling us the objective is wrong.

2. **A proper weight-fit (scipy logistic fit + bootstrap confidence intervals, on the human corpus)
   said, with real numbers:**
   - material — real signal;
   - converging-enemy (crossfire) — real signal;
   - per-square positional — **noise** (its CI spans zero);
   - **king-safety — significantly *negative* (actively harmful).** Reading the term: it rewards
     *huddling* pieces around the king, which in 4PC is passive, undeveloped play that correlates with
     losing — so the tuner had to negate the whole term.

3. **Depth can pay — but only with the right objective and enough breadth.** Once the objective is
   handled correctly, deeper search starts beating shallower, *provided the beam widens as depth grows*
   (a fixed narrow beam re-creates the "deeper is worse" pathology by pruning the best line).

## What we're deliberately NOT telling you
We've already built our own fixes (an objective-handling change + a safety rebuild) and an adaptive-beam
idea. **We're withholding the details on purpose** so your plan is independent. We'll put the two side by
side.

## The ask (your lane: the eval)
1. **Do your own research** into how strong engines actually build, weight, and tune eval terms —
   especially **relational/structural** terms (pawn structure, outposts, open files, king-safety done
   *right*) and how they avoid the "rewards passivity" trap our safety term fell into.
2. **Write up your own plan** for the eval's next phase: which terms to add, how to represent them, how
   to tune them, and how to gate whether they actually help — given that (a) the per-square positional
   approach is dead, (b) the objective is now handled elsewhere, and (c) the human corpus is still small.
3. **Optional but useful:** your own **math breakdown** of the eval as it stands — reduce it to the
   math and surface the constraints/risks you see.

Bring it back as *your* plan. We compare, then merge the best of both.
