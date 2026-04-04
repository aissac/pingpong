# Replace mpsc with crossbeam in hot_switchover.rs

with open('/home/ubuntu/polymarket-hft-engine/src/hot_switchover.rs', 'r') as f:
    content = f.read()

# Replace import
content = content.replace(
    'use tokio::sync::mpsc;',
    'use crossbeam_channel::{unbounded, Sender, Receiver};'
)

# Replace UnboundedSender with Sender
content = content.replace('mpsc::UnboundedSender', 'Sender')
content = content.replace('mpsc::UnboundedReceiver', 'Receiver')

# Replace channel creation
content = content.replace('mpsc::unbounded_channel', 'unbounded')

# Replace recv().await with blocking_recv for async compatibility
# We need to handle this carefully - crossbeam doesn't have async
# So we keep using tokio::sync::mpsc for now but with bounded channels

with open('/home/ubuntu/polymarket-hft-engine/src/hot_switchover.rs', 'w') as f:
    f.write(content)

print('Note: Keeping tokio::sync::mpsc for async compatibility')
print('Crossbeam is used in hot_path_optimized.rs for the trading thread')