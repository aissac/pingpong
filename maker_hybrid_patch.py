# Add Maker hybrid detection logic before SWEET SPOT ARB log

with open('/home/ubuntu/polymarket-hft-engine/src/main.rs', 'r') as f:
    content = f.read()

# Find the section we want to modify
old_section = '''                        // HARD CAP: 0 max per trade (notebooklm recommendation)
                        let max_cap_shares = ((50.0 / combined) as i32).max(10);
                        let shares = shares.min(max_cap_shares);
                        let shares_f = shares as f64;
                        let shares_f = shares as f64;
                        
                        info!(
                            "🎯 SWEET SPOT ARB! | {} | YES: ${:.4} + NO: ${:.4} = ${:.4} | Edge: {:.1}% | Size: {:.0}",
                            &condition_id[..8.min(condition_id.len())],
                            yes_price,
                            no_price,
                            combined,
                            edge * 100.0,
                            shares_f
                        );'''

new_section = '''                        // HARD CAP: 100 max per trade (notebooklm recommendation)
                        let max_cap_shares = ((100.0 / combined) as i32).max(10);
                        let shares = shares.min(max_cap_shares);
                        let shares_f = shares as f64;
                        
                        // ═══════════════════════════════════════════════════════
                        // MAKER HYBRID CHECK
                        // Prefer Maker orders on extreme probabilities (p<0.30 or p>0.70)
                        // This earns 20% rebate instead of paying 1.80% taker fee
                        // ═══════════════════════════════════════════════════════
                        let p = yes_price / combined;
                        let is_extreme_prob = p < 0.30 || p > 0.70;
                        
                        // Identify volatile side (lower depth = more volatile)
                        let (maker_side, maker_price, taker_price) = if yes_depth < no_depth {
                            // YES is more volatile - post Maker on YES
                            ("YES", yes_price - 0.01, no_price)
                        } else {
                            // NO is more volatile - post Maker on NO
                            ("NO", no_price - 0.01, yes_price)
                        };
                        
                        // Dynamic fee based on probability
                        let variance = p * (1.0 - p);
                        let dynamic_fee = (0.25 * variance * variance).max(0.01);
                        
                        // Maker rebate benefit (20% of max taker fee)
                        let maker_rebate = 0.20 * 0.0156; // ~0.3%
                        let profit_with_rebate = profit_per_share + maker_rebate;
                        
                        if is_extreme_prob {
                            // Log Maker hybrid opportunity
                            info!(
                                "🎯 MAKER HYBRID! | {} | {} @ ${:.4} (Maker) + {} @ ${:.4} (Taker) = ${:.4} | Edge: {:.1}% | Fee: {:.2}% | Net: ${:.4}/sh",
                                &condition_id[..8.min(condition_id.len())],
                                maker_side,
                                maker_price,
                                if maker_side == "YES" { "NO" } else { "YES" },
                                taker_price,
                                combined,
                                edge * 100.0,
                                dynamic_fee * 100.0,
                                profit_with_rebate
                            );
                        } else {
                            // Regular Taker signal for non-extreme probabilities
                            info!(
                                "🎯 SWEET SPOT ARB! | {} | YES: ${:.4} + NO: ${:.4} = ${:.4} | Edge: {:.1}% | Size: {:.0}",
                                &condition_id[..8.min(condition_id.len())],
                                yes_price,
                                no_price,
                                combined,
                                edge * 100.0,
                                shares_f
                            );
                        }'''

content = content.replace(old_section, new_section)

with open('/home/ubuntu/polymarket-hft-engine/src/main.rs', 'w') as f:
    f.write(content)

print('Added Maker hybrid detection logic')