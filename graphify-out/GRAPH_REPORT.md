# Graph Report - /home/aissac/.openclaw/workspace  (2026-04-11)

## Corpus Check
- 172 files · ~142,733 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1439 nodes · 2056 edges · 123 communities detected
- Extraction: 100% EXTRACTED · 0% INFERRED · 0% AMBIGUOUS
- Token cost: 0 input · 0 output

## God Nodes (most connected - your core abstractions)
1. `WsEngine` - 24 edges
2. `InventoryTracker` - 19 edges
3. `ClobClient` - 14 edges
4. `QuoteManager` - 13 edges
5. `RateLimiter` - 13 edges
6. `DualExecutor` - 12 edges
7. `main()` - 12 edges
8. `MarketState` - 12 edges
9. `DumpHedgeEngine` - 12 edges
10. `FeeCalculator` - 12 edges

## Surprising Connections (you probably didn't know these)
- `main()` --calls--> `run_paper_trading_loop()`  [EXTRACTED]
  src/main.rs → main.rs
- `main()` --calls--> `run_paper_loop()`  [EXTRACTED]
  src/main.rs → clobguard/src/main.rs

## Communities

### Community 0 - "Community 0"
Cohesion: 0.05
Nodes (32): build_l2_headers(), create_signed_order(), execute_maker_order(), execute_taker_order(), fetch_fee_rate(), submit_order_with_backoff(), test_header_construction(), build_l2_headers() (+24 more)

### Community 1 - "Community 1"
Cohesion: 0.04
Nodes (17): DumpHedgeConfig, DumpHedgeEngine, DumpPosition, PriceHistory, HighSideConfig, HighSideEngine, MomentumPosition, PricePoint (+9 more)

### Community 2 - "Community 2"
Cohesion: 0.06
Nodes (22): BackgroundTask, BackgroundTask, BackgroundTask, BackgroundTask, RolloverCommand, BackgroundTask, BackgroundTask, RolloverCommand (+14 more)

### Community 3 - "Community 3"
Cohesion: 0.08
Nodes (18): InventorySummary, InventoryTracker, MarketInventory, MarketPosition, MarketTracker, Position, setup_tracker(), Side (+10 more)

### Community 4 - "Community 4"
Cohesion: 0.08
Nodes (14): ConnectionState, fast_hash(), generate_ws_key(), OrderbookLevel, run_ws_engine(), spawn_ws_engine(), test_fast_hash_consistency(), test_orderbook_state_insert_get() (+6 more)

### Community 5 - "Community 5"
Cohesion: 0.09
Nodes (16): ApiError, base64_decode(), base64_encode(), ClobClient, FeeRateResponse, hmac_sha256(), Orderbook, OrderRequest (+8 more)

### Community 6 - "Community 6"
Cohesion: 0.09
Nodes (11): ArbOpportunity, EdgeDetector, EdgeDetectorConfig, OrderbookSnapshot, price_to_cents(), size_to_micro(), test_combined_ask(), test_depth_walk_single_level() (+3 more)

### Community 7 - "Community 7"
Cohesion: 0.09
Nodes (18): BotState, calculate_position_size(), calculate_setup_score(), check_mid_market(), discover_markets(), get_15m_slot(), get_5m_slot(), main() (+10 more)

### Community 8 - "Community 8"
Cohesion: 0.1
Nodes (13): CircuitBreaker, CircuitState, InventoryCircuitBreaker, LossTracker, PriceSnapshot, test_circuit_breaker_emergency_stop(), test_circuit_breaker_volatility(), test_inventory_circuit_breaker() (+5 more)

### Community 9 - "Community 9"
Cohesion: 0.11
Nodes (9): MarketPrices, MarketState, MarketStats, test_clear_on_reconnect(), test_market_prices(), test_market_state(), test_no_edge(), test_token_pair_update() (+1 more)

### Community 10 - "Community 10"
Cohesion: 0.1
Nodes (18): BackgroundTask, calc_crypto_fee_micro_usdc(), EngineEvent, HotPath, OrderAction, process_orderbook_tick(), run_hot_path(), spawn_hot_path() (+10 more)

