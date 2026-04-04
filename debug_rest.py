import requests
import json

token = "62625969519959851805884043220691879906721973091460503820174555934002939150903"
url = f"https://clob.polymarket.com/book?token_id={token}"
resp = requests.get(url, headers={"User-Agent": "Mozilla/5.0"})
data = resp.json()

print("=== REST API Orderbook ===")
print("Token:", token[:20], "...")
print("Bids (BUY orders):")
for bid in data.get("bids", [])[:5]:
    print("  $", bid["price"], "x", bid["size"])
print("Asks (SELL orders):")
for ask in data.get("asks", [])[:5]:
    print("  $", ask["price"], "x", ask["size"])

if data.get("bids") and data.get("asks"):
    best_bid = float(data["bids"][0]["price"])
    best_ask = float(data["asks"][0]["price"])
    print("")
    print("Best Bid: $", best_bid)
    print("Best Ask: $", best_ask)
    print("Spread: $", best_ask - best_bid)
