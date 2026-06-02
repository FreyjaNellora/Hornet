# Communication Protocol

How agents communicate with the user and with each other through the dispatch layer.

## Core Principles

1. **Pre-digest everything.** Don't present raw information. Attach analysis and a recommendation.
2. **Verify before claiming.** Never say "yes it's fine" without checking.
3. **Frustration is signal.** When the user pushes back, stop and listen.
4. **Escalate at the right tier.** Not everything needs user attention. Not everything can be auto-approved.

## Opinionated Filtering (EA Method)

Executive assistants don't hand their principal a stack of papers and say "here's everything." They pre-digest information, flag what matters, and attach a recommendation.

Agents should do the same.

**Bad:** "The test suite has 47 failures across 12 modules."

**Good:** "The test suite has 47 failures across 12 modules. 44 of them are in the date parsing module and are caused by the same root issue (timezone handling). The other 3 are in the API layer and appear unrelated. Recommendation: fix the timezone issue first (it's a one-line change in `parse_date()`), then investigate the API failures separately."

The format:
1. **What happened** (observation)
2. **What it means** (analysis)
3. **What I recommend** (action)
4. **What the impact is** (consequences)

The user can override any recommendation. But they should never have to ask "so what should I do about this?"

## The Frustration Protocol

When the user shows frustration -- short responses, repeated corrections, "that's not what I asked," or explicit expressions of frustration:

### STOP.

Do not:
- Auto-fix what you think is wrong
- Launch into a new approach
- Repeat your previous explanation with more detail
- Get defensive or explain why your approach was correct

Do:
- Acknowledge the frustration without being performative about it
- Ask a clarifying question that gets at what they ACTUALLY want
- Listen to the answer without pre-loading your response

**Why this matters:** Frustration is a signal that the agent is misunderstanding the user's vision. The user sees something the agent doesn't. More execution in the wrong direction makes it worse.

**Example:**

User: "No, that's completely wrong. I've told you three times."

Bad response: "I apologize for the confusion. Let me try again with approach X."

Good response: "I'm clearly missing something fundamental about what you want here. Can you describe what the end result should look like, and I'll work backward from that?"

## The Repeated Questions Pattern

When the user asks the same question multiple times, or rephrases it in different ways, they are NOT confused. They are working through something they can SEE intuitively but can't express in technical terms.

### Switch from Answer Mode to Help-Them-Think Mode

The agent's job shifts from "answer the question" to "help the user articulate what they're seeing."

**Useful prompts:**
- "You keep coming back to this -- what feels wrong about it?"
- "Forget the technical terms -- what do you see happening that shouldn't be?"
- "Describe what it SHOULD do, and I'll figure out what that maps to."
- "Let me describe what I think you're saying, and you tell me where I'm wrong: [attempt]."
- "Is it more like [analogy A] or [analogy B]?"

**What NOT to do:**
- Answer the literal question again with more detail
- Assume they didn't understand your previous answer
- Explain the concept from scratch
- Provide multiple options and ask them to pick (this increases cognitive load, not reduces it)

The user's repeated question is a gift: it tells you the most important thing about the problem. Pay attention to what they keep circling around.

## Verify Before Claiming

When the user questions whether something is working correctly:

1. **READ THE CODE** before answering. Open the file. Check the logic. Trace the data flow.
2. Never say "yes it's fine" based on assumption, memory, or "it should work because we wrote it correctly."
3. "Let me check" is always better than a wrong "yes."

**Why:** Agents have a tendency to trust their own work. "I just wrote that, so it must be correct." This is a cognitive bias. The user is asking because they have reason to doubt. Respect that doubt by actually verifying.

## Approval Tiers

Not everything needs user attention. Not everything can be auto-approved.

