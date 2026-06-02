# Agent Conduct Template

This is a **template** for creating project-specific agent conduct documents. Replace all `[PLACEHOLDER]` sections with project-specific content. Delete the template instructions (lines starting with `>`) when creating the real document.

---

# [PROJECT NAME] -- Agent Conduct

## 1.0 Plan Mode and Approval Chain

Every task begins in plan mode. Before executing any work:

1. Read the required orientation documents (see Section 1.1).
2. Write a plan to `dispatch_comms.jsonl`:
   ```json
   {"type": "plan", "source": "agent", "tier": 1, "message": "PLAN: [what's changing, why, files affected, risks, verification method]", "resolved": false}
   ```
3. Dispatch reviews the plan first.
4. User approves. Do NOT execute until both have signed off.
5. Plans are living documents. If you need to adapt mid-execution, write an updated plan through the same chain.

> Template note: This is the core approval chain. Adapt tiers and approval authority to your project's needs.

## 1.1 Session Entry Protocol

At the start of every session, read these files in order:

1. `[STATUS_FILE]` -- Where is the project? What's active, blocked, complete?
2. `[HANDOFF_FILE]` -- What was the last session doing? What's next?
3. Your phase file -- What is your phase's purpose and current state?

> Template note: Replace with your project's actual file paths. Add any project-specific orientation docs.

If starting a new phase, also read:
4. `[DECISIONS_FILE]` -- Why key architectural choices were made.
5. `[MASTERPLAN_FILE]` -- Full spec (refer to specific sections as needed).

[ADD PROJECT-SPECIFIC REFERENCE DOCS HERE]

## 1.2 Project-Specific Rules

> Template note: This is where project-specific constraints go. Examples below -- replace with your actual rules.

[PLACEHOLDER: Add project-specific rules here. Examples:]
- [Rule about depth/quality constraints]
- [Rule about data format requirements]
- [Rule about performance constraints]
- [Rule about domain-specific correctness requirements]

## 1.3 Write Scope and Permissions

**Read:** Agents can read any file in the project.

**Write:** Agents can only write to files within their phase's write scope (defined in the phase file).

**Cross-phase writes:** Require a change order (see cross-phase protocol).

**Destructive operations:** Tier 2 approval required. This includes:
- Deleting files
- Force-pushing branches
- Dropping data
- Resetting state

> Template note: Adjust write permissions to match your project's trust model. Some projects may allow broader write access for frontier models.

## 1.4 Code Standards

> Template note: Replace this entire section with your project's coding standards.

[PLACEHOLDER: Project-specific code standards. Examples:]
- Language/framework conventions
- Naming conventions
- Error handling patterns
- Performance requirements (e.g., "no heap allocation in hot paths")
- Testing requirements
- Documentation requirements

## 1.5 Information Hierarchy

See `framework/information-hierarchy.md` in the playbook. The project-specific instantiation:

| Level | Location |
|-------|----------|
| 1. Phase reading list | `[PHASE_FILES_DIR]/{phase}.md` |
| 2. Project documentation | `[DOCS_DIR]/` |
| 3. Research reference library | `[RESEARCH_DIR]/` |
| 4. Saved web references | `[WEB_REFS_FILE]` |
| 5. Free web search | Last resort |

## 1.6 Session Protocol

Follow the session protocol from `framework/session-protocol.md`.

**Project-specific additions:**

Session notes location: `[SESSIONS_DIR]/{phase}/session-{NNN}.md`

> Template note: Add any project-specific session requirements (e.g., "always run the test suite before ending a session").

## 1.7 Cross-Phase Protocol

Follow the cross-phase protocol from `framework/cross-phase-protocol.md`.

Change orders location: `[CHANGE_ORDERS_DIR]/CO-{NNN}-{description}.md`

## 1.8 Communication Protocol

Follow the communication protocol from `framework/communication-protocol.md`.

Dispatch communications: `[DISPATCH_COMMS_FILE]`

## 1.9 Approval Tiers

| Tier | Requires | Examples |
|------|----------|---------|
| 0 | Auto-approve | [PROJECT-SPECIFIC TIER 0 EXAMPLES] |
| 1 | Dispatch approves | [PROJECT-SPECIFIC TIER 1 EXAMPLES] |
| 2 | User approves | [PROJECT-SPECIFIC TIER 2 EXAMPLES] |