### Community 11 - "Community 11"
Cohesion: 0.14
Nodes (17): make_inventory(), MarketState, Quote, QuoteConfig, QuoteManager, QuoteSide, test_aggressive_clear_long_yes(), test_aggressive_clear_short_yes() (+9 more)

### Community 12 - "Community 12"
Cohesion: 0.17
Nodes (11): ArbCircuitBreaker, ArbRiskLimits, ArbStatus, DailyStats, test_can_execute_edge_too_low(), test_can_execute_spread_too_wide(), test_can_execute_success(), test_consecutive_failures() (+3 more)

### Community 13 - "Community 13"
Cohesion: 0.13
Nodes (10): AsyncBucketState, global_rate_limiter(), RateLimiter, SyncTokenBucket, test_429_backoff(), test_rate_limiter(), test_rate_limiter_basic(), test_rate_limiter_refill() (+2 more)

### Community 14 - "Community 14"
Cohesion: 0.15
Nodes (9): DualExecutor, ExecutionResult, ExecutionStats, ExecutorConfig, LegOrder, OrderStatus, test_dry_run_execution(), test_position_sizing() (+1 more)

### Community 15 - "Community 15"
Cohesion: 0.13
Nodes (10): BuilderCreds, L2Auth, RelayerClient, RelayerError, RelayerRateLimiter, RelayerRequest, RelayerResponse, test_builder_creds_from_env_missing() (+2 more)

### Community 16 - "Community 16"
Cohesion: 0.13
Nodes (13): ArbPhase, calculate_edge_after_fees(), calculate_fee_adjusted_cost(), DumpDetector, DumpInfo, FeeAdjustedOpportunity, FeeAwareEdgeDetector, is_viable_after_fees() (+5 more)

### Community 17 - "Community 17"
Cohesion: 0.14
Nodes (11): ActiveQuote, MarketInventory, MarketMakerState, QuoteManager, Side, test_inventory_skew_calculation(), test_merge_detection(), test_quote_calculation_no_skew() (+3 more)

### Community 18 - "Community 18"
Cohesion: 0.13
Nodes (10): ClobOrderRequest, ClobOrderResponse, ClobOrderStatus, DailyStats, ExecutorConfig, OrderExecutor, OrderResult, PendingOrder (+2 more)

### Community 19 - "Community 19"
Cohesion: 0.16
Nodes (10): FastOrderBook, from_fixed(), LatencyStats, log_stats_periodically(), PolymarketEvent, start_trading_thread(), test_fixed_point(), test_orderbook_update() (+2 more)

### Community 20 - "Community 20"
Cohesion: 0.13
Nodes (8): CopyTradeAction, CopyTraderEngine, PendingCopyTrade, PositionState, test_copy_trade_action_creation(), test_price_adjustment_buy(), test_price_adjustment_sell(), WhaleTrade

### Community 21 - "Community 21"
Cohesion: 0.19
Nodes (8): ArbMonitor, MarketPair, OrderbookState, PriceLevel, test_orderbook_update(), test_register_market(), test_stale_detection(), WsBookMessage

### Community 22 - "Community 22"
Cohesion: 0.19
Nodes (6): FeeCalculator, MarketType, test_adjusted_cost(), test_crypto_vs_sports_fee(), test_decimal_vs_cents(), test_market_type_detection()

### Community 23 - "Community 23"
Cohesion: 0.18
Nodes (16): BotState, calc_fee(), check_position_exits(), check_strategies(), create_position(), discover_updown(), get_duration(), main() (+8 more)

### Community 24 - "Community 24"
Cohesion: 0.12
Nodes (7): ClobOrder, FakOrderHelper, OrderResult, OrderStatus, OrderType, test_fak_buy_order(), test_fak_partial_fill()

### Community 25 - "Community 25"
Cohesion: 0.2
Nodes (5): FeeCalculator, MarketType, test_camel_case_economics_fee(), test_crypto_vs_sports_fee(), test_market_type_detection()

### Community 26 - "Community 26"
Cohesion: 0.12
Nodes (4): fast_hash(), test_hash_token_consistency(), MakerLeg, TakerLeg

### Community 27 - "Community 27"
Cohesion: 0.15
Nodes (6): LotteryConfig, LotteryEngine, LotteryPosition, LotteryTakeProfit, PriceSnapshot, test_share_calculation()

