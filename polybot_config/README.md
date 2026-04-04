# Polybot Strategy Config Backup
# Saved: 2026-03-19 19:52 EDT

## Current Configuration

### Ports (as of last save)
- Executor: 7070
- Strategy: 7071
- Config updated to match (strategy's executor URL = http://localhost:7070)

### Running Services
```
executor-service: port 7070
strategy-service: port 7071
slot_daemon.py: sends Telegram reports every 5 min
```

### Strategy: Gabagool Directional
- Type: 5-minute up/down markets (BTC, ETH)
- Mode: Paper trading
- Refresh: 500ms
- Bankroll: $100
- Quote size: $5 USD
- Key files:
  - GabagoolDirectionalEngine.java (main trading logic)
  - GabagoolMarketDiscovery.java (market finding)
  - GabagoolConfig.java (configuration model)
  - DirectionalConfig.java (directional settings)

### Key Config Changes Made During Session
1. Strategy application-develop.yaml: executor base-url changed to http://localhost:7070
2. Executor application.yaml: port changed to 7070
3. Strategy application.yaml: no port override (uses default 8081, but command line override --server.port=7071)

### To Replicate This Setup

1. Build the services:
```bash
cd /home/aissac/polybot
mvn clean package -DskipTests
```

2. Update executor port in executor-service/src/main/resources/application.yaml:
```yaml
server:
  port: 7070
```

3. Update strategy's executor URL in strategy-service/src/main/resources/application-develop.yaml:
```yaml
executor:
  base-url: http://localhost:7070
```

4. Start services:
```bash
# Start executor on 7070
cd /home/aissac/polybot
java -Xmx1g -Xms512m -XX:+ExitOnOutOfMemoryError -XX:+UseG1GC \
  -jar executor-service/target/executor-service-0.0.1-SNAPSHOT.jar \
  --spring.profiles.active=develop --server.port=7070 &

# Start strategy on 7071
java -Xmx1g -Xms512m -XX:+ExitOnOutOfMemoryError -XX:+UseG1GC \
  -jar strategy-service/target/strategy-service-0.0.1-SNAPSHOT.jar \
  --spring.profiles.active=develop --server.port=7071 &

# Start slot daemon
python3 -u scripts/slot_daemon.py &
```

5. Verify:
```bash
# Check ports
ss -tlnp | grep -E "7070|7071"

# Check trading
docker exec polybot-clickhouse clickhouse-client --query "SELECT max(ts), count(*) FROM polybot.user_trade_enriched_v4 WHERE toDate(ts) = today()"
```

### Files in This Backup
- strategy-application.yaml: Strategy's main config
- strategy-application-develop.yaml: Strategy's dev profile (has executor URL)
- executor-application.yaml: Executor main config
- executor-application-develop.yaml: Executor dev profile
- GabagoolDirectionalEngine.java: Main trading engine
- GabagoolMarketDiscovery.java: Market discovery
- GabagoolConfig.java: Strategy config model
- DirectionalConfig.java: Directional config model
