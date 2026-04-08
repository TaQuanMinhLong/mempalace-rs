# MemPalace

### The highest-scoring AI memory system ever benchmarked. And it's free.

MemPalace is a local-first memory palace system for AI agents. Store verbatim conversations and code context, then search it semantically. No cloud. No subscription. No API key required.

**This is the Rust port** of mempalace — faster, leaner, and runs entirely offline.

---

## Quick Start

```bash
# Build from source
cargo build --release
./target/release/mempalace init ~/projects/myapp

# Mine your data
./target/release/mempalace mine ~/projects/myapp                    # project files
./target/release/mempalace mine ~/chats/ --mode convos            # conversation exports
./target/release/mempalace mine ~/chats/ --mode convos --extract general  # with extraction

# Search anything you've ever discussed
./target/release/mempalace search "why did we switch to GraphQL"

# Check your palace
./target/release/mempalace status
```

---

## Why MemPalace?

Every conversation with an AI — every decision, every debugging session, every architecture debate — disappears when the session ends. Six months of work, gone.

**MemPalace takes a different approach: store everything, then make it findable.**

- **Raw verbatim storage** — Store your actual exchanges without summarization. The 96.6% LongMemEval result comes from raw mode.
- **The Palace structure** — Organize into wings (people/projects), rooms (topics), and drawers (content). Semantic search + structure = 34% better retrieval.
- **Local and free** — Everything runs on your machine. No data leaves your computer.
- **MCP server** — 19 tools for AI integration with Claude, Cursor, Gemini, and any MCP-compatible client.

---

## The Palace

Ancient Greek orators memorized speeches by placing ideas in rooms of a building. Walk through the building, find the idea. MemPalace applies the same principle to AI memory.

```
┌─────────────────────────────────────────────────────────────┐
│  WING: Person                                               │
│                                                             │
│    ┌──────────┐  ──hall──  ┌──────────┐                     │
│    │  Room A  │            │  Room B  │                     │
│    └────┬─────┘            └──────────┘                     │
│         │                                                   │
│         ▼                                                   │
│    ┌──────────┐      ┌──────────┐                           │
│    │  Closet  │ ───▶ │  Drawer  │                           │
│    └──────────┘      └──────────┘                           │
└─────────┼───────────────────────────────────────────────────┘
          │
        tunnel
          │
┌─────────┼───────────────────────────────────────────────────┐
│  WING: Project                                              │
│         │                                                   │
│    ┌────┴─────┐  ──hall──  ┌──────────┐                     │
│    │  Room A  │            │  Room C  │                     │
│    └────┬─────┘            └──────────┘                     │
│         │                                                   │
│         ▼                                                   │
│    ┌──────────┐      ┌──────────┐                           │
│    │  Closet  │ ───▶ │  Drawer  │                           │
│    └──────────┘      └──────────┘                           │
└─────────────────────────────────────────────────────────────┘
```

**Wings** — A person or project. As many as you need.
**Rooms** — Specific topics within a wing. Auth, billing, deploy — endless rooms.
**Halls** — Connections between related rooms within the same wing.
**Tunnels** — Connections between rooms across different wings.
**Drawers** — The original verbatim content.

### Memory Stack

| Layer  | What                                            | Size        | When             |
| ------ | ----------------------------------------------- | ----------- | ---------------- |
| **L0** | Identity — who is this AI?                      | ~50 tokens  | Always           |
| **L1** | Critical facts — team, projects, preferences    | ~120 tokens | Always           |
| **L2** | Room recall — recent sessions                   | On demand   | Topic comes up   |
| **L3** | Deep search — semantic query across all drawers | On demand   | Explicitly asked |

---

## Installation

### From Source

```bash
git clone https://github.com/milla-jovovich/mempalace
cd mempalace
just install
```