### Community 28 - "Community 28"
Cohesion: 0.17
Nodes (7): calc_fee(), MergeInfo, PreOrderConfig, PreOrderEngine, PreOrderPosition, test_pre_order_timing_5m(), test_pre_order_timing_too_early()

### Community 29 - "Community 29"
Cohesion: 0.21
Nodes (8): BackgroundTask, EvalTracker, fast_hash(), parse_and_update_orderbook(), parse_fixed_6(), RolloverCommand, run_sync_hot_path(), TokenBookState

### Community 30 - "Community 30"
Cohesion: 0.24
Nodes (8): ClobSigner, SignedOrder, SignerMode, SignerModeMetadata, test_build_order(), test_gnosis_safe_mode(), test_sign_order(), test_signed_order_debug()

### Community 31 - "Community 31"
Cohesion: 0.19
Nodes (7): ActivePosition, DumpAndHedgeEngine, DumpDetection, LegPhase, test_dump_detection(), test_hedge_threshold(), test_timeout()

### Community 32 - "Community 32"
Cohesion: 0.16
Nodes (5): BinanceOracle, BinanceState, BookTickerMsg, DislocationEvent, is_binance_fresh()

### Community 33 - "Community 33"
Cohesion: 0.21
Nodes (6): FeeOracle, test_adjusted_cost(), test_is_profitable(), test_max_combined_for_edge(), test_position_sizing(), test_profit_after_fees()

### Community 34 - "Community 34"
Cohesion: 0.17
Nodes (5): ExecutionState, fast_hash(), HedgeContext, OpportunitySnapshot, TokenBookState

### Community 35 - "Community 35"
Cohesion: 0.23
Nodes (8): BookUpdate, FastOrderBook, from_fixed(), PriceChange, test_branchless_update(), test_fixed_point(), to_fixed(), update_price_branchless()

### Community 36 - "Community 36"
Cohesion: 0.19
Nodes (5): fetch_market_tokens(), get_default_tokens(), get_tokens_from_api(), main(), PnLStats

### Community 37 - "Community 37"
Cohesion: 0.17
Nodes (7): check_liquidity(), GhostResult, run_ghost_simulation(), BackgroundTask, LatencyBatch, OpportunitySnapshot, TradeExecution

### Community 38 - "Community 38"
Cohesion: 0.23
Nodes (3): PnlTracker, test_record_fill(), test_record_spread()

### Community 39 - "Community 39"
Cohesion: 0.24
Nodes (9): generate_report(), get_system_health(), main(), Metrics, parse_log(), Generate monitoring report, Send message to Telegram, Parse log file for metrics (+1 more)

### Community 40 - "Community 40"
Cohesion: 0.29
Nodes (11): calculate_rates(), format_report(), generate_hmac(), load_state(), main(), parse_events(), Read last 1000 lines of JSONL log, Parse JSONL events into metrics (+3 more)

### Community 41 - "Community 41"
Cohesion: 0.21
Nodes (5): Eip712Order, OrderBuilder, OrderPayload, OrderType, test_price_decimal_validation()

### Community 42 - "Community 42"
Cohesion: 0.29
Nodes (5): HotPathState, pin_to_core(), spawn_hot_path(), test_arbitrage_check(), test_hot_path_update()

### Community 43 - "Community 43"
Cohesion: 0.2
Nodes (4): GhostResult, GhostSimulator, GhostStats, PendingGhost

### Community 44 - "Community 44"
Cohesion: 0.31
Nodes (8): build_condition_map(), test_build_condition_map(), fast_hash(), fetch_market_by_slug(), fetch_tokens_with_pairs(), get_current_periods(), main(), MarketInfo

### Community 45 - "Community 45"
Cohesion: 0.27
Nodes (2): LocalOrderBook, MarketData

### Community 46 - "Community 46"
Cohesion: 0.46
Nodes (7): calculate_hybrid_fee(), calculate_maker_rebate(), calculate_pure_taker_fee(), calculate_taker_fee(), test_fee_at_50_percent(), test_fee_at_90_percent(), test_hybrid_fee()

