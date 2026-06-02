# Operations Sources

Research foundations for the methods used in this playbook. Each section covers what the method is, how it works in its original domain, and how it maps to the AI agent project context.

---

## Toyota Production System (TPS)

### Hikitsugu (Shift Handoff)

**What it is:** Toyota's formalized shift handoff process. The outgoing shift leader walks the incoming leader through the production line, pointing out active issues, pending work, and abnormalities. There is a mandatory overlap period where both shifts are present.

**How it works:**
- Written handoff document prepared before the overlap
- Verbal walkthrough at each station with abnormalities
- Incoming leader independently verifies critical items
- Both leaders sign off on the handoff
- Handoff document archived

**Mapping to AI agents:**
- Written handoff = session note + phase file update
- Verbal walkthrough = the reading list and detailed session note
- Independent verification = session start protocol (verify build, check status)
- Sign-off = dispatch/user confirmation
- Archive = session note in `sessions/` directory

### Andon (Signal for Help)

**What it is:** A signal system (originally a cord or button) that any worker can activate to flag a problem on the production line. When pulled, a team leader responds. If not resolved quickly, the line stops.

**How it works:**
- Worker identifies a problem they cannot resolve alone
- Pulls the andon cord -- a visual/audio signal activates
- Team leader arrives to assess
- If fixable quickly, the line continues
- If not, the line stops until the problem is resolved
- Every andon pull is logged and reviewed for systemic improvement

**Mapping to AI agents:**
- Andon pull = Tier 2 escalation in dispatch_comms.jsonl
- Team leader response = user or dispatch reviewing the escalation
- Line stops = agent work pauses on the blocked task
- Logging = change order + dispatch_comms entry
- Systemic review = patterns in change orders reveal process problems

### A3 Problem Solving

**What it is:** A structured problem-solving format that fits on a single A3-sized sheet of paper (11x17 inches). Forces concise, complete problem analysis.

**How it works:**
- Left side: Background, Current Condition, Goal, Root Cause Analysis
- Right side: Countermeasures, Implementation Plan, Follow-up
- Must fit on one page -- forces prioritization and clarity
- Shared openly for feedback before implementation

**Mapping to AI agents:**
- A3 format = change order template (structured, concise, complete)
- Root cause analysis = "Why" section of change orders
- Countermeasures = "Recommended Fix" section
- Sharing for feedback = Tier 2 approval chain

### Genchi Genbutsu (Go and See)

**What it is:** "Go to the actual place and see the actual thing." Decisions must be based on firsthand observation, not reports or assumptions.

**How it works:**
- When a problem is reported, go to the production floor
- Observe the actual process, not a description of it
- Verify data against reality
- Make decisions based on what you see, not what you expect

**Mapping to AI agents:**
- "Verify before claiming" principle
- Read the actual code before saying it's correct
- Run the actual test before saying tests pass
- Check actual output before saying generation succeeded
- Never trust summaries -- check the source

### Standardized Work

**What it is:** Detailed documentation of the current best practice for every operation. Not bureaucracy -- the foundation for improvement. You can't improve what you haven't standardized.

**How it works:**
- Every operation has a documented standard method
- Workers follow the standard until a better method is found
- Improvements are tested, verified, and then become the new standard
- Standards are living documents, not fixed rules

**Mapping to AI agents:**
- Phase files = standardized work for each station
- Session protocol = standardized work for shift operations
- Templates = standardized formats for artifacts
- When agents find better approaches, they update the standard (through the appropriate approval process)

---

## Lean Manufacturing

### Kanban (Signal Cards)

**What it is:** A visual signaling system that controls work-in-progress. Work is pulled, not pushed. Each station signals when it's ready for more work.

**How it works:**
- Visual board with columns: To Do, In Progress, Done
- WIP limits prevent overloading any station
- Work moves left to right as it progresses
- Blocked items are visually flagged

**Mapping to AI agents:**
- STATUS.md = the kanban board
- Phase statuses (not-started, in-progress, blocked, complete) = column positions
- One agent per phase at a time = WIP limit
- Blocked status in dispatch_comms = visual flag

### Tier Meetings (Escalation Layers)

**What it is:** A layered meeting structure where problems escalate through tiers of authority. Tier 1 (floor level) meets frequently. Problems that can't be resolved escalate to Tier 2 (supervisory), then Tier 3 (management).

