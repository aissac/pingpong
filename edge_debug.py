#!/usr/bin/env python3
"""Check current orderbook combined prices"""

import json
import requests
from datetime import datetime, timezone

# Get current periods
now = int(datetime.now(timezone.utc).timestamp())
period_15m = (now // 900) * 900
period_5m = (now // 300) * 300

print(f"Current 15m period: {period_15m}")
print(f"Current 5m period: {period_5m}")
print()

# Query a BTC 15m market
slug = f"btc-updown-15m-{period_15m}"
url = f"https://gamma-api.polymarket.com/events?slug={slug}"

try:
    resp = requests.get(url, timeout=10)
    data = resp.json()
    
    if data and len(data) > 0:
        event = data[0]
        print(f"Event: {event.get('slug', 'unknown')}")
        
        markets = event.get('markets', [])
        if markets:
            market = markets[0]
            question = market.get('question', 'unknown')[:60]
            print(f"Question: {question}...")
            
            # Get token IDs
            clob_ids_str = market.get('clobTokenIds', '[]')
            clob_ids = json.loads(clob_ids_str) if isinstance(clob_ids_str, str) else clob_ids_str
            
            if len(clob_ids) >= 2:
                yes_token = clob_ids[0]
                no_token = clob_ids[1]
                print(f"YES token: {yes_token[:30]}...")
                print(f"NO token: {no_token[:30]}...")
                
                # Get orderbook from CLOB
                clob_url = f"https://clob.polymarket.com/book?token_id={yes_token}"
                book_resp = requests.get(clob_url, timeout=10)
                book = book_resp.json()
                
                bids = book.get('bids', [])[:3]
                asks = book.get('asks', [])[:3]
                
                print()
                print("Top 3 Bids (YES):")
                for b in bids:
                    print(f"  Price: ${float(b.get('price', 0)):.4f}, Size: {b.get('size', 0)}")
                
                print()
                print("Top 3 Asks (YES):")
                for a in asks:
                    print(f"  Price: ${float(a.get('price', 0)):.4f}, Size: {a.get('size', 0)}")
                
                # Get NO orderbook
                clob_url_no = f"https://clob.polymarket.com/book?token_id={no_token}"
                no_resp = requests.get(clob_url_no, timeout=10)
                no_book = no_resp.json()
                
                no_bids = no_book.get('bids', [])[:3]
                no_asks = no_book.get('asks', [])[:3]
                
                print()
                print("Top 3 Bids (NO):")
                for b in no_bids:
                    print(f"  Price: ${float(b.get('price', 0)):.4f}, Size: {b.get('size', 0)}")
                
                print()
                print("Top 3 Asks (NO):")
                for a in no_asks:
                    print(f"  Price: ${float(a.get('price', 0)):.4f}, Size: {a.get('size', 0)}")
                
                if bids and asks and no_bids and no_asks:
                    yes_best_bid = float(bids[0].get('price', 0))
                    yes_best_ask = float(asks[0].get('price', 0))
                    no_best_bid = float(no_bids[0].get('price', 0))
                    no_best_ask = float(no_asks[0].get('price', 0))
                    
                    # Combined price = YES + NO
                    combined_ask = yes_best_ask + no_best_ask
                    combined_bid = yes_best_bid + no_best_bid
                    
                    print()
                    print("=" * 50)
                    print("COMBINED PRICES:")
                    print(f"  Combined Ask: ${combined_ask:.4f}")
                    print(f"  Combined Bid: ${combined_bid:.4f}")
                    
                    if combined_ask < 1.0:
                        arb_edge = 1.0 - combined_ask
                        print(f"  Arbitrage Edge (ask): {arb_edge*100:.2f}%")
                    
                    if combined_bid < 1.0:
                        arb_edge = 1.0 - combined_bid
                        print(f"  Arbitrage Edge (bid): {arb_edge*100:.2f}%")
                    
                    print()
                    print(f"Threshold: $0.98 (2% edge)")
                    print(f"Would trigger edge? {'YES' if combined_bid <= 0.98 or combined_ask <= 0.98 else 'NO'}")
    else:
        print(f"No market found for slug: {slug}")
        
except Exception as e:
    print(f"Error: {e}")