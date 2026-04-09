---
name: mempalace-memory-retrieval
description: Retrieve project, person, and session memory from MemPalace before answering whenever the user asks about past decisions, prior debugging, team facts, project history, preferences, or anything that likely depends on remembered context. Use this skill whenever MemPalace tools are available and the task would benefit from checking stored memory first instead of guessing from the current chat alone.
---

# MemPalace Memory Retrieval

Use this skill when an AI agent has access to MemPalace through MCP and needs to recover remembered context before answering.

MemPalace is most valuable when the answer may already exist in stored conversations, project files, diary entries, or the knowledge graph. The point is not to sound confident. The point is to verify before speaking.

## When to use this skill

Use this skill proactively when the user asks about:

- past decisions, previous plans, earlier debugging sessions, or "what did we decide"
- project history, architecture rationale, migrations, regressions, or why something changed
- people, teams, relationships, ownership, preferences, biographies, or timeline-style questions
- facts that may have changed over time and should be checked instead of assumed
- a project-specific question where stored context likely matters more than generic codebase search

Examples:

- "What did we decide about auth last month?"
- "Why did we switch from GraphQL?"
- "What does Minh usually prefer for logging here?"
- "Have we already discussed this bug before?"
- "Remind me what happened in the last session on this project"

Do not use this skill for:

- pure code reading when the answer is clearly in the files in front of you
- library or API documentation lookup
- generic brainstorming that does not depend on prior memory
- making writes to MemPalace unless the user task actually requires updating memory

## Core protocol

Follow the MemPalace memory protocol that the MCP server exposes:

1. On wake-up, inspect the palace state with `mempalace_status`.
2. Before answering about any person, project, or past event, query memory first with `mempalace_search` or `mempalace_kg_query`.
3. If unsure about a fact, say you are checking and verify it.
4. After a meaningful session, write a short diary entry with `mempalace_diary_write` if the environment and task allow it.
5. When facts change, invalidate stale knowledge and add the new fact only if the user wants memory updated.

The governing principle is simple: wrong is worse than slow.

## Mental model

MemPalace stores memory as a palace:

- **Wings** are people or projects. Project ingestion commonly creates a wing from the directory name, slugified as `wing_<name>`.
- **Rooms** are topics within a wing, such as `auth`, `deploy`, or `graphql-switch`.
- **Drawers** hold verbatim content.
- **Search** returns the original text plus wing, room, similarity, and source file metadata.
- **Knowledge graph** stores entity relationships and timelines.
- **Diary** captures session-level notes for later recall.

Prefer using wings and rooms to narrow retrieval once you know the likely project or topic.

## Retrieval workflow

### 1. Orient yourself

Call `mempalace_status` when memory context matters and you do not already know what is stored. Use it to learn:

- whether the palace is populated
- what wings and rooms exist
- whether the knowledge graph is available
- the current memory protocol and AAAK reference

If the user's question is clearly about one project, also consider `mempalace_list_wings`, `mempalace_list_rooms`, or `mempalace_get_taxonomy` to find the best wing or room filter before searching broadly.

### 2. Pick the right retrieval tool

Use `mempalace_search` when you need verbatim evidence from stored content.

Use it for:

- prior conversations
- debugging history
- architectural rationale
- exact wording, decisions, and contextual nuance

Use `mempalace_kg_query` when the question is about entities and relationships.

Use it for:

- who works with whom
- who owns a project
- what role someone has
- factual relationships that may have validity windows

Use `mempalace_kg_timeline` when the user wants a chronological story of an entity.

Use `mempalace_diary_read` when the question is about recent sessions, what happened lately, or what the agent learned recently.

Use `mempalace_traverse` or `mempalace_find_tunnels` when the answer may span connected rooms or multiple wings.

### 3. Start broad, then narrow

A good default sequence is:

1. Run `mempalace_search` with the user's phrasing and a modest limit.
2. Inspect returned wings and rooms.
3. Re-run with `wing` and optionally `room` filters once the likely area is clear.
4. If the search result suggests specific people or entities, supplement with `mempalace_kg_query`.
5. If the answer depends on recent activity, supplement with `mempalace_diary_read`.

Avoid overcommitting to the first hit. Cross-check if the result looks ambiguous, stale, or only partially relevant.

## Practical tool guidance

### `mempalace_search`

Inputs:

