# Phase File Template

Use this template to create one file per phase in your project. Replace all `[PLACEHOLDER]` content. Delete the template instructions (lines starting with `>`).

---

# Phase [N]: [Name]

## Commander's Intent

> Template note: 2-3 sentences that capture the PURPOSE and DESIRED END STATE of this phase. Written in plain language. This is the "if all communication is lost, the agent should still be able to make correct decisions" statement. Borrowed from military operations: the commander's intent is what you fall back on when the plan breaks down.

[PLACEHOLDER: What is this phase trying to accomplish? What does success look like? What is the spirit of this work, not just the letter?]

## Reading List (Start Here)

> Template note: 3-5 files, curated for this phase. An agent should be able to read these and start working. Order matters -- most important first.

1. `[file]` -- [why this file matters for this phase]
2. `[file]` -- [why this file matters for this phase]
3. `[file]` -- [why this file matters for this phase]

## Write Scope

> Template note: Be specific. List files, directories, and modules this phase owns. Everything outside this list requires a change order.

**Owns (can create/modify/delete):**
- `[directory or file pattern]`
- `[directory or file pattern]`

**Shared (can modify with care, log changes):**
- `[file]` -- [what sections or aspects this phase can touch]

**Read-only (everything else):**
- All other project files

## Current State

| Field | Value |
|-------|-------|
| Status | not-started / in-progress / complete / blocked / rework |
| Last Session | [date] -- [link to session note] |
| Blocking Issues | none / [list with links to change orders or issues] |
| Next Action | [what the next agent should do first] |

## Acceptance Checklist

> Template note: These must be objectively verifiable. Not "code is clean" but "all functions have doc comments and all public APIs have tests." Each item should be checkable by an independent reviewer.

- [ ] [Criterion 1 -- specific, measurable, verifiable]
- [ ] [Criterion 2]
- [ ] [Criterion 3]
- [ ] [Criterion N]

## Active Watch Items

> Template note: Things that might go wrong, or conditions to monitor. Format: "If X happens, do Y." These are living -- add new ones as discoveries are made, remove them when resolved.

- [Watch item: "If X happens, do Y." or "Monitor Z because W."]

## Rework Log

> Template note: Track any time work in this phase had to be redone. This is not a shame log -- it's an information source. Patterns in rework reveal process problems.

| Date | Requested By | What Changed | Why | Impact |
|------|-------------|-------------|-----|--------|
| | | | | |

## Downstream Notes

> Template note: What does the next phase (or any dependent phase) need to know about your output? Format, assumptions, known limitations, gotchas.

[PLACEHOLDER: What the next phase needs to know about this phase's output. Include:]
- Output format and location
- Assumptions the next phase can rely on
- Known limitations or edge cases
- "If you see X, it's because Y" explanations

---

## Usage Notes

### When to Update This File

- **Session start:** Verify Current State is accurate.
- **Session end:** Update Current State, Active Watch Items, Acceptance Checklist, Downstream Notes.
- **On rework:** Add entry to Rework Log.
- **On change order (incoming):** Add to Active Watch Items.
- **On change order (resolved):** Update affected sections, remove resolved watch items.

### Compression Rule

This file should contain ONLY what the next agent needs to do their job. History, reasoning, failed approaches, and detailed context belong in session notes, not here.

**Test:** "Would the next agent need this to do their job, or is it just context about how I got here?"
- Needs it to do their job -> keep in phase file
- Context about how you got here -> session note only
