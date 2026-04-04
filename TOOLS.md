# TOOLS.md - Local Notes

Skills define _how_ tools work. This file is for _your_ specifics — the stuff that's unique to your setup.

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
