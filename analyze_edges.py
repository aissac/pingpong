import urllib.request
import json

# Get orderbook for BTC/ETH updown markets
url = 'https://gamma-api.polymarket.com/events?closed=false&active=true&limit=100'
req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
resp = urllib.request.urlopen(req, timeout=10)
events = json.loads(resp.read().decode())

markets_analyzed = 0
for event in events:
    slug = event.get('slug', '')
    if 'btc-updown' in slug or 'eth-updown' in slug:
        for market in event.get('markets', []):
            clob_token_ids = market.get('clobTokenIds', [])
            if isinstance(clob_token_ids, str):
                clob_token_ids = json.loads(clob_token_ids)
            if len(clob_token_ids) >= 2:
                yes_token = clob_token_ids[0]
                no_token = clob_token_ids[1]
                
                # Get orderbooks
                try:
                    yes_url = f'https://clob.polymarket.com/book?token_id={yes_token}'
                    resp = urllib.request.urlopen(yes_url, timeout=5)
                    yes_book = json.loads(resp.read().decode())
                    yes_asks = yes_book.get('asks', [])
                    
                    no_url = f'https://clob.polymarket.com/book?token_id={no_token}'
                    resp = urllib.request.urlopen(no_url, timeout=5)
                    no_book = json.loads(resp.read().decode())
                    no_asks = no_book.get('asks', [])
                    
                    if yes_asks and no_asks:
                        yes_ask = float(yes_asks[0]['price'])
                        no_ask = float(no_asks[0]['price'])
                        combined_ask = yes_ask + no_ask
                        
                        print(f'{slug[:30]}:')
                        print(f'  YES Ask: ${yes_ask:.4f}')
                        print(f'  NO  Ask: ${no_ask:.4f}')
                        print(f'  COMBINED: ${combined_ask:.4f} {"<-- EDGE!" if combined_ask < 0.98 else ""}')
                        
                        markets_analyzed += 1
                        if markets_analyzed >= 6:
                            break
                except Exception as e:
                    pass
        
        if markets_analyzed >= 6:
            break

print(f'\nAnalyzed {markets_analyzed} markets')
print(f'Threshold: $0.98')
print(f'Edges found: Markets where YES Ask + NO Ask < $0.98')