# Cross-Phase Protocol

How to handle work that crosses phase boundaries. Based on Toyota andon, military FRAGO, and legal change order systems.

## The Problem

When an agent working on Phase X discovers that Phase Y needs changes, they face a dilemma:
- If they fix it themselves, they might break assumptions that Phase Y's agent was relying on.
- If they ignore it, the problem persists and may compound.
- If they try to do both their own work AND the cross-phase fix, they lose focus and context.

The solution: **formalize the handoff.** Create a change order, log it, and let the right agent handle it.

## The Protocol

### Step 1: Stop Work on the Cross-Phase Issue

Do not attempt to fix something outside your write scope. Even if the fix seems trivial. Even if you're "sure" it's correct. The point of phase boundaries is that different contexts hold different knowledge.

### Step 2: Create a Change Order

Create a file in `change-orders/` using the change order template (see `change-order-template.md`).

**Naming convention:** `CO-{NNN}-{brief-description}.md`

Example: `CO-017-parser-output-format-mismatch.md`

### Step 3: Log It

Write to dispatch_comms.jsonl:
```json
{
  "type": "change-order",
  "source": "agent",
  "tier": 2,
  "phase": "3",
  "message": "CHANGE ORDER CO-017: Phase 3 discovered that Phase 2's parser outputs dates as strings, but Phase 3 expects epoch integers. See change-orders/CO-017-parser-output-format-mismatch.md. Impact: structural (interface change). Requesting approval to proceed.",
  "resolved": false
}
```

### Step 4: Continue Other Work

If you have other work within your phase that isn't blocked by this issue, continue it. Don't block your entire session on a cross-phase dependency.

If your entire phase is blocked, say so clearly:
```json
{
  "type": "blocked",
  "source": "agent",
  "tier": 2,
  "phase": "3",
  "message": "Phase 3 blocked on CO-017. Cannot proceed with integration testing until parser output format is resolved. No other Phase 3 work available.",
  "resolved": false
}
```

### Step 5: Resolution

The user (or dispatch layer) handles resolution by:
1. Reviewing the change order
2. Either:
   - Spawning an agent for the target phase to make the change
   - Authorizing the requesting agent to make the specific, scoped change
   - Deciding the change isn't needed and explaining why
3. Marking the change order as resolved

## The FRAGO Principle

From military operations: a Fragmentary Order (FRAGO) communicates only what's **different** from the existing plan. It doesn't restate the full operations order.

Change orders follow the same principle:
- Reference the existing spec/design. Don't restate it.
- Describe only the **delta** -- what needs to change and why.
- If the change is complex enough to require restating the full design, that's a sign it might be a new phase, not a change order.

**Good change order:** "Phase 2's parser (per Masterplan Section 4.3) outputs dates as ISO 8601 strings. Phase 3 needs epoch integers for the time-series database. Recommend: Phase 2 adds a `format` parameter defaulting to ISO 8601 but supporting epoch."

**Bad change order:** "The parser reads input files and extracts fields including dates. Currently it formats them as strings. The database needs integers. Here's how the parser works: [500 words]. Here's how the database works: [500 words]. Here's what I think should change: [100 words]."

## Both Phase Files Get Updated

When a change order is created:

**Requesting phase file** (e.g., Phase 3):
```markdown
## Active Watch Items
- CO-017: Waiting on Phase 2 parser output format change. Phase 3 integration testing blocked.
```

**Target phase file** (e.g., Phase 2):
```markdown
## Active Watch Items
- CO-017: Change requested by Phase 3 -- parser output format needs epoch integer option. See change-orders/CO-017.md.
```

When the change order is resolved, both phase files are updated to reflect the resolution, and the watch items are removed.

## Impact Assessment

Every change order must classify its impact:

### Cosmetic
- Comments, naming, documentation
- No behavioral change
- No downstream impact
- Can often be auto-approved (Tier 1)

### Structural
- Interfaces, contracts, data formats, behavior
- Has downstream impact
- Requires approval (Tier 2)
- Affected phases must be identified

### Architectural
- Fundamental design changes
- Affects multiple phases
- May require rethinking subsequent phases
- Requires thorough review and user approval (Tier 2, potentially with discussion)

## When the Current Agent Can Fix It

Sometimes the user will authorize the requesting agent to make the cross-phase fix directly. This is acceptable when:

1. The fix is well-scoped and specific
2. The agent understands both phases
3. The change order documents what was done
4. Both phase files are updated
5. The fix is verified (builds, tests pass)

Even in this case, the change order still gets created and resolved. The audit trail is non-negotiable.

## Common Cross-Phase Issues

| Issue Type | Typical Resolution |
|-----------|-------------------|
| Interface mismatch (format, schema) | Target phase adjusts output to match spec, or spec is updated |
| Missing functionality | Target phase adds the feature, with updated acceptance criteria |
| Performance issue | Usually requires investigation by the target phase |
| Bug in upstream output | Target phase fixes the bug |
| Spec ambiguity | User clarifies, both phases update their understanding |
| Design conflict | User decides, potentially with ADR |

## Anti-Patterns

| Anti-Pattern | Why It Fails | Correct Approach |
|-------------|-------------|-----------------|
| "Quick fix" across phase boundary | Breaks assumptions, no audit trail, may introduce subtle bugs | Create change order, even for "trivial" fixes |
| Waiting silently for resolution | User doesn't know you're blocked, time is wasted | Log blocked status clearly and immediately |
| Over-specifying the fix | Telling the target phase HOW to fix it constrains their approach | Describe the PROBLEM. Let the target phase propose the solution. |
| Ignoring the issue | Problem compounds, discovered later when it's harder to fix | Always create the change order. Future you will thank present you. |
| Multiple changes in one CO | Hard to track, hard to approve, hard to resolve | One change order per discrete issue |
