import urllib.request
import json

# Get BTC updown market
slug = 'btc-updown-15m-1774847700'
url = f'https://gamma-api.polymarket.com/events?slug={slug}'
req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
resp = urllib.request.urlopen(req, timeout=10)
data = json.loads(resp.read().decode())

event = data[0]
print(f'Event: {event.get("title", "unknown")}')

for m in event.get('markets', []):
    clob_token_ids = m.get('clobTokenIds', [])
    if isinstance(clob_token_ids, str):
        clob_token_ids = json.loads(clob_token_ids)
    
    if len(clob_token_ids) >= 2:
        yes_token = clob_token_ids[0]
        no_token = clob_token_ids[1]
        
        print(f'\nYES token: {yes_token[:30]}...')
        
        # Get YES orderbook
        yes_url = f'https://clob.polymarket.com/book?token_id={yes_token}'
        resp = urllib.request.urlopen(yes_url, timeout=5)
        yes_book = json.loads(resp.read().decode())
        yes_asks = yes_book.get('asks', [])[:3]
        yes_bids = yes_book.get('bids', [])[:3]
        
        print(f'YES Asks (SELL orders):')
        for a in yes_asks:
            print(f'  ${float(a["price"]):.4f} x {a["size"][:10]}')
        print(f'YES Bids (BUY orders):')
        for b in yes_bids:
            print(f'  ${float(b["price"]):.4f} x {b["size"][:10]}')
        
        print(f'\nNO token: {no_token[:30]}...')
        
        # Get NO orderbook  
        no_url = f'https://clob.polymarket.com/book?token_id={no_token}'
        resp = urllib.request.urlopen(no_url, timeout=5)
        no_book = json.loads(resp.read().decode())
        no_asks = no_book.get('asks', [])[:3]
        no_bids = no_book.get('bids', [])[:3]
        
        print(f'NO Asks (SELL orders):')
        for a in no_asks:
            print(f'  ${float(a["price"]):.4f} x {a["size"][:10]}')
        print(f'NO Bids (BUY orders):')
        for b in no_bids:
            print(f'  ${float(b["price"]):.4f} x {b["size"][:10]}')
        
        # Calculate edge
        if yes_asks and no_asks:
            combined = float(yes_asks[0]['price']) + float(no_asks[0]['price'])
            print(f'\n=== EDGE ANALYSIS ===')
            print(f'YES Ask: ${float(yes_asks[0]["price"]):.4f}')
            print(f'NO  Ask: ${float(no_asks[0]["price"]):.4f}')
            print(f'COMBINED ASK: ${combined:.4f}')
            print(f'Threshold: $0.98')
            if combined < 0.98:
                print(f'Edge: YES!')
            else:
                print(f'Edge: NO (need < $0.98)')
        
        # Orderbook health
        print(f'\n=== ORDERBOOK HEALTH ===')
        print(f'YES has asks: {len(yes_asks) > 0}')
        print(f'YES has bids: {len(yes_bids) > 0}')
        print(f'NO has asks: {len(no_asks) > 0}')
        print(f'NO has bids: {len(no_bids) > 0}')
        
        break