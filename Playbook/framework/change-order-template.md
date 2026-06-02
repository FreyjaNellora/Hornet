# Change Order Template

Use this template when an agent working on one phase discovers that another phase needs changes. Create the file in `change-orders/` with the naming convention `CO-{NNN}-{brief-description}.md`.

---

# Change Order CO-[NNN]

**Date:** [YYYY-MM-DD]
**Requested By:** Phase [X] (Session [NNN])
**Target Phase:** Phase [Y]
**Status:** open / approved / in-progress / resolved / denied

## What Needs Changing

[Specific description of what needs to change in the target phase. Reference existing spec sections where applicable. Be precise -- "the output format of function X" not "the parser."]

## Why

[Root cause. Why does this change need to happen? What problem did the requesting agent encounter? Include evidence -- error messages, test failures, data mismatches.]

## Impact Assessment

- [ ] Cosmetic (comments, naming, docs -- no behavioral change)
- [ ] Structural (interfaces, contracts, behavior -- downstream impact)
- [ ] Architectural (fundamental design -- multi-phase impact)

## Affected Phases

[List ALL phases that would be impacted by this change, not just the requesting and target phases. Consider downstream dependencies.]

| Phase | Impact |
|-------|--------|
| Phase [X] (requester) | [How this phase is affected / what it's waiting on] |
| Phase [Y] (target) | [What needs to change] |
| Phase [Z] (downstream) | [How this change would cascade, if at all] |

## Recommended Fix

[What the requesting agent thinks should be done. This is a RECOMMENDATION, not a directive. The target phase agent may propose a different solution.]

## Alternatives Considered

[Other approaches that were considered and why they were rejected. This helps the target phase agent understand the constraints.]

## Resolution

> This section is filled in when the change order is resolved.

**Resolved By:** [who -- user, dispatch, agent for Phase Y]
**Resolution Date:** [YYYY-MM-DD]
**Resolution:** [What was actually done. May differ from the recommended fix.]
**Verification:** [How was the fix verified? Tests, manual check, etc.]

---

## Template Notes (delete when using)

### Naming Convention
`CO-{NNN}-{brief-description}.md`
- NNN: Sequential number, zero-padded to 3 digits
- brief-description: Lowercase, hyphen-separated, max 5 words
- Example: `CO-017-parser-date-format.md`

### When NOT to Create a Change Order
- The change is within your own phase's write scope
- The change is a Tier 0 operation (auto-approve)
- You're updating shared documentation that your phase is allowed to touch

### When to ALWAYS Create a Change Order
- Any code change outside your write scope
- Any behavioral change to another phase's output
- Any interface change that crosses phase boundaries
- Any time you're tempted to "just quickly fix" something in another phase