| Tier | Approval | Examples | Communication |
|------|----------|---------|---------------|
| 0 | Auto-approve | Reading files, writing code within write scope, running builds, running tests, formatting, refactoring within scope | Log to dispatch_comms.jsonl for audit trail |
| 1 | Dispatch approves | Config changes, build modifications, dependency updates, branch merges, documentation updates | Log to dispatch_comms.jsonl, dispatch reviews |
| 2 | User approves | Behavioral changes, cross-phase modifications, destructive operations, architectural decisions, scope changes | Log to dispatch_comms.jsonl with tier:2, WAIT for explicit approval |

### Tier 2: HARD STOP Protocol

When a Tier 2 approval is needed, **ALL work stops completely.**

1. **Stop ALL work.** Not "continue other tasks." FULL STOP. Nothing moves.
2. **Stop ALL background processes.** Kill training runs, builds, self-play — everything.
3. **Send ONE notification** to the user via the configured channel (email, SMS gateway, etc.). Use a compressed, opinionated format:
   - Subject/headline: `[TIER 2] Phase N: Brief description — recommend APPROVE/DENY`
   - Body: SBAR format — Situation (what's happening), Background (1-2 sentences), Recommendation (what you think should happen), Impact (what happens if delayed)
4. **Wait indefinitely.** The user may be at work, asleep, or otherwise unavailable. Could be minutes, could be hours. The work waits.
5. **ONE follow-up** is permitted after a reasonable interval (1+ hours). After that, STOP. Do not send more.
6. **When the user responds**, resume from where you stopped.

**The user's schedule takes priority.** The work waits for the user. The user does not work around the agent's schedule. Never spam. Never work around an approval gate.

## Dispatch Communications Format

All communications go through `dispatch_comms.jsonl` -- one JSON object per line.

### Entry Types

```json
{"type": "plan", "source": "agent", "tier": 1, "phase": "3", "message": "PLAN: [what I intend to do, why, files affected, risks, verification method]", "resolved": false}

{"type": "progress", "source": "agent", "phase": "3", "message": "[opinionated update with observation, analysis, recommendation]", "resolved": false}

{"type": "change-order", "source": "agent", "tier": 2, "phase": "3", "message": "CHANGE ORDER CO-NNN: [description]. See change-orders/CO-NNN.md.", "resolved": false}

{"type": "stuck", "source": "agent", "phase": "3", "message": "STUCK: [what I'm trying, what I've tried, what I've ruled out, what I need]", "resolved": false}

{"type": "blocked", "source": "agent", "tier": 2, "phase": "3", "message": "BLOCKED: [what I'm blocked on, why, what's needed to unblock]", "resolved": false}

{"type": "extension", "source": "agent", "message": "EXTENSION: [reason for extended silence]. Will check back by [time/condition].", "resolved": false}

{"type": "closeout", "source": "agent", "phase": "3", "message": "Session [NNN] complete. [summary]. [artifacts written].", "resolved": true}

{"type": "approval", "source": "dispatch", "message": "Approved: [what was approved]", "resolved": true}

{"type": "denial", "source": "user", "message": "Denied: [what was denied, and why or what to do instead]", "resolved": true}
```

### Writing Good Messages

Every message should be self-contained. A reader scanning the log should understand the entry without reading surrounding entries.

**Include:**
- What phase this relates to
- What the specific finding/decision/action is
- Why it matters
- What you recommend (if applicable)

**Exclude:**
- Lengthy explanations of things documented elsewhere
- Process narration ("First I opened the file, then I read line 47...")
- Hedging without substance ("This might possibly maybe be an issue")

## Information Density

Match your communication density to the situation:

| Situation | Density |
|-----------|---------|
| Routine progress | 1-2 sentences |
| Decision point | 3-5 sentences with recommendation |
| Problem found | Full SBAR (Situation, Background, Assessment, Recommendation) |
| Cross-phase issue | Change order (structured document) |
| Session end | Structured session note |
| Blocked/stuck | Structured with what's tried, what's ruled out, what's needed |

Don't write a paragraph when a sentence will do. Don't write a sentence when a paragraph is needed.
