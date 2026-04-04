import urllib.request
import json

# Get the btc-updown market directly
url = 'https://gamma-api.polymarket.com/events?slug=btc-updown-15m-1774845000'
req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
resp = urllib.request.urlopen(req, timeout=10)
data = json.loads(resp.read().decode())

event = data[0]
print(f'Event: {event.get("slug", "unknown")}')
print(f'Title: {event.get("title", "unknown")}')

markets = event.get('markets', [])
for m in markets:
    question = m.get('question', '')
    if 'up' in question.lower() or 'down' in question.lower() or 'higher' in question.lower():
        print(f'\nQuestion: {question}')
        outcomes = m.get('outcomes', [])
        if isinstance(outcomes, str):
            outcomes = json.loads(outcomes)
        print(f'Outcomes: {outcomes}')
        
        clob_token_ids = m.get('clobTokenIds', [])
        if isinstance(clob_token_ids, str):
            clob_token_ids = json.loads(clob_token_ids)
        print(f'Token IDs: {len(clob_token_ids)} tokens')
        
        if len(clob_token_ids) >= 2:
            yes_token = clob_token_ids[0]
            no_token = clob_token_ids[1]
            
            # Get orderbook
            yes_url = f'https://clob.polymarket.com/book?token_id={yes_token}'
            resp = urllib.request.urlopen(yes_url, timeout=5)
            yes_book = json.loads(resp.read().decode())
            yes_asks = yes_book.get('asks', [])
            yes_bids = yes_book.get('bids', [])
            
            no_url = f'https://clob.polymarket.com/book?token_id={no_token}'
            resp = urllib.request.urlopen(no_url, timeout=5)
            no_book = json.loads(resp.read().decode())
            no_asks = no_book.get('asks', [])
            no_bids = no_book.get('bids', [])
            
            print(f'\nYES Orderbook:')
            print(f'  Best Bid: ${float(yes_bids[0]["price"]):.4f} (size: {yes_bids[0]["size"][:15]})' if yes_bids else '  Best Bid: NONE')
            print(f'  Best Ask: ${float(yes_asks[0]["price"]):.4f} (size: {yes_asks[0]["size"][:15]})' if yes_asks else '  Best Ask: NONE')
            
            print(f'\nNO Orderbook:')
            print(f'  Best Bid: ${float(no_bids[0]["price"]):.4f} (size: {no_bids[0]["size"][:15]})' if no_bids else '  Best Bid: NONE')
            print(f'  Best Ask: ${float(no_asks[0]["price"]):.4f} (size: {no_asks[0]["size"][:15]})' if no_asks else '  Best Ask: NONE')
            
            if yes_asks and no_asks:
                yes_ask = float(yes_asks[0]['price'])
                no_ask = float(no_asks[0]['price'])
                combined = yes_ask + no_ask
                
                print(f'\n=== EDGE ANALYSIS ===')
                print(f'YES Ask: ${yes_ask:.4f}')
                print(f'NO  Ask: ${no_ask:.4f}')
                print(f'COMBINED ASK: ${combined:.4f}')
                print(f'Threshold: $0.98')
                print(f'Edge detected: {"YES" if combined < 0.98 else "NO"}')
                print(f'')
                print(f'Why no edge?')
                if combined >= 1.0:
                    print(f'  Combined ASK >= $1.00 (market efficient)')
                elif combined >= 0.98:
                    print(f'  Combined ASK between $0.98-$1.00')
                    print(f'  Market makers capturing spread')
                    print(f'  No arbitrage opportunity')
        break