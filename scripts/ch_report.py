#!/usr/bin/env python3
"""
Pingpong Report Script - Generates Telegram reports from ClickHouse
Run every 15 minutes via cron

Reports:
- 5-minute flow stats
- 1-hour summary
- Session totals
"""
import clickhouse_connect
import requests
from datetime import datetime, timedelta

# Configuration
CH_CLIENT = clickhouse_connect.get_client(host='localhost', port=8123, database='polymarket')
TELEGRAM_BOT_TOKEN = "8754623467:AAHTUYGscxz1eDp2tH3olxuTXGdJHeOf0oY"
TELEGRAM_CHAT_ID = "1798631768"

def send_telegram(message):
    """Send message to Telegram"""
    url = f"https://api.telegram.org/bot{TELEGRAM_BOT_TOKEN}/sendMessage"
    data = {
        "chat_id": TELEGRAM_CHAT_ID,
        "text": message,
        "parse_mode": "Markdown"
    }
    try:
        resp = requests.post(url, json=data, timeout=10)
        return resp.json()
    except Exception as e:
        print(f"Telegram error: {e}")
        return None

def get_5min_stats():
    """Get last 5 minutes of stats"""
    five_min_ago = (datetime.utcnow() - timedelta(minutes=5)).strftime('%Y-%m-%d %H:%M:%S')
    
    # ARBs detected
    try:
        result = CH_CLIENT.query(f"""
            SELECT 
                count() as count,
                avg(edge_pct) as avg_edge,
                max(edge_pct) as max_edge
            FROM arb_events 
            WHERE ts >= '{five_min_ago}'
        """)
        row = result.result_rows[0] if result.result_rows else (0, 0, 0)
        arb_count = row[0] or 0
        avg_edge = round(row[1], 1) if row[1] else 0
        max_edge = round(row[2], 1) if row[2] else 0
    except:
        arb_count, avg_edge, max_edge = 0, 0, 0
    
    # Trades executed
    try:
        result = CH_CLIENT.query(f"""
            SELECT 
                count() as count,
                sum(profit) as total_pnl,
                sum(size) as total_volume
            FROM trades 
            WHERE ts >= '{five_min_ago}'
        """)
        row = result.result_rows[0] if result.result_rows else (0, 0, 0)
        trade_count = row[0] or 0
        total_pnl = round(row[1], 2) if row[1] else 0
        total_volume = round(row[2], 1) if row[2] else 0
    except:
        trade_count, total_pnl, total_volume = 0, 0, 0
    
    # Capital blocks
    try:
        result = CH_CLIENT.query(f"""
            SELECT count() 
            FROM risk_events 
            WHERE ts >= '{five_min_ago}' AND event_type = 'CAPITAL_BLOCK'
        """)
        capital_count = result.result_rows[0][0] if result.result_rows else 0
    except:
        capital_count = 0
    
    # MLE blocks
    try:
        result = CH_CLIENT.query(f"""
            SELECT count() 
            FROM risk_events 
            WHERE ts >= '{five_min_ago}' AND event_type = 'MLE_BLOCK'
        """)
        mle_count = result.result_rows[0][0] if result.result_rows else 0
    except:
        mle_count = 0
    
    # Ghost detections
    try:
        result = CH_CLIENT.query(f"""
            SELECT count() 
            FROM risk_events 
            WHERE ts >= '{five_min_ago}' AND event_type = 'GHOST_LIQUIDITY'
        """)
        ghost_count = result.result_rows[0][0] if result.result_rows else 0
    except:
        ghost_count = 0
    
    # Adverse selection events
    try:
        result = CH_CLIENT.query(f"""
            SELECT 
                count() as count,
                avg(delta) as avg_delta
            FROM adverse_selection_events
            WHERE ts >= '{five_min_ago}'
        """)
        row = result.result_rows[0] if result.result_rows else (0, 0)
        adverse_count = row[0] or 0
        avg_adverse_delta = round(row[1], 2) if row[1] else 0
    except:
        adverse_count, avg_adverse_delta = 0, 0
    
    return {
        'arb_count': arb_count,
        'avg_edge': avg_edge,
        'max_edge': max_edge,
        'trade_count': trade_count,
        'total_pnl': total_pnl,
        'total_volume': total_volume,
        'capital_blocks': capital_count,
        'mle_blocks': mle_count,
        'ghosts': ghost_count,
        'adverse_count': adverse_count,
        'avg_adverse_delta': avg_adverse_delta
    }

