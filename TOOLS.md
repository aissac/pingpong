# TOOLS.md - Local Notes

Skills define _how_ tools work. This file is for _your_ specifics — the stuff that's unique to your setup.

## 🏓 Token Efficiency Stack (April 9, 2026)

**Priority order when answering code questions:**
1. **Graphify** → `query_graph` (BFS=overview, DFS=trace flow), `shortest_path` (A→B), `get_community` (strategy group), `get_neighbors` (dependencies), `god_nodes` (core abstractions)
2. **jcodemunch** → `mcporter call jcodemunch <tool>` for refactoring, smell detection, dead code, dependency analysis, docs generation
3. **RTK** → manually prepend `rtk` to expensive shell commands: `rtk git status`, `rtk cargo build`, `rtk ls`, `rtk grep`
4. **grep/rg** → last resort for file contents, not architecture navigation

**RTK usage:**
```bash
rtk git status     # ~200 tokens vs ~2000 raw
rtk git diff      # condensed vs noisy
rtk git log -n 10 # one-liner commits
rtk cargo build   # filtered output
rtk cargo test    # failures only
rtk ls            # compact tree
rtk rtk gain      # show savings so far
```

**jcodemunch tools (50 available):** refactor_ask, find_dead_code, analyze_complexity, suggest_tests, explain_code, generate_tests, code_suggestions, dependency_relationship, analyze_file_health, generate_docs, etc.

**RTK hook installed but not wired to OpenClaw exec.** Bash hook works for Claude Code only. RTK v0.35.0 at `~/.local/bin/rtk`.

---

## NotebookLM Integration

**Tool:** `notebooklm-py` (Python library + CLI)
**Purpose:** Research automation, ask questions about sources, generate content

**Setup:**
```bash
pip install notebooklm-py
export PATH="$HOME/.local/bin:$PATH"  # for CLI
```

**Usage:**
```bash
notebooklm login                    # Authenticate first (opens browser)
notebooklm list                     # List notebooks
notebooklm use <notebook_id>        # Set active notebook
notebooklm ask "question"           # Ask about notebook sources
notebooklm source list              # List sources in notebook
notebooklm artifact list            # List generated artifacts
```

**Notebooks:**
- Strategic Foundations for Polymarket and Algorithmic Trading: `857bce48-57e6-4ee7-a621-6fe8a588a239`

**Key pattern from NotebookLM research:**
- Track YES and NO prices SEPARATELY with `Option<f64>`
- Only compute combined cost when BOTH exist
- Reset to None on reconnect to avoid stale prices

---

## Recall Stack (April 11, 2026 — 4 layers, streamlined)

### Wake-up Protocol
On every session start, verify recall stack is working:
1. `memory_search` — check it returns relevant results
2. `lcm_grep` — verify LCM DB is accessible (181 summaries, working)
3. `mcporter call mempalace mempalace_status` — verify palace is responsive (829 drawers)
4. `mcporter call graphify query_graph question="test"` — verify graphify is live
5. Active Memory auto-runs before replies — no manual check needed

### The Stack
1. **Memory Search** (`memory_search`) — curated long-term memory (MEMORY.md + daily files). nomic-embed-text, 768 dims, 472 chunks. Primary recall for "what do I know about X"
2. **LCM** (`lcm_grep` / `lcm_expand_query`) — exact conversation recall, compressed summaries with citation DAG. 181 summaries. Use for "what exact command was run" or verifying precise claims
3. **MemPalace** (`mcporter call mempalace mempalace_search`) — 829 sessions indexed, semantic search with similarity scores, knowledge graph. Use for conceptually related discussions across all past sessions
4. **Active Memory** — auto-injects relevant memory context before each reply. Uses memory_search + memory_get. No manual action needed
5. **Graphify** (`mcporter call graphify ...`) — codebase architecture. 1439 nodes, 123 communities
   - `query_graph question=X` — BFS overview (default)
   - `query_graph question=X mode=dfs` — trace execution flow
   - `shortest_path source=A target=B` — path between concepts
   - `get_community community_id=N` — strategy group (e.g., Community 1 = all strategies)
   - `get_neighbors label=X relation_filter=calls` — what depends on X
   - `get_node label=X` — full detail on struct/function
   - `god_nodes top_n=5` — core abstractions

### Supporting Systems
- **Dreams** — auto-promotes short-term recall → MEMORY.md daily at 3am. Thresholds: minScore=0.6, minRecallCount=2, minUniqueQueries=2
- **MEMORY.md** — curated knowledge, updated manually + by Dreams auto-promotion

### Never Guess — Always Check First
Before answering "how does X work" or "what did we decide about Y":
- Curated knowledge → memory_search
- Exact quotes/citations → LCM
- Semantic session search → MemPalace
- Code questions → Graphify
- Active Memory handles common recall automatically
- If unsure, check. Wrong is worse than slow.

### Key Commands
- `memory_search query="AWS downtime"` — curated long-term memory
- `lcm_grep query="whale strategy"` — exact text search in conversation history
- `lcm_expand_query summaryIds=["sum_xxx"] prompt="What strategy was decided?"` — deep recall with citations
- `mcporter call mempalace mempalace_search query="whale strategy"` — semantic search across 829 sessions
- `mcporter call graphify query_graph question="how does X work"` — codebase architecture

### Killed / Removed
- ~~Short-term Recall~~ — was broken, replaced by Dreams
- ~~Wiki~~ — never built; Graphify's GRAPH_REPORT.md replaces it for code