**How it works:**
- Tier 1: Shift start, short standup, immediate issues
- Tier 2: Daily, cross-functional, systemic issues
- Tier 3: Weekly, strategic, resource allocation

**Mapping to AI agents:**
- Tier 0 (auto-approve) = floor-level decisions
- Tier 1 (dispatch) = supervisory review
- Tier 2 (user) = management decisions
- Escalation path mirrors the tier meeting structure

### Gemba Walk

**What it is:** Management practice of walking the production floor regularly to observe actual conditions, not reports about conditions.

**How it works:**
- Leaders physically visit the workspace
- Observe, don't direct
- Ask questions, don't give answers
- Look for discrepancies between standard and actual

**Mapping to AI agents:**
- User reviewing dispatch_comms.jsonl = gemba walk
- Cleanup audit after sessions = structured gemba walk
- The user can see actual agent work, not just agent reports about work

### Shift Pass-Down Log

**What it is:** A structured log that captures everything the next shift needs to know. Separate from the detailed work log -- this is the curated, compressed handoff.

**How it works:**
- Written during the last hour of the shift
- Contains: what's running, what's abnormal, what's pending, what to watch
- Read aloud at shift change
- Signed by both shifts

**Mapping to AI agents:**
- Session note = detailed shift log
- Phase file update = compressed pass-down
- HANDOFF.md = the "read aloud" summary
- The signature protocol = dispatch_comms closeout entry

---

## Medical Handoffs

### SBAR (Situation, Background, Assessment, Recommendation)

**What it is:** A structured communication framework developed by the US Navy and adopted widely in healthcare. Ensures critical information is transmitted clearly and completely.

**The framework:**
- **S -- Situation:** What is happening right now? (1-2 sentences)
- **B -- Background:** What is the relevant context? (Key facts, not the full history)
- **A -- Assessment:** What do I think is going on? (Professional judgment)
- **R -- Recommendation:** What do I think should be done? (Specific action)

**Example in medical context:**
- S: "Patient in Room 4 has blood pressure dropping -- 90/60, was 120/80 two hours ago."
- B: "72-year-old male, post-op day 1 from hip replacement, on blood thinners."
- A: "Possible internal bleeding given the trajectory and anticoagulation."
- R: "Request stat labs and surgical consult. Hold next anticoagulant dose."

**Mapping to AI agents:**
- Every dispatch_comms entry should follow SBAR structure
- S: What's happening (the observation)
- B: Relevant context (reference to phase, spec section)
- A: What the agent thinks it means (analysis)
- R: What the agent recommends (action)
- This is the "opinionated filtering" principle

### I-PASS (Illness Severity, Patient Summary, Action List, Situation Awareness, Synthesis)

**What it is:** A mnemonic for structured handoffs, developed at Boston Children's Hospital. Shown to reduce medical errors by 30% in clinical studies.

**The framework:**
- **I -- Illness Severity:** How critical is this? (Stable / Watch / Urgent)
- **P -- Patient Summary:** One-sentence summary of the situation
- **A -- Action List:** What needs to be done? (Prioritized, specific)
- **S -- Situation Awareness and Contingencies:** What could go wrong? "If X, then Y."
- **S -- Synthesis:** Receiver reads back understanding; sender confirms or corrects.

**Mapping to AI agents:**
- I = Phase status (in-progress / blocked / needs-rework)
- P = Summary in phase file Current State table
- A = "Next Action" field + session note "What's Next" section
- S = "Active Watch Items" in phase file
- S = Session start protocol (agent verifies understanding against the phase file)

---

## Nuclear Power Operations

### Shift Turnover Checklist

**What it is:** Nuclear power plants use extremely rigorous shift turnover procedures because errors can be catastrophic. The incoming shift independently verifies plant status against the outgoing shift's report.

**How it works:**
- Outgoing shift prepares written turnover package
- Package includes: plant status, equipment status, abnormalities, pending evolutions, regulatory notifications
- Incoming shift reads the package BEFORE the verbal turnover
- Verbal turnover covers items requiring judgment or emphasis
- Incoming shift independently verifies critical parameters (reads gauges, checks logs)
- Both shifts sign the turnover log
- Minimum overlap period is mandated

**Mapping to AI agents:**
- Written turnover package = session note + phase file
- Read before verbal = reading list protocol at session start
- Independent verification = agent checks build, runs status commands
- Sign-off = dispatch_comms entries
- Overlap period = the reading list ensures the incoming agent has full context before acting

### Three-Way Communication

