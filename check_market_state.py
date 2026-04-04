import urllib.request
import json

# Get BTC updown market orderbook
print("=== Checking BTC Up/Down Market Orderbooks ===")
print()

# Try to find the market via slug
url = 'https://gamma-api.polymarket.com/events?active=true&closed=false&limit=50'
req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
resp = urllib.request.urlopen(req, timeout=10)
events = json.loads(resp.read().decode())

btc_markets = []
for event in events:
    slug = event.get('slug', '')
    if 'btc-updown' in slug or 'eth-updown' in slug:
        btc_markets.append({
            'slug': slug,
            'markets': event.get('markets', [])
        })
        if len(btc_markets) >= 3:
            break

print(f"Found {len(btc_markets)} up/down markets")
print()

for market in btc_markets:
    print(f"Market: {market['slug']}")
    for m in market['markets'][:1]:  # First market only
        clob_token_ids = m.get('clobTokenIds', [])
        if isinstance(clob_token_ids, str):
            clob_token_ids = json.loads(clob_token_ids)
        
        if len(clob_token_ids) >= 2:
            yes_token = clob_token_ids[0]
            no_token = clob_token_ids[1]
            
            # Get YES orderbook
            try:
                yes_url = f'https://clob.polymarket.com/book?token_id={yes_token}'
                resp = urllib.request.urlopen(yes_url, timeout=5)
                yes_book = json.loads(resp.read().decode())
                yes_asks = yes_book.get('asks', [])[:3]
                yes_bids = yes_book.get('bids', [])[:3]
                
                print(f"  YES token: {yes_token[:20]}...")
                print(f"  YES Best Ask: ${float(yes_asks[0]['price']):.4f} x {yes_asks[0]['size'][:10]}" if yes_asks else "  YES Best Ask: NONE")
                print(f"  YES Best Bid: ${float(yes_bids[0]['price']):.4f} x {yes_bids[0]['size'][:10]}" if yes_bids else "  YES Best Bid: NONE")
            except Exception as e:
                print(f"  YES Error: {e}")
            
            # Get NO orderbook
            try:
                no_url = f'https://clob.polymarket.com/book?token_id={no_token}'
                resp = urllib.request.urlopen(no_url, timeout=5)
                no_book = json.loads(resp.read().decode())
                no_asks = no_book.get('asks', [])[:3]
                no_bids = no_book.get('bids', [])[:3]
                
                print(f"  NO  Best Ask: ${float(no_asks[0]['price']):.4f} x {no_asks[0]['size'][:10]}" if no_asks else "  NO  Best Ask: NONE")
                print(f"  NO  Best Bid: ${float(no_bids[0]['price']):.4f} x {no_bids[0]['size'][:10]}" if no_bids else "  NO  Best Bid: NONE")
            except Exception as e:
                print(f"  NO  Error: {e}")
            
            # Calculate combined ASK
            if yes_asks and no_asks:
                yes_ask = float(yes_asks[0]['price'])
                no_ask = float(no_asks[0]['price'])
                combined = yes_ask + no_ask
                print(f"  COMBINED ASK: ${combined:.4f}")
                print(f"  Edge threshold: $0.98")
                print(f"  Edge detected: {'YES!' if combined < 0.98 else 'NO'}")
            
            # Check if both sides have bids
            if yes_bids and no_bids:
                print(f"  ✅ Both sides have bids")
            else:
                print(f"  ⚠️ Missing bids: YES={len(yes_bids)} NO={len(no_bids)}")
            
            print()