# The Lights-Out Factory Model

## The Analogy

A lights-out factory runs without human intervention on the floor. Machines execute. Humans design the process, set quality gates, and intervene only when the andon cord is pulled.

An AI agent project works the same way:

| Factory Concept | Project Equivalent |
|----------------|-------------------|
| Production line | The project as a whole |
| Station | A phase (design, implementation, testing, etc.) |
| Worker | An AI agent |
| Shift | A session (one agent's continuous work period) |
| Shift handoff | Session notes + phase file updates |
| Production board | STATUS.md |
| Station manual | Phase file + reading list |
| Andon cord | Tier 2 escalation to user |
| Quality gate | Acceptance checklist at phase boundary |
| Change order | Cross-phase modification request |
| Foreman | Dispatch (orchestration layer or user) |

### Why This Works for AI Agents

AI agents have specific constraints that make the factory model natural:

1. **No persistent memory.** Each session starts cold. Like a shift worker arriving at the factory, the agent must read the handoff to know what's happening.
2. **Bounded context windows.** Agents can't hold everything in memory. The information hierarchy tells them what to read first and what to skip.
3. **Variable capability.** Different models have different strengths. Standardized interfaces (phase files, session notes, change orders) let any capable model slot in.
4. **Parallelizable work.** Independent phases can run simultaneously with different agents, just like independent stations on a production line.

## Three Information Stores

These three categories of information serve fundamentally different purposes. **Never mix them.** When information is in the wrong store, agents waste time reading irrelevant material, miss critical updates, or make decisions based on stale data.

### 1. State (What's True Now)

The current condition of the project. Updated continuously. Always reflects reality as of the last update.

- **STATUS.md** -- The production board. Which phases are active, blocked, complete. What's the critical path.
- **Phase files** -- Current state of each phase. What's done, what's pending, what's blocked.
- **Active issue tracker** -- Open bugs, open questions, unresolved decisions.

**Rule:** State documents are **replaced**, not appended. When the state changes, the old state is overwritten. History is preserved elsewhere.

### 2. History (What Happened)

A permanent record of events. Written once, never modified.

- **Session notes** -- Full detail of what an agent did, tried, learned, and recommends.
- **Decision logs** -- What was decided, why, what alternatives were considered.
- **Change order resolutions** -- How cross-phase issues were resolved.
- **Audit logs** -- Formal records of phase completion, acceptance criteria met.

**Rule:** History documents are **append-only**. Never edit a session note after the session ends. If something was wrong, create a new entry correcting it.

### 3. Reference (How Things Work)

Stable documentation that changes rarely. The "manuals" of the project.

- **Masterplan / spec** -- What the project is, what each phase does, acceptance criteria.
- **Architecture docs** -- How systems are designed and why.
- **Rules references** -- Domain-specific rules (game rules, API contracts, business logic).
- **Decision records (ADRs)** -- Why key architectural choices were made.

**Rule:** Reference documents change only through deliberate, approved updates. When reference changes, all downstream phase files must be checked for impact.

## The Documentation Layering Table

Each layer has a different persistence, update cadence, and purpose. Reading from top to bottom gives progressively more volatile information.

| Layer | Persistence | Update Frequency | Content | Examples |
|-------|------------|-------------------|---------|----------|
| Permanent reference | Years | Rarely | How systems work, architecture, domain rules | Masterplan, architecture docs, ADRs, rules reference |
| Operational state | Continuous | Real-time | Current status, active issues, what's true right now | STATUS.md, phase files, issue tracker |
| Shift handoff | Per transition | Each handoff | Deltas since last shift, judgment calls, watch items, pending work | HANDOFF.md, phase file "Active Watch Items" |
| Event records | Permanent | Once (at event time) | What happened, why, what was learned | Session notes, decision logs, change order resolutions |
| Session logs | Permanent | Once (during session) | What was tried, results, reasoning, recommendations | dispatch_comms.jsonl entries, detailed session notes |

### How Layers Interact

**Top-down flow:** Reference defines what should happen. State tracks what IS happening. Handoffs communicate what JUST happened. Event records preserve what DID happen. Session logs capture the full granularity.

**Bottom-up flow:** When session logs reveal that reference is wrong (a spec is incomplete, a rule is ambiguous), that discovery flows up through a change order, gets approved, and updates the reference layer.

**Compression principle:** Information flows upward by compression. A session log might be 500 lines. The session note distills it to 50 lines. The phase file update distills it to 5 lines. STATUS.md gets one line. Each layer preserves only what its audience needs.

## Station Independence

Each phase (station) should be as independent as possible:

1. **Clear inputs and outputs.** What does this phase receive? What does it produce?
2. **Defined write scope.** What files/modules does this phase own? Everything else requires a change order.
3. **Self-contained reading list.** An agent starting this phase can get up to speed by reading 3-5 documents.
4. **Testable acceptance criteria.** How do you know this phase is done? Not "it looks good" but "these specific checks pass."

When phases must interact, they do so through the cross-phase protocol (change orders), never through informal side-channels.

## Quality Gates

A phase is not complete until:

1. All acceptance criteria are met (not "I think they're met" -- verified).
2. The phase file is updated to reflect completion.
3. Downstream notes are written (what the next phase needs to know).
4. STATUS.md is updated.
5. A session note captures the final state.
6. The user confirms completion (phases aren't done until the user says so).

## Running Multiple Stations

When the project has independent phases that can run in parallel:

- Each agent gets their own phase file and write scope.
- Agents communicate through change orders, not direct interaction.
- STATUS.md is the shared view of the whole line.
- The user (or dispatch layer) coordinates sequencing and resolves conflicts.

## Anti-Patterns

| Anti-Pattern | Why It Fails | Correct Approach |
|-------------|-------------|-----------------|
| Mixing state and history | State gets cluttered with old info; agents read stale context | State replaces. History appends. Separate files. |
| Skipping session notes | Next agent starts blind, repeats work | Always write session notes, even for "nothing happened" sessions |
| Informal cross-phase fixes | Changes break downstream assumptions, no audit trail | Change orders for everything outside your write scope |
| Over-documenting state | Phase files become novels; agents can't find current info | Compress ruthlessly. Details live in session notes. |
| Under-documenting history | Lessons are lost; same mistakes repeat | Session notes capture what was tried AND what failed |
| Trusting process output | "500 tests passed" but did they test the right things? | Spot-check actual results. Read real data. |