### Community 47 - "Community 47"
Cohesion: 0.46
Nodes (7): build_maps(), get_active_15m_periods(), get_active_5m_periods(), get_current_15m_period(), get_current_5m_period(), hash_token(), query_market()

### Community 48 - "Community 48"
Cohesion: 0.39
Nodes (6): check_liquidity(), fetch_market_by_slug(), fetch_tokens(), get_current_periods(), main(), MarketInfo

### Community 49 - "Community 49"
Cohesion: 0.48
Nodes (6): check_liquidity(), fetch_market_by_slug(), fetch_tokens(), get_current_periods(), main(), MarketInfo

### Community 50 - "Community 50"
Cohesion: 0.38
Nodes (3): JsonlLogger, LogEvent, run_logger_thread()

### Community 51 - "Community 51"
Cohesion: 0.38
Nodes (6): BookUpdate, fast_hash(), OrderBookUpdate, parse_fixed_6(), parse_orderbook_update_simd(), PriceChange

### Community 52 - "Community 52"
Cohesion: 0.38
Nodes (6): BookUpdate, fast_hash(), OrderBookUpdate, parse_fixed_6(), parse_orderbook_update_simd(), PriceChange

### Community 53 - "Community 53"
Cohesion: 0.48
Nodes (2): JsonlLogger, LogEvent

### Community 54 - "Community 54"
Cohesion: 0.48
Nodes (6): fast_hash(), fetch_market_by_slug(), fetch_tokens_with_pairs(), get_current_periods(), main(), MarketInfo

### Community 55 - "Community 55"
Cohesion: 0.43
Nodes (6): BackgroundTask, fast_hash(), GhostStatus, parse_fixed_6(), process_text_message(), run_sync_hot_path()

### Community 56 - "Community 56"
Cohesion: 0.33
Nodes (3): execute_ctf_merge(), MergeTask, run_merge_worker()

### Community 57 - "Community 57"
Cohesion: 0.48
Nodes (6): fast_hash(), fetch_market_by_slug(), fetch_tokens_with_pairs(), get_current_periods(), main(), MarketInfo

### Community 58 - "Community 58"
Cohesion: 0.48
Nodes (5): fetch_active_crypto_markets(), fetch_market_by_slug(), fetch_market_by_slug_rollover(), generate_target_slugs(), is_market_active()

### Community 59 - "Community 59"
Cohesion: 0.29
Nodes (1): DryRunStats

### Community 60 - "Community 60"
Cohesion: 0.53
Nodes (5): BackgroundTask, fast_hash(), parse_fixed_6(), process_message(), run_sync_hot_path()

### Community 61 - "Community 61"
Cohesion: 0.53
Nodes (5): fetch_market_by_slug(), fetch_tokens(), get_current_periods(), main(), MarketInfo

### Community 62 - "Community 62"
Cohesion: 0.6
Nodes (5): get_disk_usage(), get_hft_stats(), main(), parse_opportunities(), send_telegram()

### Community 63 - "Community 63"
Cohesion: 0.53
Nodes (5): BackgroundTask, fast_hash(), parse_fixed_6(), process_message(), run_sync_hot_path()

### Community 64 - "Community 64"
Cohesion: 0.47
Nodes (5): BackgroundTask, fast_hash(), GhostStatus, parse_fixed_6(), run_sync_hot_path()

### Community 65 - "Community 65"
Cohesion: 0.33
Nodes (5): get_stats(), Parse HFT log for latency stats and opportunity tracking., # TODO: When HFT binary logs opportunities, parse from log, Send PnL report to Telegram., send_report()

### Community 66 - "Community 66"
Cohesion: 0.53
Nodes (5): BackgroundTask, fast_hash(), parse_fixed_6(), process_message(), run_sync_hot_path()

### Community 67 - "Community 67"
Cohesion: 0.53
Nodes (5): fetch_active_crypto_markets(), fetch_market_by_slug(), get_current_periods(), MarketInfo, test_get_current_periods()

### Community 68 - "Community 68"
Cohesion: 0.4
Nodes (3): fetch_active_markets(), SafeStringSlice, test_fetch_active_markets()

