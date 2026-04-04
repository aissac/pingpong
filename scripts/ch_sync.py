#!/usr/bin/env python3
"""
Pingpong Sync Script - Syncs log events to ClickHouse
Run every 5 minutes via cron

Handles log rotation by checking if total_lines < last_marker (reset to 0)

Parses:
- SWEET SPOT ARB! | 0x... | YES: $0.30 + NO: $0.50 = $0.80 | Edge: 18% | Size: 30
- Trade complete: 0x... | | profit=$0.1636 ($4.91 total) [ghost: NO]
- BLOCKED: CAPITAL WINDOW $...+$...>$... for 0x...
- MLE BLOCKED: 0x...
- GHOST LIQUIDITY DETECTED: 0x...
"""
import re
import clickhouse_connect
from datetime import datetime
import json
import os

# Configuration
LOG_PATH = "/tmp/pingpong.log"
MARKER_PATH = "/tmp/ch_marker.txt"
CH_CLIENT = clickhouse_connect.get_client(host='localhost', port=8123, database='polymarket')

def get_marker():
    """Get last processed line number"""
    try:
        with open(MARKER_PATH, 'r') as f:
            return int(f.read().strip())
    except:
        return 0

def set_marker(count):
    """Save last processed line number"""
    with open(MARKER_PATH, 'w') as f:
        f.write(str(count))

def parse_timestamp(line):
    """Extract ISO timestamp from log line"""
    m = re.search(r'(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+Z)', line)
    if m:
        ts_str = m.group(1).replace('Z', '+00:00')
        return datetime.fromisoformat(ts_str)
    return None

def parse_arb(line):
    """Parse: 🎯 SWEET SPOT ARB! | 0x29c99b | btc-updown-5m-123 | YES: $0.3000 + NO: $0.5000 = $0.8000 | Edge: 18.0% | Size: 30"""
    try:
        ts = parse_timestamp(line)
        if not ts:
            return None
        # Try new format with slug
        m = re.search(
            r'SWEET SPOT ARB! \| (0x[0-9a-f]+) \| ([a-z0-9\-]+) \| YES: \$([0-9.]+) \+ NO: \$([0-9.]+) = \$([0-9.]+) \| Edge: ([0-9.]+)% \| Size: ([0-9]+)',
            line
        )
        if m:
            return {
                'type': 'arb',
                'ts': ts,
                'condition_id': m.group(1),
                'slug': m.group(2),
                'yes_price': float(m.group(3)),
                'no_price': float(m.group(4)),
                'edge_pct': float(m.group(6)),
                'size': float(m.group(7)),
                'market_type': 'updown'
            }
        # Fallback to old format without slug
        m = re.search(
            r'SWEET SPOT ARB! \| (0x[0-9a-f]+) \| YES: \$([0-9.]+) \+ NO: \$([0-9.]+) = \$([0-9.]+) \| Edge: ([0-9.]+)% \| Size: ([0-9]+)',
            line
        )
        if m:
            return {
                'type': 'arb',
                'ts': ts,
                'condition_id': m.group(1),
                'slug': '',
                'yes_price': float(m.group(2)),
                'no_price': float(m.group(3)),
                'edge_pct': float(m.group(5)),
                'size': float(m.group(6)),
                'market_type': 'updown'
            }
        return None
    except Exception as e:
        return None

def parse_trade(line):
    """Parse: ✅ Trade complete: 0x29c99b |  | profit=$0.1636 ($4.91 total) [ghost: NO]"""
    try:
        ts = parse_timestamp(line)
        if not ts:
            return None
        m = re.search(r'Trade complete: (0x[0-9a-f]+).*profit=\$([0-9.]+).*\(\$([0-9.]+) total\)', line)
        if m:
            return {
                'type': 'trade',
                'ts': ts,
                'market_id': m.group(1),
                'profit': float(m.group(2)),
                'size': float(m.group(3)),
                'slug': ''
            }
        return None
    except Exception as e:
        return None