- `query`: natural-language query
- `limit`: optional, defaults to a small number
- `wing`: optional wing filter
- `room`: optional room filter

Returns JSON text with:

- `results[]`
- `text`
- `wing`
- `room`
- `source_file`
- `similarity`
- `distance`

How to use it well:

- Start with the user's own terms.
- If results are noisy, rephrase using likely domain words from the first hit.
- If the user asks about one repo or person, filter by `wing`.
- If the user asks about one topic inside a repo, filter by both `wing` and `room`.
- Quote or summarize the retrieved evidence rather than inventing missing details.

### `mempalace_kg_query`

Use when the question is fundamentally relational rather than document-like.

Examples:

- "Who is working on Project Atlas?"
- "What do we know about Priya?"
- "When did Alex stop leading infra?"

If the answer sounds like a fact that could become outdated, explicitly mention that you checked memory and note any time bounds if the tool returns them.

### `mempalace_diary_read`

Use to recover recent sessions for a given agent or topic when the user asks for a recap.

This is especially useful for:

- "What happened last time?"
- "What did we learn in the previous session?"
- "Catch me up on recent work"

### `mempalace_list_wings`, `mempalace_list_rooms`, `mempalace_get_taxonomy`

Use these as navigation tools, not as the final answer.

They help when:

- you do not know the exact wing name
- you need to discover how a project was categorized
- the user's wording does not match stored room names yet

### `mempalace_traverse` and `mempalace_find_tunnels`

Use these when the user is asking for adjacent or cross-project context.

Examples:

- "What else is connected to auth-migration?"
- "What bridges the backend and infra wings?"

These tools are better for exploration than for quoting evidence. Pair them with `mempalace_search` when you need textual support.

## Answering after retrieval

When you answer, make the retrieved memory visible in your reasoning to the user.

Good pattern:

- briefly say you checked MemPalace
- cite the strongest retrieved evidence in plain language
- distinguish verified facts from your inference
- mention uncertainty if memory was sparse or conflicting

Example response shape:

1. "I checked MemPalace for prior auth discussions."
2. "The strongest matches were in `wing_myapp` / `auth-migration`, where the stored notes say the team switched because token refresh bugs kept recurring."
3. "I also found a diary entry from a later session confirming the migration was completed after the staging incident."
4. "So the verified answer is X; the part I'm inferring is Y."

If nothing useful is found, say that clearly. Do not pretend memory exists.

## Ambiguity and safety rules

- If multiple wings look plausible, ask a short clarification or search both and say so.
- If search and KG disagree, say they disagree and avoid flattening them into one answer.
- If memory may be stale, say "according to stored memory" and include the time context when possible.
- If the user asks for current code behavior, combine memory retrieval with code inspection rather than substituting one for the other.
- Never fabricate a remembered fact just because the user expects continuity.

## Writing back to memory

This skill is mainly about retrieval, but sometimes a session should update memory.

Use write tools only when appropriate:

- `mempalace_diary_write` after a meaningful work session, recap, or milestone
- `mempalace_kg_add` when the user wants a new stable fact remembered
- `mempalace_kg_invalidate` when an old fact is no longer true
- `mempalace_add_drawer` only when the task is explicitly about filing new source material into memory

Do not write by default during retrieval-only tasks.

## Examples

### Example 1: project decision

User: "What did we decide about GraphQL in this project?"

Suggested approach:

1. `mempalace_status`
2. `mempalace_search` with query like "GraphQL decision"
3. Narrow with the returned project wing if obvious
4. Answer with quoted evidence from the top hits

### Example 2: person fact

User: "Who owns infra now?"

Suggested approach:

1. `mempalace_kg_query` for the relevant entity or relationship
2. If needed, `mempalace_search` for corroborating discussion
3. Answer with the verified relationship and note any time qualifiers

### Example 3: session recap

User: "Can you remind me what we did last session on the Rust MCP server?"

Suggested approach:

1. `mempalace_diary_read`
2. `mempalace_search` for "MCP server" within the likely project wing
3. Answer with a concise recap grounded in the retrieved entries

## Output expectations

When this skill triggers, the agent should produce:

- a memory-aware answer grounded in retrieved evidence
- a brief note that memory was checked when that matters to trust
- explicit uncertainty if retrieval is sparse, stale, or conflicting
- follow-up questions only when the ambiguity blocks useful retrieval

The user should come away feeling that the agent remembered responsibly, not that it guessed confidently.