**What it is:** A communication protocol to prevent misunderstanding of critical instructions.

**How it works:**
1. Sender states the instruction
2. Receiver repeats it back
3. Sender confirms or corrects

**Mapping to AI agents:**
- Agent writes plan to dispatch_comms (instruction)
- Dispatch reviews and acknowledges (repeat back)
- User confirms or corrects (confirmation)
- This is the Tier 1/Tier 2 approval chain

---

## Aviation

### ATIS (Automatic Terminal Information Service)

**What it is:** A continuously broadcast recording of current airport conditions. Pilots listen to it before contacting the tower, so they arrive already informed about the basics.

**How it works:**
- Broadcast includes: weather, active runways, NOTAMs, field conditions
- Updated regularly with new information letter (Alpha, Bravo, Charlie...)
- Pilot says "Information Charlie" to confirm they have the latest
- Tower doesn't need to repeat basic information

**Mapping to AI agents:**
- STATUS.md = ATIS broadcast. Updated continuously.
- Agent reads STATUS.md at session start = "listening to ATIS"
- Eliminates the need for dispatch to brief every agent on the basics
- The agent "has information" when they've read the current status

### Sterile Cockpit Rule

**What it is:** FAA regulation requiring that during critical phases of flight (below 10,000 feet), the cockpit crew focuses ONLY on duties related to safe operation. No extraneous conversation or activity.

**How it works:**
- Below 10,000 feet: only flight-critical communication
- No casual conversation, no administrative tasks
- Applies during takeoff, approach, and landing
- Violations are reported and investigated

**Mapping to AI agents:**
- During critical operations (data migration, destructive changes, cross-phase modifications): focus only on the task
- No opportunistic "while I'm here" fixes
- Complete the critical operation, verify it, THEN address other items
- Prevents the "I was just going to quickly fix this other thing too" failure mode

### Crew Resource Management (CRM) and Briefings

**What it is:** A set of training procedures for flight crews that emphasize communication, situational awareness, and decision-making.

**How it works:**
- Pre-flight briefing: review the plan, identify risks, assign responsibilities
- During flight: cross-check each other's work, speak up about concerns
- Post-flight debrief: what went well, what didn't, what to do differently

**Mapping to AI agents:**
- Pre-session: reading list protocol (the "briefing")
- During session: dispatch_comms entries (the "cross-check")
- Post-session: session note (the "debrief")
- The cleanup audit agent = the "second officer" cross-checking work

---

## Military Operations

### SITREP (Situation Report)

**What it is:** A standardized report format used to communicate current operational status up the chain of command.

