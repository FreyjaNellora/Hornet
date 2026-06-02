# Session Protocol

How to run a session (shift). A session is one agent's continuous work period on one phase.

## Origins

This protocol synthesizes patterns from:
- **SBAR** (Situation, Background, Assessment, Recommendation) -- medical handoff structure
- **I-PASS** (Illness severity, Patient summary, Action list, Situation awareness, Synthesis) -- structured medical handoff
- **Toyota hikitsugu** -- shift handoff with overlap period, verbal + written transfer
- **Military SITREP** -- structured situation reports at defined intervals
- **Nuclear shift turnover** -- checklist-based handoff with independent verification

The common thread: handoffs are the highest-risk moment. Every framework above exists because people died when handoffs failed. Our stakes are lower, but the principle is the same -- information loss at handoff is the primary failure mode.

## Session Start Protocol

### 1. Read the Production Board (STATUS.md)

Understand the overall project state before diving into your phase. Know what's blocked, what's active, what recently changed.

**Time:** 1-2 minutes.

### 2. Read Your Phase File

Your phase file is your station manual. It tells you:
- Commander's intent (what this phase exists to accomplish)
- Current state (what's done, what's pending, what's blocked)
- Active watch items (things to keep an eye on)
- Acceptance checklist (how you know you're done)
- Write scope (what you're allowed to modify)

**Time:** 2-5 minutes.

### 3. Read the Latest Session Note for Your Phase

The last agent's detailed handoff. This tells you:
- What they were working on
- What they tried (including what failed)
- What they recommend doing next
- Any hazards or gotchas they discovered

**Time:** 3-10 minutes.

### 4. Verify Build / Environment Is Clean

Before making any changes:
- Check version control status (any uncommitted work?)
- Verify the build compiles / tests pass
- Confirm you're on the correct branch
- Check for any outstanding change orders that affect your phase

**Time:** 2-5 minutes.

### 5. Write a Start Entry

Log to dispatch_comms.jsonl:
```json
{
  "type": "status",
  "source": "agent",
  "phase": "N",
  "message": "Session start. Read phase file and session note [X]. Plan: [brief plan]. First action: [what I'm doing first].",
  "resolved": false
}
```

## During Session

### Decision Point Logging

Log to dispatch_comms.jsonl at meaningful decision points -- not every line of code, but every fork in the road.

**Good log entry:**
```json
{
  "type": "progress",
  "source": "agent",
  "phase": "3",
  "message": "Found that the parser silently drops malformed input instead of erroring. This explains the missing data bug from session 11. Fix: add validation at parse entry point. This is within Phase 3 write scope. Implementing now.",
  "resolved": false
}
```

**Bad log entry:**
```json
{
  "type": "progress",
  "source": "agent",
  "phase": "3",
  "message": "Working on the parser.",
  "resolved": false
}
```

### Opinionated Entries

Every log entry should answer three questions:
1. **What happened?** (Observation)
2. **What does it mean?** (Assessment)
3. **What should be done?** (Recommendation)

This maps directly to the SBAR pattern from medical handoffs. "Situation: X. Background: Y. Assessment: Z. Recommendation: W."

Don't just report facts -- attach your judgment. The user can override, but they shouldn't have to ask "so what?" after reading your update.

### State-Change Focus

Capture **decisions**, not discussions. Capture **outcomes**, not process. Capture **what changed**, not what you read.

If you spent 30 minutes reading code to understand a module, the log entry isn't "read module X for 30 minutes." It's "Module X uses pattern Y, which means Z for our implementation. Decision: approach A instead of B because [reason]."

### Check-In Timing

Adaptive based on current state:

| Situation | Check Frequency |
|-----------|----------------|
| Waiting on approval | Every 2 minutes |
| Just received a response | Every 5 minutes |
| Normal work, no pending requests | Every 10 minutes |
| Deep in long task (build, test suite) | Every 15 minutes |

If you need more time without checking in, write an extension entry:
```json
{
  "type": "extension",
  "source": "agent",
  "message": "EXTENSION: Running full test suite, estimated 20 min. Will check back when complete.",
  "resolved": false
}
```

### When You're Stuck

The debugging anti-spiral rule: **each analysis pass must cite something NEW.** If you're re-reading the same code, re-running the same test, or restating the same hypothesis, you're spiraling.

When stuck:
1. Write down what you know, what you've tried, and what you've ruled out.
2. Identify what specific NEW information would unblock you.
3. Go get that information (read a different file, run a different test, check a different assumption).
4. If you can't identify new information to gather, escalate.

```json
{
  "type": "stuck",
  "source": "agent",
  "phase": "3",
  "message": "STUCK: [what I'm trying to do]. Tried: [approaches]. Ruled out: [hypotheses]. Need: [what would unblock me]. Recommendation: [what I think should happen].",
  "resolved": false
}
```

## Session End Protocol

### 1. Write the Session Note

Full detail. This is the permanent record of this shift.

**Location:** `sessions/{phase}/session-{NNN}.md`

**Contents:**
```markdown
# Session [NNN] -- Phase [N]

**Date:** [date]
**Duration:** [approximate]
**Agent:** [model identifier if relevant]

## Summary
[2-3 sentences: what was the goal, what was accomplished]

## What Was Done
[Detailed list of actions taken, in order]

## What Was Tried But Failed
[Approaches that didn't work, and WHY they didn't work.
This is often the most valuable section -- it prevents the next agent
from repeating dead-end approaches.]

## Decisions Made
[Any choices made during this session, with reasoning]

## What's Next
[What the next agent should do, in priority order]

## Watch Items
[Things that might go wrong. "If X happens, check Y."]

## Open Questions
[Things this agent couldn't resolve]
```

### 2. Update the Phase File

Compress. Only current state survives. History goes to the session note.

**The compression question:** "Would the next agent need this to do their job, or is it just context about how I got here?"
- If the next agent needs it to do their job -> phase file
- If it's context about how you got here -> session note only

Update these sections:
- **Current State** table (status, last session, blocking issues)
- **Active Watch Items** (add new ones, remove resolved ones)
- **Acceptance Checklist** (check off completed items)
- **Downstream Notes** (update if your work affects the next phase)
- **Rework Log** (if any rework happened this session)

### 3. Update STATUS.md

The production board. One line per phase showing current state. Update your phase's line.

### 4. Final Verification

Before ending the session:
- Run `git status` -- is there uncommitted work?
- Re-read the phase file -- does it accurately reflect reality?
- Check for untracked artifacts (temp files, logs, test output) that should be cleaned up or preserved.
- Write a closeout entry to dispatch_comms.jsonl.

```json
{
  "type": "closeout",
  "source": "agent",
  "phase": "3",
  "message": "Session [NNN] complete. [1-2 sentence summary]. Session note written. Phase file updated. STATUS.md updated. No uncommitted work.",
  "resolved": true
}
```

## The Golden Rule of Handoffs

**Write the handoff you wish you had received.**

If you arrived at this phase and the previous session note was thin, vague, or missing -- don't perpetuate that. Write the note that would have saved you 15 minutes of detective work. The 5 minutes you spend writing a good handoff saves the next agent 30 minutes of confusion.
