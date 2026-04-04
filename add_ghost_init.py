#!/usr/bin/env python3
import re

# Read the file
with open('/home/ubuntu/polymarket-hft-engine/src/hot_switchover.rs', 'r') as f:
    content = f.read()

# Add ghost_simulator initialization after max_daily_loss
old_init = '            max_daily_loss: 50.0, // Max $50 daily loss before pause\n        }\n    }'
new_init = '''            max_daily_loss: 50.0, // Max $50 daily loss before pause
            ghost_simulator: Arc::new(crate::ghost_simulator::GhostSimulator::new()),
        }
    }'''

content = content.replace(old_init, new_init)

# Write back
with open('/home/ubuntu/polymarket-hft-engine/src/hot_switchover.rs', 'w') as f:
    f.write(content)

print('Added ghost_simulator initialization')