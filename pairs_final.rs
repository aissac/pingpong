            // Iterate through each YES/NO pair to show Combined ASK
            for (yes_hash, no_hash) in token_pairs.iter() {
                if let (Some(yes_state), Some(no_state)) = (orderbook.get(yes_hash), orderbook.get(no_hash)) {
                    if let (
                        Some((yes_bid, _)), 
                        Some((yes_ask, _)), 
                        Some((no_bid, _)), 
                        Some((no_ask, _))
                    ) = (
                        yes_state.get_best_bid(),
                        yes_state.get_best_ask(),
                        no_state.get_best_bid(),
                        no_state.get_best_ask()
                    ) {
                        let combined_ask_cents = yes_ask + no_ask;
                        println!(
                            "[COMBINED] YES: Ask=${:.2} Bid=${:.2} | NO: Ask=${:.2} Bid=${:.2} => Combined=${:.2}",
                            yes_ask as f64 / 100.0,
                            yes_bid as f64 / 100.0,
                            no_ask as f64 / 100.0,
                            no_bid as f64 / 100.0,
                            combined_ask_cents as f64 / 100.0
                        );
                    }
                }
            }