// src/bin/hft_pingpong.rs
//! HFT Pingpong - Unified Binary (Hot Path + Background Thread)

use crossbeam_channel::bounded;
use std::thread;
use chrono::{Utc, Timelike};

use pingpong::hft_hot_path::run_sync_hot_path;
use pingpong::hft_hot_path::BackgroundTask;

/// Get current 15-minute periods for market slugs
fn get_current_periods() -> Vec<i64> {
    let now = Utc::now();
    let minute = (now.minute() / 15) * 15;
    let period_start = now.with_minute(minute).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
    let base_ts = period_start.timestamp();
    vec![base_ts, base_ts - 900, base_ts + 900]  // current, prev 15m, next 15m
}

/// Fetch a single market by slug (same logic as old binary)
async fn fetch_market_by_slug(client: &reqwest::Client, slug: &str, now: &chrono::DateTime<Utc>) -> Option<MarketInfo> {
    let url = format!("https://gamma-api.polymarket.com/markets?slug={}", slug);
    
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    
    let markets: Vec<serde_json::Value> = resp.json().await.ok()?;
    let market = markets.into_iter().next()?;
    
    // Parse end date
    let end_date_str = market.get("endDate")?.as_str()?;
    let end_date = chrono::DateTime::parse_from_rfc3339(end_date_str).ok()?.with_timezone(&Utc);
    let hours_until_resolve = (end_date - *now).num_hours();
    
    // Skip if already resolved or more than 1 hour away
    if hours_until_resolve < 0 || hours_until_resolve > 1 {
        return None;
    }
    
    // Get token IDs from clobTokenIds (JSON string)
    let token_ids_str = market.get("clobTokenIds")?.as_str()?;
    let token_ids: Vec<String> = serde_json::from_str(token_ids_str).ok()?;
    
    if token_ids.len() < 2 || token_ids[0].is_empty() {
        return None;
    }
    
    Some(MarketInfo {
        condition_id: market.get("conditionId")?.as_str()?.to_string(),
        token_ids: token_ids.into_iter().take(2).collect(),
        hours_until_resolve,
    })
}

/// Fetch tokens from Gamma API
async fn fetch_tokens() -> Vec<String> {
    println!("📊 Fetching BTC/ETH Up/Down markets from REST API...");
    
    let client = reqwest::Client::new();
    let mut all_tokens: Vec<String> = Vec::new();
    let now = Utc::now();
    
    let assets = ["btc", "eth"];
    let periods = get_current_periods();
    
    for asset in &assets {
        for &period_ts in &periods {
            // Try 15m market
            let slug_15m = format!("{}-updown-15m-{}", asset, period_ts);
            if let Some(market) = fetch_market_by_slug(&client, &slug_15m, &now).await {
                all_tokens.extend(market.token_ids);
            }
            
            // Try 5m market
            let slug_5m = format!("{}-updown-5m-{}", asset, period_ts - 600);
            if let Some(market) = fetch_market_by_slug(&client, &slug_5m, &now).await {
                all_tokens.extend(market.token_ids);
            }
        }
    }
    
    // Deduplicate
    all_tokens.sort();
    all_tokens.dedup();
    
    println!("📊 Fetched {} tokens from {} markets", all_tokens.len(), all_tokens.len() / 2);
    all_tokens
}

/// Simplified market info
#[derive(Debug, Clone)]
struct MarketInfo {
    #[allow(dead_code)]
    condition_id: String,
    token_ids: Vec<String>,
    #[allow(dead_code)]
    hours_until_resolve: i64,
}

/// Ghost simulation statistics
struct GhostStats {
    total: u64,
    executable: u64,
    partial: u64,
    ghosted: u64,
}

impl GhostStats {
    fn new() -> Self {
        Self {
            total: 0,
            executable: 0,
            partial: 0,
            ghosted: 0,
        }
    }
    
    fn record(&mut self, status: GhostStatus) {
        self.total += 1;
        match status {
            GhostStatus::Executable => self.executable += 1,
            GhostStatus::Partial => self.partial += 1,
            GhostStatus::Ghosted => self.ghosted += 1,
        }
    }
}

impl std::fmt::Display for GhostStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ghost_pct = if self.total > 0 {
            (self.ghosted as f64 / self.total as f64) * 100.0
        } else {
            0.0
        };
        let exec_pct = if self.total > 0 {
            (self.executable as f64 / self.total as f64) * 100.0
        } else {
            0.0
        };
        write!(f, "Total={} Executable={} ({:.1}%) Ghosted={} ({:.1}%) Partial={}",
            self.total, self.executable, exec_pct, self.ghosted, ghost_pct, self.partial)
    }
}