### Community 69 - "Community 69"
Cohesion: 0.6
Nodes (5): check_and_trigger_merge(), generate_ws_signature(), process_ws_message(), run_user_ws(), test_ws_signature()

### Community 70 - "Community 70"
Cohesion: 0.53
Nodes (4): fetch_active_crypto_markets(), fetch_market_by_slug(), fetch_market_by_slug_rollover(), generate_target_slugs()

### Community 71 - "Community 71"
Cohesion: 0.53
Nodes (5): BackgroundTask, fast_hash(), parse_fixed_6(), process_memchr_message(), run_sync_hot_path()

### Community 72 - "Community 72"
Cohesion: 0.47
Nodes (5): EdgeDetected, fast_hash(), parse_fixed_6(), run_sync_hot_path(), TokenBookState

### Community 73 - "Community 73"
Cohesion: 0.33
Nodes (0): 

### Community 74 - "Community 74"
Cohesion: 0.4
Nodes (2): get_hft_stats(), Parse log for HFT stats

### Community 75 - "Community 75"
Cohesion: 0.4
Nodes (1): ExecutionMode

### Community 76 - "Community 76"
Cohesion: 0.6
Nodes (4): BackgroundTask, fast_hash(), parse_fixed_6(), run_sync_hot_path()

### Community 77 - "Community 77"
Cohesion: 0.4
Nodes (1): WebSocketReader

### Community 78 - "Community 78"
Cohesion: 0.6
Nodes (3): build_condition_map(), build_maps(), hash_token()

### Community 79 - "Community 79"
Cohesion: 0.5
Nodes (2): connect_to_polymarket(), WebSocketReader

### Community 80 - "Community 80"
Cohesion: 0.6
Nodes (3): build_condition_map(), build_maps(), hash_token()

### Community 81 - "Community 81"
Cohesion: 0.4
Nodes (2): get_hft_stats(), Parse log for HFT stats

### Community 82 - "Community 82"
Cohesion: 0.4
Nodes (1): MarketInfo

### Community 83 - "Community 83"
Cohesion: 0.4
Nodes (2): get_hft_stats(), Parse log for HFT stats

### Community 84 - "Community 84"
Cohesion: 0.7
Nodes (4): build_condition_map(), build_maps(), hash_token(), parse_iso_date()

### Community 85 - "Community 85"
Cohesion: 0.4
Nodes (2): get_hft_stats(), Parse log for HFT stats

### Community 86 - "Community 86"
Cohesion: 0.5
Nodes (2): fetch_active_markets(), test_fetch_active_markets()

### Community 87 - "Community 87"
Cohesion: 0.4
Nodes (2): MakerLeg, TakerLeg

### Community 88 - "Community 88"
Cohesion: 0.5
Nodes (1): MarketDiscovery

### Community 89 - "Community 89"
Cohesion: 0.83
Nodes (3): build_condition_map(), build_maps(), hash_token()

### Community 90 - "Community 90"
Cohesion: 0.83
Nodes (3): fetch_tokens(), get_current_periods(), main()

### Community 91 - "Community 91"
Cohesion: 0.83
Nodes (3): fetch_market_tokens(), get_target_slugs(), run_rollover_thread()

### Community 92 - "Community 92"
Cohesion: 0.5
Nodes (1): WebSocketReader

### Community 93 - "Community 93"
Cohesion: 0.83
Nodes (3): build_condition_map(), build_maps(), hash_token()

### Community 94 - "Community 94"
Cohesion: 0.83
Nodes (3): fetch_market_tokens(), get_target_slugs(), run_rollover_thread()

### Community 95 - "Community 95"
Cohesion: 0.5
Nodes (0): 

### Community 96 - "Community 96"
Cohesion: 0.83
Nodes (3): fetch_market_tokens(), get_target_slugs(), run_rollover_thread()

### Community 97 - "Community 97"
Cohesion: 0.5
Nodes (1): WebSocketReader

### Community 98 - "Community 98"
Cohesion: 0.5
Nodes (1): WebSocketReader

### Community 99 - "Community 99"
Cohesion: 0.83
Nodes (3): build_condition_map(), build_maps(), hash_token()

