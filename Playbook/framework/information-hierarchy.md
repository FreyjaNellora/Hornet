# Information Hierarchy

## The 5-Level Lookup Order

When an agent needs information, they should search in this order. Each level is cheaper and faster than the next. Most work should never get past Level 2.

### Level 1: Phase Reading List (Start Here)

**Cost:** Minimal. 3-5 files, pre-selected for relevance.
**When to use:** Every session start. Every time you're unsure what to do next.

The phase file contains a curated reading list: the 3-5 documents that tell you everything you need to operate in this phase. These were selected by the person who designed the phase or by the previous agent who refined them.

If the reading list doesn't answer your question, it should at least tell you WHERE to look next.

**Example:**
```
## Reading List (Start Here)
1. STATUS.md -- Where is the project overall?
2. phases/phase-3.md -- What is this phase's purpose and current state?
3. sessions/phase-3/session-12.md -- What did the last agent do?
4. docs/architecture.md, Section 4 -- How the module I'm working on fits together
5. docs/rules-reference.md -- Domain rules I must follow
```

### Level 2: Project Documentation

**Cost:** Low. Files are local, indexed, and organized.
**When to use:** When the reading list doesn't cover a specific question.

This is the full project documentation:
- Masterplan / spec (what the project is supposed to do)
- Architecture docs (how it's designed)
- Decision records / ADRs (why key choices were made)
- Rules references (domain-specific constraints)
- Prior session notes (what other agents learned)
- Change order history (what cross-phase issues were raised and resolved)

**Search strategy:** Use file search (glob) and content search (grep) before reading files linearly. Most questions can be answered by searching for keywords.

**Example questions answered at this level:**
- "What are the acceptance criteria for this phase?"
- "Why was this design decision made?"
- "Has another agent already tried this approach?"
- "What does the downstream phase expect as input?"

### Level 3: Research Reference Library

**Cost:** Medium. Pre-fetched material that needs reading and synthesis.
**When to use:** When the question is about HOW to implement something using established methods.

A curated collection of academic papers, technical references, and industry best practices that are relevant to the project's domain. These were gathered during project setup or prior research sessions and saved locally.

**Examples:**
- For a chess engine: papers on NNUE architecture, search algorithms, evaluation design
- For a web app: framework documentation, API references, security best practices
- For an ML project: relevant papers, benchmark results, known failure modes

**Key principle:** This library should be populated PROACTIVELY during project setup or research phases. Agents doing implementation work should find what they need here, not on the web.

### Level 4: Saved Web References

**Cost:** Higher. Requires parsing saved URLs and summaries.
**When to use:** When the research library doesn't cover a specific topic, but a prior agent bookmarked a relevant resource.

A log of URLs, summaries, and key findings from prior web research. Organized by topic. Includes:
- The URL
- When it was accessed
- A summary of what was found
- Key takeaways relevant to the project
- Whether the information was verified

**Format example:**
```
## Topic: Incremental Hash Updates for Board Games
- URL: https://example.com/zobrist-hashing
- Accessed: 2025-03-15
- Summary: Explains Zobrist hashing with incremental XOR updates.
- Key takeaway: Use random 64-bit values per (piece, square) pair. XOR is its own inverse.
- Verified: Yes, implemented and tested in Phase 4.
```

### Level 5: Free Web Search (Last Resort)

**Cost:** Highest. Slow, noisy, may return outdated or incorrect information.
**When to use:** Only when Levels 1-4 are exhausted AND the question is critical to current work.

**Before searching the web:**
1. Confirm the question can't be answered from existing project docs.
2. Confirm the question can't be answered from the research library.
3. Confirm no prior agent has already researched this topic.
4. Formulate a specific query (not "how do I do X" but "algorithm for X with constraints Y and Z").

**After searching the web:**
1. Save the finding to the Level 4 reference log.
2. Note the source quality and whether the information needs verification.
3. If the information is important enough to act on, consider adding it to the Level 3 research library.

**Warning:** Web search results can be outdated, wrong, or misleading. Always cross-reference findings against multiple sources. Never implement something based on a single web search result without verification.

## Why This Order Matters

| Level | Time Cost | Noise | Reliability |
|-------|-----------|-------|-------------|
| 1. Phase reading list | Seconds | Zero (curated) | High (selected for this work) |
| 2. Project docs | Minutes | Low (organized) | High (project-specific) |
| 3. Research library | Minutes-hours | Medium (needs synthesis) | Medium-high (pre-vetted) |
| 4. Saved web refs | Minutes | Medium (summaries help) | Medium (depends on source) |
| 5. Free web search | Minutes-hours | High (unfiltered) | Low-medium (unverified) |

## Agent Responsibility

Every agent has two information duties:

1. **Consume efficiently.** Follow the hierarchy. Don't jump to web search when the answer is in the phase file.
2. **Contribute back.** When you learn something, put it in the right level:
   - Discovered a useful doc? Add it to the phase reading list.
   - Made a decision? Write an ADR in project docs.
   - Found a useful paper? Add it to the research library.
   - Found a useful URL? Log it in saved web references.
   - The goal is that Level 5 gets used less and less over time.

## Populating the Library (Project Setup)

During project initialization or dedicated research phases:

1. Identify the key domains the project touches.
2. Gather foundational references for each domain.
3. Summarize and organize into the research library (Level 3).
4. Bookmark specific URLs with summaries (Level 4).
5. Create reading lists for each phase that reference the most relevant materials (Level 1).

This upfront investment pays for itself many times over. An agent who can answer their question from Level 2 instead of Level 5 saves 10-30 minutes per lookup and gets more reliable information.