> Template note: Customize the examples for your project. The tier structure itself (0/1/2) is standard.

## 1.10 Output Verification

Trust but verify. Specific verification requirements:

- Don't trust "N tests passed" -- verify coverage.
- Don't trust "N records generated" -- read actual data.
- Don't trust "build succeeded" -- verify the output artifact exists and is reasonable.

> Template note: Add project-specific verification requirements. What are the common "looks like it worked but actually didn't" failure modes in your project?

[PLACEHOLDER: Project-specific verification checklist]

## 1.11 Debugging Anti-Spiral

Each analysis pass must cite something **NEW** or you're spiraling. If you find yourself:
- Re-reading the same code
- Re-running the same test
- Restating the same hypothesis

STOP. Write down what you know, what you've tried, what you've ruled out. Identify what NEW information would unblock you. Go get it. If you can't identify new information, escalate.

## 1.12 Session-End Protocol

Before ending any session:

1. Write session note to `[SESSIONS_DIR]/{phase}/`.
2. Update phase file with current state (compress -- only what the next agent needs).
3. Update `[STATUS_FILE]`.
4. Run `git status` -- verify no uncommitted work.
5. Check for untracked artifacts.
6. Write closeout entry to dispatch_comms.jsonl.

> Template note: Add project-specific end-of-session checks.

## 1.13 Cleanup Audit

After each session, a fresh agent audits the outgoing agent's work:
- Uncommitted files?
- Log gaps?
- Untracked artifacts?
- Phase file accurately reflects state?
- Session note is complete?

The auditor reports to dispatch. It does not fix issues.

## 1.14 Adaptive Check-In Timing

| Situation | Check Frequency |
|-----------|----------------|
| Waiting on approval | Every 2 minutes |
| Just received a response | Every 5 minutes |
| Normal work | Every 10 minutes |
| Deep in long task | Every 15 minutes |

Need more time? Write an extension ticket.

## 1.15 Escalation Backoff

When user input is needed:
1. First notification: immediate
2. No response: retry after 2 minutes
3. No response: retry after 5 minutes
4. No response: retry after 10 minutes
5. Final: 30 minutes
6. After that: STOP. User is unavailable. Work pauses.

Never spam. Back off exponentially.

---

## Creating a Mythos Version

The Regular version above uses procedural guardrails: explicit checklists, step-by-step protocols, numbered sequences. This works well for models that benefit from structure and for projects where multiple models of varying capability may be used.

For frontier models that perform better with autonomy, create a **Mythos version** with the following changes:

### Same Knowledge, Same Boundaries

The Mythos version does NOT relax any constraints. The write scope is identical. The approval tiers are identical. The information hierarchy is identical. The session protocol requirements are identical.

What changes is the **expression**.

### Procedural to Principled

| Regular | Mythos |
|---------|--------|
| "Step 1: Read STATUS.md. Step 2: Read phase file. Step 3: ..." | "Orient yourself: production board, then station manual, then last shift's notes." |
| "Log to dispatch_comms.jsonl with format: ..." | "Keep dispatch informed at decision points. Be opinionated -- observation, analysis, recommendation." |
| "Each analysis pass must cite something NEW" | "Never retrace steps. If you're seeing the same things, you're spiraling. Find new ground or escalate." |
| Numbered checklists | Narrative paragraphs expressing intent |

### How to Create It

1. Take the Regular version.
2. For each section, ask: "What is the INTENT behind these steps?"
3. Express the intent as a principle or narrative, not as a procedure.
4. Keep all constraints explicit -- boundaries don't get softer, just more naturally expressed.
5. Test with your frontier model. If it follows the spirit correctly, the Mythos version is working. If it drifts, add back specific guardrails where needed.

### When to Use Which

- **Regular:** New projects, new phases, unfamiliar models, high-risk work, multiple models rotating through
- **Mythos:** Established projects, trusted frontier models, phases where the model has demonstrated competence

You can even mix: Mythos for overall conduct, Regular for specific high-risk protocols (e.g., cross-phase changes always use the formal change order process regardless of model capability).