**Standard format:**
- **Line 1:** Date/time group
- **Line 2:** Unit making report
- **Line 3:** Reference (location/operation)
- **Line 4:** Situation (what's happening)
- **Line 5:** Future plans (what's next)

**Mapping to AI agents:**
- dispatch_comms progress entries follow this pattern
- Phase + timestamp + situation + plan = SITREP
- Regular SITREPs at decision points keep dispatch/user informed

### OPORD (Operations Order)

**What it is:** A detailed plan for a military operation. Contains five paragraphs: Situation, Mission, Execution, Sustainment, Command/Signal.

**How it works:**
- **Situation:** Enemy forces, friendly forces, environment
- **Mission:** Who, what, when, where, why (the 5 W's)
- **Execution:** How -- detailed phases, tasks, coordinating instructions
- **Sustainment:** Logistics, resources, support
- **Command/Signal:** Who's in charge, how to communicate

**Mapping to AI agents:**
- Masterplan = the OPORD for the entire project
- Phase file = the task organization and execution details for one unit
- The five-paragraph structure maps well to project spec documents

### FRAGO (Fragmentary Order)

**What it is:** An abbreviated order that communicates changes to an existing OPORD. Only the changed elements are transmitted.

**How it works:**
- References the parent OPORD
- Contains ONLY the elements that have changed
- Everything not mentioned remains in effect
- Issued when the situation changes but the mission hasn't fundamentally changed

**Mapping to AI agents:**
- Change orders = FRAGOs
- Reference the original spec (don't restate everything)
- Describe only the delta
- If the change is big enough to restate the whole plan, it's a new OPORD (new phase or project restructure), not a FRAGO

### Commander's Intent

**What it is:** A concise expression of the purpose and desired end state of an operation. Written so that subordinates can make correct decisions even when communication is lost.

**How it works:**
- 2-3 sentences maximum
- Describes the PURPOSE (why we're doing this)
- Describes the END STATE (what success looks like)
- Does NOT prescribe method (how is left to subordinates)
- When the plan falls apart, commander's intent guides decisions

**Mapping to AI agents:**
- Phase file "Commander's Intent" section
- When an agent encounters an unanticipated situation, they ask: "What would achieve the commander's intent?"
- This is why intent is at the top of every phase file -- it's the fallback decision framework

### Battle Rhythm

**What it is:** The recurring cycle of events and activities that synchronize operations across a headquarters. Meetings, reports, and decision points at predictable intervals.

**How it works:**
- Daily: operations update brief, intelligence update
- Weekly: planning meetings, assessment reviews
- As needed: decision briefs for time-sensitive issues
- Every event has a standard format and expected outputs

**Mapping to AI agents:**
- Session start/end = daily battle rhythm events
- dispatch_comms check-in timing = the cadence
- Change orders = decision briefs
- The predictability of the rhythm lets everyone know when to expect what

### Running Estimate

**What it is:** A continuously updated assessment of the situation that informs decision-making. Not a snapshot -- a living document that reflects current understanding.

**How it works:**
- Maintained by each staff section
- Updated as new information arrives
- Contains: facts, assumptions, conclusions, recommendations
- Used to brief commanders and support decision-making

**Mapping to AI agents:**
- Phase file = the running estimate for that phase
- Updated every session with current understanding
- The "Current State" and "Active Watch Items" sections are the running estimate
- STATUS.md = the combined running estimate for the whole operation

---

## Site Reliability Engineering (SRE)

### Runbooks vs Incident Reports vs Shift Notes

**What it is:** SRE distinguishes three types of operational documents with different purposes and lifecycles.

**How they work:**

| Document | Purpose | Lifecycle | Update Pattern |
|----------|---------|-----------|---------------|
| Runbook | How to handle a known scenario | Long-lived | Updated when procedures change |
| Incident Report | What happened during a specific event | Permanent | Written once after resolution |
| Shift Notes | What happened during a shift | Permanent | Written once at shift end |

**Mapping to AI agents:**
- Runbooks = phase files + reference docs (how to do things)
- Incident reports = change order resolutions + decision records (what happened and why)
- Shift notes = session notes (what this agent did this session)
- The distinction prevents mixing "how to" with "what happened" with "what's happening now"

### Escalation Policies

**What it is:** Formal rules about who gets paged, when, and how urgently.

**How it works:**
- Severity levels define response time and responder
- Escalation paths define who to contact if the primary doesn't respond
- Backoff policies prevent alert fatigue
- On-call rotation prevents burnout

**Mapping to AI agents:**
- Tier 0/1/2 approval system
- Escalation backoff protocol (2 min, 5 min, 10 min, 30 min, stop)
- Prevents "spamming the user" anti-pattern

### Postmortems (Blameless)

**What it is:** After an incident, a structured review of what happened, why, and how to prevent recurrence. Blameless means focusing on systems, not individuals.

**How it works:**
- Timeline of events
- Root cause analysis (usually multiple contributing factors)
- What went well (don't just focus on failures)
- Action items with owners and deadlines
- No blame -- "the system allowed this to happen"

**Mapping to AI agents:**
- Session notes capture what failed and why
- Rework log in phase files tracks recurring issues
- The goal is systemic improvement, not agent blame
- "Why did the system allow this mistake?" not "why did the agent make this mistake?"

---

## Executive Assistant / Secretary Methods

### Daily Brief Format

**What it is:** How executive assistants prepare information for their principal's day.

**How it works:**
- Pre-digested information: not raw data, but analysis with recommendations
- Prioritized: most important first
- Actionable: each item has a clear "what do you need to do?"
- Brief: respects the principal's time and attention
- Flagged: items requiring decisions are clearly marked

**Mapping to AI agents:**
- dispatch_comms entries should be pre-digested with recommendations
- STATUS.md is the daily brief -- prioritized, actionable, flagged
- Agents don't dump information -- they filter, analyze, and recommend

### Meeting Minutes

**What it is:** A structured record of what was discussed, decided, and assigned during a meeting.

**How it works:**
- Date, attendees, agenda
- For each topic: discussion summary, decision made, action items with owners
- Distinct from a transcript -- captures decisions, not dialogue
- Distributed promptly for review

**Mapping to AI agents:**
- Session notes = meeting minutes for the "meeting" between the agent and the codebase
- Capture decisions, not process
- "We decided X because Y" not "first I looked at A, then B, then thought about C..."

### Tickler Files

**What it is:** A reminder system organized by date. Items that need attention on a future date are filed under that date.

**How it works:**
- 43 folders: 31 daily + 12 monthly
- Items filed under the date they need attention
- Check today's folder every morning
- Completed items are filed or discarded

**Mapping to AI agents:**
- Active Watch Items in phase files = tickler items
- "If X happens, do Y" = conditional ticklers
- The session start protocol includes checking watch items = checking today's tickler

### RACI Matrix

**What it is:** A responsibility assignment matrix. For each task: who is Responsible, Accountable, Consulted, Informed.

**How it works:**
- **R (Responsible):** Does the work
- **A (Accountable):** Signs off, owns the outcome
- **C (Consulted):** Provides input before the decision
- **I (Informed):** Told about the decision after it's made

**Mapping to AI agents:**
| Role | Agent Context |
|------|--------------|
| Responsible | The agent doing the work |
| Accountable | The user (always) |
| Consulted | Dispatch (for Tier 1), User (for Tier 2) |
| Informed | dispatch_comms.jsonl readers |

### Handled Log

**What it is:** A running log of everything an EA has handled, with disposition. Used to prove work was done, track patterns, and brief replacements.

**How it works:**
- Every incoming request is logged
- Disposition: handled, delegated, deferred, denied
- Referenced when questions arise about whether something was done
- Reviewed periodically for patterns

**Mapping to AI agents:**
- dispatch_comms.jsonl = the handled log
- Every plan, decision, escalation, and resolution is logged
- The `resolved` field tracks disposition
- Reviewable for patterns and audit

### Filing Systems

**What it is:** Organized storage with consistent naming, categorization, and retrieval methods.

**Key principles:**
- One place for each type of document
- Consistent naming conventions
- Index/table of contents for large collections
- Regular purging of outdated material

**Mapping to AI agents:**
- Three information stores (state/history/reference) = filing categories
- Naming conventions for session notes, change orders = consistent filing
- STATUS.md = the index
- Phase file compression = purging outdated material from operational files

---

## ML Pipelines

### MLflow / Weights & Biases (Experiment Tracking)

**What it is:** Platforms for tracking machine learning experiments -- parameters, metrics, artifacts, and results.

**How they work:**
- Every training run is logged with: hyperparameters, metrics over time, final results, artifacts (models, plots)
- Runs are comparable: "this run vs. that run" with clear parameter diffs
- Artifacts are versioned and linked to the run that produced them
- Failed runs are preserved (negative results are data)

**Mapping to AI agents:**
- Session notes = experiment logs (capture parameters, results, and what was learned)
- Phase files = the "best model so far" dashboard
- Failed approaches in session notes = negative results (just as valuable as positive)
- Artifact lineage: which session produced which output?

### Negative Results

**What it is:** The practice of formally recording approaches that didn't work.

**Why it matters:**
- Prevents the next agent from trying the same dead-end approach
- Often more informative than positive results ("this works" is less useful than "this doesn't work because X")
- In ML, knowing what hyperparameters DON'T work is half the optimization

**Mapping to AI agents:**
- Session notes "What Was Tried But Failed" section
- This is often the most valuable section of a session note
- Without it, agents repeat each other's failed experiments

### Artifact Lineage

**What it is:** Tracking which process, with which inputs and parameters, produced which output artifact.

**How it works:**
- Every output artifact is tagged with the process that created it
- Inputs are recorded: data versions, code versions, configurations
- Lineage is traceable: "this model was trained by run X on data version Y with config Z"

**Mapping to AI agents:**
- Session notes record which session produced which artifacts
- Phase files track current artifacts and their provenance
- When something breaks, lineage helps trace back to the source

---

## Knowledge Management

### DIKW Hierarchy (Data, Information, Knowledge, Wisdom)

**What it is:** A framework for understanding the progression from raw data to actionable wisdom.

| Level | Definition | Example |
|-------|-----------|---------|
| Data | Raw facts | "Test failed at line 47" |
| Information | Data with context | "The date parsing test fails because the timezone offset is wrong" |
| Knowledge | Information with understanding | "Timezone handling is broken because we use local time where UTC is expected" |
| Wisdom | Knowledge applied to decisions | "All time handling should use UTC internally, convert only at display" |

**Mapping to AI agents:**
- Session logs contain data and information
- Session notes elevate to knowledge (analysis, root causes)
- Phase files and decision records capture wisdom (principles, decisions)
- The compression from session log to session note to phase file follows the DIKW hierarchy upward

### Documentation Layering

**What it is:** The practice of maintaining documentation at multiple levels of detail, each serving a different audience and purpose.

**How it works:**
- Strategic layer: why (mission, vision, goals)
- Tactical layer: what (plans, specs, acceptance criteria)
- Operational layer: how (procedures, runbooks, standards)
- Historical layer: when/what happened (logs, reports, postmortems)

**Mapping to AI agents:**
- Strategic = Masterplan, Commander's Intent
- Tactical = Phase files, acceptance checklists
- Operational = Session protocol, agent conduct
- Historical = Session notes, decision records, change orders

### After Action Review (AAR)

**What it is:** A structured debrief conducted after an operation or project phase to capture lessons learned.

**The format:**
1. What was supposed to happen?
2. What actually happened?
3. Why was there a difference?
4. What can we do differently next time?

**Mapping to AI agents:**
- Session notes implicitly follow AAR structure
- "What Was Done" vs. "What Was Tried But Failed" = planned vs. actual
- "Why" analysis in session notes = root cause
- Phase file updates and rework log = systemic improvement

---

## Legal Operations

### Docket Systems

**What it is:** A tracking system for legal proceedings that ensures nothing falls through the cracks. Every case has deadlines, and missing a deadline can be malpractice.

**How it works:**
- Central register of all cases and their status
- Deadlines are tracked and flagged in advance
- Multiple reminder levels: 30 days, 7 days, 1 day
- Completion is confirmed and logged

**Mapping to AI agents:**
- STATUS.md = the docket (all phases and their status)
- Active Watch Items = deadline/condition monitoring
- Change orders with status tracking = case tracking
- Nothing falls through because everything is logged in dispatch_comms

### Closing Checklists

**What it is:** A comprehensive checklist used when closing a legal transaction to ensure every required step has been completed.

**How it works:**
- Every closing item is listed with: responsible party, due date, status
- Items are checked off only when verified complete
- The checklist is the authoritative record of what was done
- Signed by all parties

**Mapping to AI agents:**
- Acceptance checklist in phase files = closing checklist
- Items checked off only when verified (not when "done")
- Phase is not complete until all items are checked AND user confirms
- The checklist IS the acceptance criterion

### Bates Numbering

**What it is:** A sequential numbering system applied to documents in legal proceedings. Every page gets a unique, sequential identifier that never changes.

**How it works:**
- Sequential numbers applied to every page: BATES-000001, BATES-000002...
- Once numbered, the number is permanent
- Enables precise reference: "See BATES-000147" is unambiguous
- Gaps in numbering are flagged as potential issues

**Mapping to AI agents:**
- Sequential session numbering (Session 001, 002, 003...)
- Sequential change order numbering (CO-001, CO-002...)
- Numbers never reuse
- Gaps are noticeable and should be explained
- Enables precise reference in dispatch_comms and phase files

---

## Synthesis: How These All Fit Together

The playbook isn't a random collection of borrowed practices. These methods form a coherent system:

| Need | Primary Source | Supporting Sources |
|------|---------------|-------------------|
| Shift handoff without information loss | Toyota hikitsugu, Nuclear turnover | I-PASS, Military SITREP |
| Structured problem communication | Medical SBAR | Military FRAGO, SRE escalation |
| Cross-boundary coordination | Toyota andon, Legal change orders | Military FRAGO, RACI |
| Document organization | Knowledge Management DIKW | Legal filing, EA systems, SRE doc types |
| Quality verification | Toyota genchi genbutsu | Nuclear three-way comm, Aviation CRM |
| Status tracking | Lean kanban | Legal docket, Military running estimate |
| Decision recording | Knowledge Management AAR | SRE postmortem, ML experiment tracking |
| Escalation management | SRE escalation policies | Military chain of command, EA RACI |
| Anti-information-loss | ML negative results | Aviation sterile cockpit, Nuclear overlap |
| Compression and filtering | EA daily brief | DIKW hierarchy, Documentation layering |

The common thread across all these domains: **formalized information transfer prevents catastrophic failures.** Whether it's a patient dying from a missed medication, a nuclear plant melting down from a miscommunicated status, or a software project spiraling from lost context -- the failure mode is always the same: critical information didn't reach the person who needed it.

This playbook ensures that for AI agent projects, it always does.