def parse_capital_block(line):
    """Parse: BLOCKED: CAPITAL WINDOW $75.00+$10.00>$300.00 for 0x29c99b"""
    try:
        ts = parse_timestamp(line)
        if not ts:
            return None
        m = re.search(r'BLOCKED: CAPITAL WINDOW \$([0-9.]+)\+\$([0-9.]+)>\$([0-9.]+) for (0x[0-9a-f]+)', line)
        if m:
            return {
                'type': 'capital_block',
                'ts': ts,
                'market_id': m.group(4),
                'current': float(m.group(1)),
                'needed': float(m.group(2)),
                'limit': float(m.group(3))
            }
        return None
    except:
        return None

def parse_mle_block(line):
    """Parse: MLE BLOCKED: 0x29c99b"""
    try:
        ts = parse_timestamp(line)
        if not ts:
            return None
        m = re.search(r'MLE BLOCKED: (0x[0-9a-f]+)', line)
        if m:
            return {
                'type': 'mle_block',
                'ts': ts,
                'market_id': m.group(1)
            }
        return None
    except:
        return None

def parse_ghost(line):
    """Parse: 👻 GHOST LIQUIDITY DETECTED: 0x29c99b"""
    try:
        ts = parse_timestamp(line)
        if not ts:
            return None
        m = re.search(r'GHOST LIQUIDITY (?:DETECTED)?: (0x[0-9a-f]+)', line)
        if m:
            return {
                'type': 'ghost',
                'ts': ts,
                'market_id': m.group(1)
            }
        return None
    except:
        return None

def parse_adverse_selection(line):
    """Parse: ⚠️ ADVERSE SELECTION: 0x29c99b | Fill: $0.80 -> 10s: $1.04 | Δ=$0.24"""
    try:
        ts = parse_timestamp(line)
        if not ts:
            return None
        m = re.search(r'ADVERSE SELECTION: (0x[0-9a-f]+) \| Fill: \$([0-9.]+) -> 10s: \$([0-9.]+) \| Δ=\$([0-9.]+)', line)
        if m:
            return {
                'type': 'adverse',
                'ts': ts,
                'market_id': m.group(1),
                'fill_combined': float(m.group(2)),
                'combined_10s': float(m.group(3)),
                'delta': float(m.group(4))
            }
        return None
    except:
        return None

