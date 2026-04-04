import asyncio
import websockets
import json

WS_URL = "wss://ws-subscriptions-clob.polymarket.com/ws/market"
TOKENS = [
    "62625969519959851805884043220691879906721973091460503820174555934002939150903",
    "88235721644125608625896636244500487000588960218563144101866194058161843991887"
]

async def log_messages():
    async with websockets.connect(WS_URL) as ws:
        # Subscribe
        subscribe = {
            "type": "market",
            "operation": "subscribe",
            "markets": [],
            "assets_ids": TOKENS,
            "initial_dump": True
        }
        await ws.send(json.dumps(subscribe))
        print("Subscribed to", len(TOKENS), "tokens")
        
        # Log first 3 messages
        for i in range(3):
            msg = await ws.recv()
            data = json.loads(msg)
            print(f"\n=== MESSAGE {i+1} ===")
            print(json.dumps(data, indent=2)[:2000])
            
            # Check for bids/asks structure
            msg_str = json.dumps(data)
            if "bids" in msg_str or "asks" in msg_str:
                print("\nFound bids/asks in message!")
                if isinstance(data, dict):
                    for key in ["bids", "asks", "price_changes"]:
                        if key in data:
                            print(f"{key}: {str(data[key])[:500]}")

asyncio.run(log_messages())