### Community 100 - "Community 100"
Cohesion: 0.83
Nodes (3): fetch_market_tokens(), get_target_slugs(), run_rollover_thread()

### Community 101 - "Community 101"
Cohesion: 0.83
Nodes (3): get_hft_stats(), get_opportunity_stats(), send_report()

### Community 102 - "Community 102"
Cohesion: 0.5
Nodes (1): OrderbookSnapshot

### Community 103 - "Community 103"
Cohesion: 0.5
Nodes (0): 

### Community 104 - "Community 104"
Cohesion: 0.83
Nodes (3): create_orderbook_stream(), run_websocket_stream(), test_websocket_connection()

### Community 105 - "Community 105"
Cohesion: 0.5
Nodes (0): 

### Community 106 - "Community 106"
Cohesion: 0.67
Nodes (2): handle_event(), run_user_ws()

### Community 107 - "Community 107"
Cohesion: 0.83
Nodes (3): build_condition_map(), build_maps(), hash_token()

### Community 108 - "Community 108"
Cohesion: 0.5
Nodes (2): GammaEvent, GammaMarket

### Community 109 - "Community 109"
Cohesion: 1.0
Nodes (2): send_start_alert(), start_resilient_bot()

### Community 110 - "Community 110"
Cohesion: 0.67
Nodes (0): 

### Community 111 - "Community 111"
Cohesion: 1.0
Nodes (2): fetch_tokens(), main()

### Community 112 - "Community 112"
Cohesion: 1.0
Nodes (0): 

### Community 113 - "Community 113"
Cohesion: 1.0
Nodes (1): OrderBookUpdate

### Community 114 - "Community 114"
Cohesion: 1.0
Nodes (0): 

### Community 115 - "Community 115"
Cohesion: 1.0
Nodes (0): 

### Community 116 - "Community 116"
Cohesion: 1.0
Nodes (0): 

### Community 117 - "Community 117"
Cohesion: 1.0
Nodes (1): str

### Community 118 - "Community 118"
Cohesion: 1.0
Nodes (0): 

### Community 119 - "Community 119"
Cohesion: 1.0
Nodes (1): Order

### Community 120 - "Community 120"
Cohesion: 1.0
Nodes (0): 

### Community 121 - "Community 121"
Cohesion: 1.0
Nodes (0): 

### Community 122 - "Community 122"
Cohesion: 1.0
Nodes (0): 

## Knowledge Gaps
- **162 isolated node(s):** `GhostResult`, `PendingGhost`, `GhostStats`, `OpportunitySnapshot`, `Parse log for HFT stats` (+157 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **Thin community `Community 112`** (2 nodes): `sdk_test.rs`, `main()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 113`** (2 nodes): `orderbook_update.rs`, `OrderBookUpdate`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 114`** (2 nodes): `main_new.rs`, `main()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 115`** (2 nodes): `debug_ws.rs`, `main()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 116`** (2 nodes): `simd_extractor.rs`, `extract_all_borrowed()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 117`** (2 nodes): `str`, `.safe_slice()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 118`** (2 nodes): `start_alert.rs`, `send_start_alert()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 119`** (2 nodes): `Order`, `.fmt()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 120`** (1 nodes): `pairs_final.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 121`** (1 nodes): `INTEGRATION.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 122`** (1 nodes): `edge_debug.py`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **What connects `GhostResult`, `PendingGhost`, `GhostStats` to the rest of the system?**
  _162 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Community 0` be split into smaller, more focused modules?**
  _Cohesion score 0.05 - nodes in this community are weakly interconnected._
- **Should `Community 1` be split into smaller, more focused modules?**
  _Cohesion score 0.04 - nodes in this community are weakly interconnected._
- **Should `Community 2` be split into smaller, more focused modules?**
  _Cohesion score 0.06 - nodes in this community are weakly interconnected._
- **Should `Community 3` be split into smaller, more focused modules?**
  _Cohesion score 0.08 - nodes in this community are weakly interconnected._
- **Should `Community 4` be split into smaller, more focused modules?**
  _Cohesion score 0.08 - nodes in this community are weakly interconnected._
- **Should `Community 5` be split into smaller, more focused modules?**
  _Cohesion score 0.09 - nodes in this community are weakly interconnected._