def get_hourly_stats():
    """Get last hour stats"""
    hour_ago = (datetime.utcnow() - timedelta(hours=1)).strftime('%Y-%m-%d %H:%M:%S')
    
    # ARBs
    try:
        result = CH_CLIENT.query(f"""
            SELECT 
                count() as count,
                avg(edge_pct) as avg_edge
            FROM arb_events 
            WHERE ts >= '{hour_ago}'
        """)
        row = result.result_rows[0] if result.result_rows else (0, 0)
        arb_count = row[0] or 0
        avg_edge = round(row[1], 1) if row[1] else 0
    except:
        arb_count, avg_edge = 0, 0
    
    # Trades with win/loss breakdown
    try:
        result = CH_CLIENT.query(f"""
            SELECT 
                count() as count,
                sum(profit) as total_pnl,
                sum(CASE WHEN profit > 0 THEN 1 ELSE 0 END) as wins,
                sum(CASE WHEN profit < 0 THEN 1 ELSE 0 END) as losses
            FROM trades 
            WHERE ts >= '{hour_ago}'
        """)
        row = result.result_rows[0] if result.result_rows else (0, 0, 0, 0)
        total_trades = row[0] or 0
        total_pnl = round(row[1], 2) if row[1] else 0
        wins = row[2] or 0
        losses = row[3] or 0
        win_rate = round(wins / total_trades * 100, 1) if total_trades > 0 else 0
    except:
        total_trades, total_pnl, wins, losses, win_rate = 0, 0, 0, 0, 0
    
    # Risk events
    try:
        result = CH_CLIENT.query(f"""
            SELECT 
                sum(CASE WHEN event_type = 'CAPITAL_BLOCK' THEN 1 ELSE 0 END) as capital_blocks,
                sum(CASE WHEN event_type = 'MLE_BLOCK' THEN 1 ELSE 0 END) as mle_blocks,
                sum(CASE WHEN event_type = 'GHOST_LIQUIDITY' THEN 1 ELSE 0 END) as ghosts
            FROM risk_events 
            WHERE ts >= '{hour_ago}'
        """)
        row = result.result_rows[0] if result.result_rows else (0, 0, 0)
        capital_count = row[0] or 0
        mle_count = row[1] or 0
        ghost_count = row[2] or 0
    except:
        capital_count, mle_count, ghost_count = 0, 0, 0
    
    # Adverse selection
    try:
        result = CH_CLIENT.query(f"""
            SELECT 
                count() as count,
                avg(combined_at_10s - combined_at_fill) as avg_delta
            FROM adverse_selection_events
            WHERE ts >= '{hour_ago}'
        """)
        row = result.result_rows[0] if result.result_rows else (0, 0)
        adverse_count = row[0] or 0
        avg_adverse_delta = round(row[1], 2) if row[1] else 0
    except:
        adverse_count, avg_adverse_delta = 0, 0
    
    return {
        'arb_count': arb_count,
        'avg_edge': avg_edge,
        'total_trades': total_trades,
        'total_pnl': total_pnl,
        'wins': wins,
        'losses': losses,
        'win_rate': win_rate,
        'capital_blocks': capital_count,
        'mle_blocks': mle_count,
        'ghosts': ghost_count,
        'adverse_count': adverse_count,
        'avg_adverse_delta': avg_adverse_delta
    }

