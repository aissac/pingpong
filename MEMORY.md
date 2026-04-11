# DEE's Trading Context

## Current Project
Clone whale profile `0xe1d6b51521bd4365769199f392f9818661bd907` (81.8% win rate) into Jimm's Rust arb_bot on AWS.

## Partnership
- **Jimm** (@jim_09, 1047985696): owns AWS arb_bot (3.77.156.196, c6i.large Frankfurt)
- **DEE** (ElBardee, 1798631768): strategist/researcher — runs profile_monitor ClickHouse DB on AWS, researches whale via NotebookLM

## Bot Configuration (Jimm's AWS)
- Binary: `/home/ubuntu/polymarket-market-maker/target/release/arb_bot`
- DRY_RUN=true (paper trading)
- Log: `/tmp/paper_trade.log`
- Strategies enabled: PRE_ORDER, HIGH_SIDE, LOTTERY
- Strategies disabled: DUMP-HEDGE (whale never uses it)

## Whale Strategy (being cloned)
- 65% HIGH_SIDE (≥90¢, last 30s of 5M slot)
- 18% LOTTERY (1-3¢, throughout)
- 15% UNKNOWN
- 2% PRE_ORDER (both sides @ $0.45, pre-market)
- Position sizing: `capital × 0.5% × setup_score` (spread 30%, depth 25%, flow 25%, time 20%)

## Profile Monitor (DEE's AWS)
- ClickHouse DB with 2,200+ trades ingested
- profile_monitor service: systemd unit on AWS
- Data: `/home/ubuntu/trades_data/`
- Whale trades API: `https://data-api.polymarket.com/trades?user=0xe1d6b51521bd4365769199f392f9818661bd907`

## Workflow
DEE researches whale strategy via NotebookLM + monitors live via ClickHouse. Jimm implements into Rust arb_bot via OpenClaw SSH. Both run OpenClaw simultaneously on aissac.

## Memory Architecture (April 11, 2026 — streamlined)

| Layer | Tool | Best For | Search Type |
|-------|------|----------|-------------|
| **Active Memory** | (auto) | Surfacing relevant context before replies | Semantic (nomic) |
| **Memory Search** | `memory_search` | Curated knowledge (MEMORY.md + daily files) | Semantic (nomic) |
| **MemPalace** | `mcporter call mempalace mempalace_search` | Semantic search across 829 past sessions, similarity-ranked | Semantic (nomic) |
| **LCM** | `lcm_grep` / `lcm_expand_query` | Exact quotes, citations, conversation DAG | FTS + structured |
| **Graphify** | `mcporter call graphify query_graph` | Codebase architecture, code structure | Graph traversal |
| **Dreams** | (auto, 3am daily) | Promoting short-term recall → MEMORY.md | Weighted scoring |

**When to use which:**
- "What's our current strategy?" → Memory Search
- "What exact command was run on April 3?" → LCM (precise text, citations)
- "What did we discuss about whale patterns across all sessions?" → MemPalace (semantic, 829 sessions)
- "How does ghost_simulator.rs work?" → Graphify (code)
- Common recall → Active Memory handles automatically

**MemPalace** covers all 829 past sessions with semantic search (similarity-ranked). LCM covers this conversation's summaries with citation DAG. They're complementary, not redundant.

**Dreams config:**
- minScore=0.6, minRecallCount=2, minUniqueQueries=2
- Frequency: daily at 3am
- Promotes to MEMORY.md automatically

## polymarket Details
- WS subscription uses `asset_id` field (NOT token_id)
- Book events and price_changes arrays both need parsing
- Fee: 0.25 × variance² (peak 1.56% @ p=0.5, lower at extremes)
- Maker rebate: 20%

## OpenClaw Stack (April 11, 2026)
- **Version:** 2026.4.10
- **LCM:** Working, 181 summaries, gemma4:31b-cloud summary model
- **Active Memory:** Enabled, direct chats only, recent mode, balanced prompt
- **Dreams:** Enabled, 3am daily, minScore=0.6
- **Lessons learned:** Don't stop gateway before npm install finishes. Don't restart gateway while running on it.

## Useful Paths
- OpenClaw sessions: `~/.openclaw/agents/main/sessions/`
- OpenClaw logs: `/tmp/openclaw/openclaw-YYYY-MM-DD.log`
- AWS SSH: `ssh -i ~/.ssh/pingpong.pem ubuntu@3.77.156.196`