def sync():
    """Main sync function"""
    last_marker = get_marker()
    
    # Count total lines
    total_lines = 0
    try:
        with open(LOG_PATH, 'r') as f:
            for _ in f:
                total_lines += 1
    except FileNotFoundError:
        print(f"Log file not found: {LOG_PATH}")
        return
    
    # HANDLE LOG ROTATION: if total_lines < last_marker, log was rotated
    if total_lines < last_marker:
        print(f"⚠️  Log rotation detected (total: {total_lines}, last: {last_marker})")
        print(f"   Resetting marker to 0 and processing from start")
        last_marker = 0
    
    if total_lines <= last_marker:
        print(f"No new lines (total: {total_lines}, last: {last_marker})")
        return
    
    new_lines = total_lines - last_marker
    print(f"Processing {new_lines} new lines (total: {total_lines}, last: {last_marker})")
    
    # Parse events
    arbs = []
    trades = []
    capital_blocks = []
    mle_blocks = []
    ghosts = []
    adverse_events = []
    
    with open(LOG_PATH, 'r') as f:
        # Skip to last processed position
        for _ in range(last_marker):
            next(f)
        
        # Process new lines
        for line in f:
            event = parse_arb(line)
            if event:
                arbs.append(event)
                continue
            
            event = parse_trade(line)
            if event:
                trades.append(event)
                continue
            
            event = parse_capital_block(line)
            if event:
                capital_blocks.append(event)
                continue
            
            event = parse_mle_block(line)
            if event:
                mle_blocks.append(event)
                continue
            
            event = parse_ghost(line)
            if event:
                ghosts.append(event)
                continue
            
            event = parse_adverse_selection(line)
            if event:
                adverse_events.append(event)
    
    print(f"Parsed: {len(arbs)} ARBs, {len(trades)} trades, {len(capital_blocks)} capital blocks, {len(mle_blocks)} MLE blocks, {len(ghosts)} ghosts, {len(adverse_events)} adverse")
    
    # Insert ARBs
    if arbs:
        data = [(a['ts'], a['condition_id'], a['yes_price'], a['no_price'], a['edge_pct'], a['size'], a['market_type'], a['slug']) for a in arbs]
        try:
            CH_CLIENT.insert('polymarket.arb_events', data, 
                column_names=['ts', 'condition_id', 'yes_price', 'no_price', 'edge_pct', 'size', 'market_type', 'slug'])
            print(f"✓ Inserted {len(arbs)} ARB events")
        except Exception as e:
            print(f"✗ Error inserting ARBs: {e}")
    
    # Insert trades
    if trades:
        data = [(t['ts'], t['market_id'], t['profit'], t['size'], True, True, t['slug']) for t in trades]
        try:
            CH_CLIENT.insert('polymarket.trades', data,
                column_names=['ts', 'market_id', 'profit', 'size', 'yes_filled', 'no_filled', 'slug'])
            print(f"✓ Inserted {len(trades)} trades")
        except Exception as e:
            print(f"✗ Error inserting trades: {e}")
    
    # Insert capital blocks
    if capital_blocks:
        data = [(c['ts'], 'CAPITAL_BLOCK', c['market_id'], json.dumps({'current': c['current'], 'needed': c['needed'], 'limit': c['limit']})) for c in capital_blocks]
        try:
            CH_CLIENT.insert('polymarket.risk_events', data,
                column_names=['ts', 'event_type', 'market_id', 'details'])
            print(f"✓ Inserted {len(capital_blocks)} capital blocks")
        except Exception as e:
            print(f"✗ Error inserting capital blocks: {e}")
    
    # Insert MLE blocks
    if mle_blocks:
        data = [(m['ts'], 'MLE_BLOCK', m['market_id'], '{}') for m in mle_blocks]
        try:
            CH_CLIENT.insert('polymarket.risk_events', data,
                column_names=['ts', 'event_type', 'market_id', 'details'])
            print(f"✓ Inserted {len(mle_blocks)} MLE blocks")
        except Exception as e:
            print(f"✗ Error inserting MLE blocks: {e}")
    
    # Insert ghost detections
    if ghosts:
        data = [(g['ts'], 'GHOST_LIQUIDITY', g['market_id'], '{}') for g in ghosts]
        try:
            CH_CLIENT.insert('polymarket.risk_events', data,
                column_names=['ts', 'event_type', 'market_id', 'details'])
            print(f"✓ Inserted {len(ghosts)} ghost detections")
        except Exception as e:
            print(f"✗ Error inserting ghosts: {e}")
    
    # Insert adverse selection events
    if adverse_events:
        # Table schema: ts, market_id, leg1_price, leg2_price_at_fill, leg2_price_at_10s, combined_at_fill, combined_at_10s, target_combined, adverse
        # We have: fill_combined (combined_at_fill), combined_10s, delta
        # leg1_price and leg2_price_at_fill are not available from log, set to 0
        # target_combined = combined_at_fill * 0.95 (assumed 5% profit margin)
        data = [(
            a['ts'], 
            a['market_id'], 
            0.0,  # leg1_price - not in log
            0.0,  # leg2_price_at_fill - not in log
            0.0,  # leg2_price_at_10s - not in log
            a['fill_combined'], 
            a['combined_10s'], 
            a['fill_combined'] * 0.95,  # target_combined (estimated)
            True  # adverse - always true since we logged it
        ) for a in adverse_events]
        try:
            CH_CLIENT.insert('polymarket.adverse_selection_events', data,
                column_names=['ts', 'market_id', 'leg1_price', 'leg2_price_at_fill', 'leg2_price_at_10s', 'combined_at_fill', 'combined_at_10s', 'target_combined', 'adverse'])
            print(f"✓ Inserted {len(adverse_events)} adverse selection events")
        except Exception as e:
            print(f"✗ Error inserting adverse events: {e}")
    
    # Update marker
    set_marker(total_lines)
    print(f"✓ Marker updated to {total_lines}")

if __name__ == "__main__":
    sync()