This will build the release binary and install it to `~/.local/bin/mempalace`.

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- [`just`](https://github.com/casey/just) (command runner, install via `cargo install just`)
- No external dependencies — everything is embedded

---

## All Commands

```bash
# Setup
mempalace init ~/.mempalace                              # Initialize palace

# Mining
mempalace mine <dir>                                     # Mine project files
mempalace mine <dir> --mode convos                       # Mine conversations
mempalace mine <dir> --mode convos --extract general      # With entity extraction

# Splitting (for mega transcript files)
mempalace split <dir>                                    # Split into per-session files

# Search
mempalace search "query"                                  # Search everything
mempalace search "query" --wing myapp                    # Within a wing
mempalace search "query" --room auth                     # Within a room

# Memory stack
mempalace wake-up                                        # Load L0 + L1 context
mempalace wake-up --wing myproject                       # Project-specific

# Compression
mempalace compress --wing myapp                          # AAAK compression

# Status
mempalace status                                         # Palace overview
mempalace repair                                         # Repair/rebuild index

# MCP Server
mempalace serve                                          # Start MCP server (JSON-RPC over stdio)
```

Or use `just` for common tasks:

```bash
just build              # Build release binary
just install            # Install to ~/.local/bin
just test              # Run tests
just mine              # Mine current directory
just search "query"    # Search
just status            # Check status
```

---

## MCP Server

Connect MemPalace to your AI:

```bash
# Add to Claude/Cursor/etc via MCP
claude mcp add mempalace -- just serve
```

Or after installing:

```bash
claude mcp add mempalace -- mempalace serve
```

### 19 Tools

**Palace (read)**

| Tool                        | What                                          |
| --------------------------- | --------------------------------------------- |
| `mempalace_status`          | Palace overview + AAAK spec + memory protocol |
| `mempalace_list_wings`      | Wings with drawer counts                      |
| `mempalace_list_rooms`      | Rooms within a wing                           |
| `mempalace_get_taxonomy`    | Full wing → room → count tree                 |
| `mempalace_search`          | Semantic search with wing/room filters        |
| `mempalace_check_duplicate` | Check before filing                           |
| `mempalace_get_aaak_spec`   | AAAK dialect reference                        |

**Palace (write)**

| Tool                      | What                  |
| ------------------------- | --------------------- |
| `mempalace_add_drawer`    | File verbatim content |
| `mempalace_delete_drawer` | Remove by ID          |

**Knowledge Graph**

| Tool                      | What                                     |
| ------------------------- | ---------------------------------------- |
| `mempalace_kg_query`      | Entity relationships with time filtering |
| `mempalace_kg_add`        | Add facts                                |
| `mempalace_kg_invalidate` | Mark facts as ended                      |
| `mempalace_kg_timeline`   | Chronological entity story               |
| `mempalace_kg_stats`      | Graph overview                           |

**Navigation**

| Tool                     | What                                    |
| ------------------------ | --------------------------------------- |
| `mempalace_traverse`     | Walk the graph from a room across wings |
| `mempalace_find_tunnels` | Find rooms bridging two wings           |
| `mempalace_graph_stats`  | Graph connectivity overview             |

**Agent Diary**

| Tool                    | What                      |
| ----------------------- | ------------------------- |
| `mempalace_diary_write` | Write AAAK diary entry    |
| `mempalace_diary_read`  | Read recent diary entries |

---

## Architecture

### Storage

- **Vector storage** — LanceDB (embedded, file-based). No server needed.
- **Knowledge graph** — SQLite with temporal entity-relationship triples.
- **Config** — JSON files in `~/.mempalace/`

### Project Structure

```
src/
├── main.rs              # CLI entry point
├── lib.rs               # Library root
├── config.rs            # Configuration management
├── error.rs             # Error types
├── palace/
│   ├── drawer.rs       # Drawer data model
│   ├── wing.rs         # Wing model
│   └── room.rs         # Room model
├── storage/
│   ├── lancedb.rs      # LanceDB vector storage
│   └── sqlite_kg.rs    # SQLite knowledge graph
├── miner/
│   ├── file_miner.rs   # Project file ingestion
│   ├── convo_miner.rs  # Conversation mining
│   └── splitter.rs      # Mega file splitting
├── search/
│   ├── semantic.rs      # Semantic search
│   └── retrieval.rs     # Layer-based retrieval
├── graph/
│   ├── knowledge.rs    # Knowledge graph operations
│   └── palace_graph.rs # Room navigation
├── extract/
│   ├── entity.rs       # Entity detection
│   ├── room.rs         # Room detection
│   └── general.rs      # Memory extraction
├── dialect/
│   └── aaak.rs        # AAAK compression
├── normalize/
│   └── parser.rs       # Multi-format chat parser
├── layers.rs           # 4-layer memory stack
└── mcp/
    └── server.rs       # MCP server
```

---

## AAAK Dialect (experimental)

AAAK (Adversarial Aggregated Knowledge) is a compressed memory dialect for packing repeated entities into fewer tokens at scale.

**Format:**

- Entity codes: `ALC` = Alice, `JOR` = Jordan, `KAI` = Kai
- Emotion markers: `*warm*`, `*fierce*`, `*raw*`, `*bloom*`
- Pipe-separated fields: `FAM:` family | `PROJ:` projects | `WARNING:` warnings

**Example:**

```
FAM: ALC→HEART Jordan | 2D(kids): RIL(18,sports) MAX(11,chess+swimming)
PROJ: ORION→backend→GraphQL migration | DRIFTWOOD→analytics→P99<50ms
WARNING: ALC→sensitive about deadlines | KAI→prefers async
```

AAAK is **readable by any LLM** — Claude, GPT, Gemini, Llama, Mistral. No decoder needed.

---

## Benchmarks

| Benchmark               | Mode                | Score         | API Calls |
| ----------------------- | ------------------- | ------------- | --------- |
| **LongMemEval R@5**     | Raw (verbatim)      | **96.6%**     | Zero      |
| **LongMemEval R@5**     | Raw + rerank        | **100%**      | ~500      |
| Palace structure impact | Wing+room filtering | **+34%** R@10 | Zero      |

The 96.6% raw score is the highest published LongMemEval result requiring no API key, no cloud, and no LLM at query time.

---

## Configuration

### Global (`~/.mempalace/config.json`)

```json
{
  "palace_path": "~/.mempalace/palace",
  "collection_name": "mempalace_drawers",
  "knowledge_graph_path": "~/.mempalace/knowledge_graph.sqlite3"
}
```

### Wing config (`~/.mempalace/wing_config.json`)

```json
{
  "default_wing": "wing_general",
  "wings": {
    "wing_kai": { "type": "person", "keywords": ["kai", "kai's"] },
    "wing_orion": { "type": "project", "keywords": ["orion", "backend"] }
  }
}
```

### Identity (`~/.mempalace/identity.txt`)

Plain text. Becomes Layer 0 — loaded every session.

---

## Requirements

- Rust 1.70+
- No external dependencies
- No API key
- No internet after install

---

## License

MIT — see [LICENSE](LICENSE).

---

## Credits

MemPalace was created by [Milla Jovovich](https://github.com/milla-jovovich) and [Ben Sigman](https://github.com/sigmaxipi).

This Rust port is a faithful recreation of the Python version with the same architecture and features.
