# Latency Benchmarks

**Current Status:** 0.7µs average (elite HFT tier)

## Latency Journey

| Stage | Latency | Improvement | Notes |
|-------|---------|-------------|-------|
| **Original (Python/TS)** | 50ms | Baseline | Pre-Rust rewrite |
| **After Rust rewrite** | 50µs | 1,000× | Basic Rust implementation |
| **After SIMD JSON** | 9.38µs | 5,300× | simd-json crate |
| **After serde_json stable** | 0.7µs | 71,000× | Final optimization |

## Current Performance (March 29, 2026)

```
Average:  0.7µs
Minimum:  0.21µs
p99:      1-5µs
Samples:  Continuous
```

**Comparison:**
- **polyfill-rs benchmark:** 0.28-7.70µs
- **Pingpong:** 0.7µs average ✅ (within elite range)

## Key Optimizations Implemented

### 1. SIMD JSON Parsing
```toml
[dependencies]
simd-json = "0.13"  # 1.77× faster than serde_json
```
**Impact:** Reduced JSON parse time from ~8µs to ~4.5µs

### 2. Lock-Free Channels
```toml
[dependencies]
crossbeam-channel = "0.5"  # SPSC queues, no mutex
```
**Impact:** Eliminated contention between WebSocket thread and strategy thread

### 3. Core Affinity (CPU Pinning)
```toml
[dependencies]
core_affinity = "0.8"
```
**Config:**
```bash
# GRUB config (needs reboot)
isolcpus=1 nohz_full=1 rcu_nocbs=1
```
**Impact:** Eliminated 60µs p99 spikes from context switches

### 4. Thread-Local Orderbook
```rust
// NO DashMap, NO Arc<Mutex<>>
thread_local! {
    static ORDERBOOK: RefCell<LocalOrderBook> = ...
}
```
**Impact:** Zero synchronization overhead in hot path

### 5. Sync WebSocket (No Tokio)
```rust
// HFT binary uses sync tungstenite
use tungstenite::connect;
// Busy-poll loop with CPU core pinning
```
**Impact:** Eliminated async runtime overhead (~2-5µs per message)

### 6. Fixed-Point Math
```rust
// NO f64 in hot path
type PriceU64 = u64;  // Price × 10^8
```
**Impact:** Eliminated float conversion overhead

### 7. Zero-Allocation Token Parsing
```rust
// Dynamic-length token IDs (66-78 chars)
let end = memchr(b'"', slice).unwrap_or(slice.len());
let token_id = std::str::from_utf8(&slice[..end])?;
```
**Impact:** Eliminated string allocation per message

## Measurement Code

```rust
// TSC timing (nanosecond precision)
let start = minstant::now();
// ... process message ...
let elapsed_ns = minstant::now() - start;

// Batch stats every 5 seconds
let avg_ns = total_ns / sample_count;
let p99_ns = sorted_samples[(samples.len() as f64 * 0.99) as usize];

println!("[HFT] 🔥 avg={:.2}µs min={:.2}µs max={:.2}µs p99={:.2}µs", 
    avg_ns as f64 / 1000.0, 
    min_ns as f64 / 1000.0, 
    max_ns as f64 / 1000.0, 
    p99_ns as f64 / 1000.0);
```

## Network RTT

| Location | RTT | Notes |
|----------|-----|-------|
| **AWS Frankfurt (c6i.large)** | ~1ms | Colocated with Polymarket |
| **Home (NYC)** | ~80ms | Transatlantic |
| **VPS (NYC)** | ~20ms | Domestic US |

**Key insight:** Processing latency (0.7µs) is now negligible vs network RTT (1ms).
**Bottleneck shifted:** CPU → Network

## Ghost Rate Impact

**50ms RTT simulation (March 28):**
- **62% ghosted:** Liquidity vanished before order could fill
- **35% executable:** Real opportunities
- **3% partial:** Some liquidity remained

**With 1ms RTT (Frankfurt):**
- Expected ghost rate: <10% (proportional to RTT)
- **50× improvement** in fill rate vs home connection

## Files Modified

| File | Optimization | Commit |
|------|--------------|--------|
| `src/hft_hot_path.rs` | Sub-microsecond hot path | `46a5d87` |
| `src/bin/hft_pingpong.rs` | TSC timing, batch stats | `276adbc` |
| `Cargo.toml` | crossbeam, minstant, core_affinity | `276adbc` |
| `/etc/default/grub` | Core isolation params | `6aa68e8` |

## Path to Sub-100ns

**Remaining optimizations:**
1. ✅ SIMD JSON (done)
2. ✅ Thread-local orderbook (done)
3. ✅ Sync WebSocket (done in HFT binary)
4. ⏳ Zero-allocation token_id hashing
5. ⏳ Reduce logging overhead in hot path

**Theoretical minimum:** ~200-300ns (polyfill-rs lower bound)

---

## Sources

- MEMORY.md (2026-03-28 to 2026-03-29 sessions)
- `src/hft_hot_path.rs` (latency measurement code)
- NotebookLM: polyfill-rs benchmark comparison
- GitHub commits `276adbc`, `46a5d87`, `6aa68e8`
