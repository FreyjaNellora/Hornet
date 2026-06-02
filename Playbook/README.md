# Universal Operations Playbook

A project-agnostic framework for running lights-out AI agent projects.

## What This Is

This playbook codifies operational methods drawn from manufacturing, medicine, military operations, aviation, nuclear power, site reliability engineering, and executive assistance into a single coherent framework for AI agent projects.

The core insight: an AI agent session is a **shift** in a **factory**. The agent is a worker who arrives, reads the shift handoff, does their work, and leaves detailed notes for the next shift. The project is a production line with phases (stations), each with clear inputs, outputs, and acceptance criteria.

This framework ensures:
- **Zero-knowledge cold starts.** Any agent can pick up any phase at any time.
- **No information loss.** Every decision, failure, and insight is captured.
- **Clean boundaries.** Agents know what they own and what requires escalation.
- **Minimal user intervention.** The system runs lights-out except for explicit approval gates.

## How To Use It

### For a New Project

1. Copy this entire `Playbook/` directory into your project (or reference it externally).
2. Create your project's masterplan document defining all phases, acceptance criteria, and architecture.
3. Use `framework/agent-conduct-template.md` to create your project-specific agent conduct file. Fill in the placeholders.
4. Use `framework/phase-file-template.md` to create one file per phase.
5. Create a `STATUS.md` (production board) and `HANDOFF.md` (shift handoff) in your project root or docs directory.
6. Create a `sessions/` directory for session notes, organized by phase.
7. Create a `change-orders/` directory for cross-phase change requests.
8. Set up `dispatch_comms.jsonl` as the real-time communication channel.

### For an Existing Project

If you already have documentation, map it to the three information stores:
- **State** (what's true now): STATUS.md, phase files, active issue trackers
- **History** (what happened): session notes, decision logs, change order resolutions
- **Reference** (how things work): masterplan, architecture docs, rules references

Then layer the session protocol on top.

## Directory Structure

```
Playbook/
  README.md                              -- This file
  framework/
    factory-model.md                     -- The core operational model
    information-hierarchy.md             -- 5-level lookup order for agents
    session-protocol.md                  -- How to run a session (shift)
    cross-phase-protocol.md              -- How to handle cross-phase work
    communication-protocol.md            -- How agents communicate with users
    agent-conduct-template.md            -- Template for project-specific conduct docs
    phase-file-template.md               -- Template for per-phase files
    change-order-template.md             -- Template for cross-phase change orders
  research/
    operations-sources.md                -- All source frameworks and how they map
```

## Principles

1. **Separate state from history from reference.** Never mix these three information stores.
2. **Compress forward, preserve backward.** Phase files compress to current state. Session notes preserve full detail.
3. **Agents are opinionated.** Don't just report -- recommend. Pre-digest information. Attach a recommendation to every finding.
4. **Trust but verify.** Never trust that a process succeeded based on output alone. Spot-check actual results.
5. **Escalate, don't improvise.** When something is outside your scope, create a change order and move on. Don't silently fix things you don't own.
6. **Frustration is signal.** When the user pushes back, stop executing and start listening. They see something you don't.

## Adapting for Model Capability

The templates include notes on two modes:
- **Regular mode:** Procedural guardrails, explicit checklists, step-by-step protocols. For models that benefit from structure.
- **Mythos mode:** Same knowledge and boundaries, but expressed as principles and intent rather than procedures. For frontier models that perform better with autonomy.

Choose the mode that matches your model's capability level. The boundaries are identical -- only the expression changes.