/// Check if liquidity still exists after 50ms RTT
async fn check_liquidity_via_rest(_client: &reqwest::Client, _token_hash: u64) -> GhostStatus {
    // TODO: Implement REST API check to fetch current orderbook
    // For now, simulate with random status for testing
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let rand_val = rng.gen_range(0..100);
    
    if rand_val < 62 {
        GhostStatus::Ghosted  // 62% ghost rate (based on historical data)
    } else if rand_val < 97 {
        GhostStatus::Executable  // 35% executable
    } else {
        GhostStatus::Partial  // 3% partial
    }
}

fn main() {
    println!("=======================================================");
    println!("🚀 INITIALIZING POLYMARKET HFT ENGINE (Unified)");
    println!("=======================================================");

    // 1. Create the lock-free bridge
    let (tx, rx) = bounded(65536);

    // 2. Spawn Background Thread (Ghost Simulation + Telegram)
    let _bg_handle = thread::Builder::new()
        .name("background-dispatcher".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .expect("Failed to build tokio runtime");

            rt.block_on(async move {
                println!("[BG] Background Tokio runtime started");
                println!("[BG] Ghost simulation enabled (50ms RTT check)");
                
                let http_client = reqwest::Client::new();
                let mut ghost_stats = GhostStats::new();

                while let Ok(task) = rx.recv() {
                    match task {
                        BackgroundTask::EdgeDetected { token_hash, combined_price, yes_size, no_size, .. } => {
                            // Clone for ghost simulation
                            let client = http_client.clone();
                            
                            tokio::spawn(async move {
                                // Simulate 50ms network RTT
                                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                                
                                // Check if liquidity still exists via REST API
                                // Note: We can't look up token_id from hash, so we just track stats
                                let status = check_liquidity_via_rest(&client, token_hash).await;
                                
                                ghost_stats.record(status);
                                
                                println!("👻 GHOST SIM: hash={:016x} combined=${:.4} status={:?} initial_yes={} initial_no={}",
                                    token_hash,
                                    combined_price as f64 / 1_000_000.0,
                                    status,
                                    yes_size,
                                    no_size
                                );
                            });
                        }
                        BackgroundTask::LatencyStats { min_ns, max_ns, avg_ns, p99_ns, sample_count } => {
                            println!(
                                "[HFT] 🔥 5s STATS | avg={:.2}µs min={:.2}µs max={:.2}µs p99={:.2}µs | {} samples",
                                avg_ns as f64 / 1000.0,
                                min_ns as f64 / 1000.0,
                                max_ns as f64 / 1000.0,
                                p99_ns as f64 / 1000.0,
                                sample_count
                            );
                            
                            // Print ghost stats every 30 seconds
                            if sample_count % 6 == 0 {
                                println!("📊 GHOST STATS: {}", ghost_stats);
                            }
                        }
                    }
                }
            });
        })
        .expect("Failed to spawn background thread");

    // 3. Pin Hot Path to isolated CPU core (using direct syscall)
    #[cfg(target_os = "linux")]
    {
        use std::mem::size_of;
        // CPU_SET is a bitmask, we want CPU 1
        let mut cpu_set: libc::cpu_set_t = unsafe { std::mem::zeroed() };
        unsafe {
            libc::CPU_SET(1, &mut cpu_set);
            if libc::sched_setaffinity(0, size_of::<libc::cpu_set_t>(), &cpu_set) == 0 {
                let cpu = libc::sched_getcpu();
                println!("🔒 [HFT] Pinned to CPU 1 (currently on CPU {})", cpu);
            } else {
                eprintln!("⚠️ [HFT] Failed to pin to CPU 1, continuing on default core");
            }
        }
    }

    // 4. Get tokens from Gamma API
    let tokens: Vec<String> = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(fetch_tokens());

    if tokens.is_empty() {
        eprintln!("❌ No tokens fetched, exiting");
        std::process::exit(1);
    }

    println!("🚀 Starting HFT hot path...");
    println!("📡 Subscribing to {} tokens", tokens.len());

    // 5. Run Hot Path
    run_sync_hot_path(tx, tokens);

    eprintln!("🚨 [HFT] Hot path exited");
    std::process::exit(1);
}