def get_top_adverse_markets():
    """Get markets with highest adverse selection rates"""
    hour_ago = (datetime.utcnow() - timedelta(hours=1)).strftime('%Y-%m-%d %H:%M:%S')
    
    try:
        result = CH_CLIENT.query(f"""
            SELECT 
                market_id,
                count() as adverse_count,
                round(avg(combined_at_10s - combined_at_fill), 2) as avg_delta
            FROM adverse_selection_events
            WHERE ts >= '{hour_ago}'
            GROUP BY market_id
            ORDER BY adverse_count DESC
            LIMIT 5
        """)
        markets = []
        for row in result.result_rows:
            markets.append({
                'market_id': row[0],
                'count': row[1],
                'avg_delta': row[2]
            })
        return markets
    except:
        return []

def get_total_stats():
    """Get total stats since start"""
    try:
        result = CH_CLIENT.query("SELECT count() FROM arb_events")
        total_arbs = result.result_rows[0][0] if result.result_rows else 0
    except:
        total_arbs = 0
    
    try:
        result = CH_CLIENT.query("SELECT count(), sum(profit) FROM trades")
        row = result.result_rows[0] if result.result_rows else (0, 0)
        total_trades = row[0] or 0
        total_pnl = round(row[1], 2) if row[1] else 0
    except:
        total_trades, total_pnl = 0, 0
    
    return {
        'total_arbs': total_arbs,
        'total_trades': total_trades,
        'total_pnl': total_pnl
    }

def format_report():
    """Generate formatted report"""
    stats_5min = get_5min_stats()
    stats_hour = get_hourly_stats()
    stats_total = get_total_stats()
    top_adverse = get_top_adverse_markets()
    
    # Calculate ghost rate
    ghost_rate = round(stats_hour['ghosts'] / stats_hour['total_trades'] * 100, 1) if stats_hour['total_trades'] > 0 else 0
    
    # Calculate adverse rate
    adverse_rate = round(stats_hour['adverse_count'] / stats_hour['total_trades'] * 100, 1) if stats_hour['total_trades'] > 0 else 0
    
    # Format top adverse markets
    adverse_markets_str = "\n  • No adverse events"
    if top_adverse:
        lines = []
        for m in top_adverse[:3]:
            market_short = m['market_id'][:8]
            lines.append(f"  • {market_short}: {m['count']} events, Δ=${m['avg_delta']}")
        adverse_markets_str = "\n" + "\n".join(lines)
    
    report = f"""🏓 **PINGPONG REPORT** - {datetime.utcnow().strftime('%H:%M:%S')} UTC

📊 **5-Min Flow:**
• ARBs detected: {stats_5min['arb_count']}
• Avg edge: {stats_5min['avg_edge']}%
• Trades executed: {stats_5min['trade_count']}
• Volume: ${stats_5min['total_volume']}
• PnL: ${stats_5min['total_pnl']}

📈 **1-Hour Summary:**
• ARBs: {stats_hour['arb_count']} ({stats_hour['avg_edge']}% avg edge)
• Trades: {stats_hour['total_trades']} (${stats_hour['total_pnl']} PnL)
• Win rate: {stats_hour['win_rate']}%

⚠️ **Risk Events (1hr):**
• Capital blocks: {stats_hour['capital_blocks']}
• MLE blocks: {stats_hour['mle_blocks']}
• Ghost detections: {stats_hour['ghosts']} ({ghost_rate}% of trades)
• Adverse selections: {stats_hour['adverse_count']} ({adverse_rate}% of trades)
• Avg adverse delta: ${stats_hour['avg_adverse_delta']}

🔴 **Top Adverse Markets (1hr):**{adverse_markets_str}

🎯 **Session Total:**
• ARBs: {stats_total['total_arbs']}
• Trades: {stats_total['total_trades']}
• Total PnL: ${stats_total['total_pnl']}
"""
    return report

def main():
    report = format_report()
    print(report)
    send_telegram(report)

if __name__ == "__main__":
    main()