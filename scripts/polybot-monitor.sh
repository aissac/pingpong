#!/bin/bash
# Polybot Service Monitor
# Checks all services and restarts if needed

LOG_FILE="/home/aissac/polybot/logs/monitor.log"
DATE=$(date '+%Y-%m-%d %H:%M:%S')

# Function to check service health
check_service() {
    local port=$1
    local name=$2
    local health=$(curl -s http://localhost:$port/actuator/health 2>/dev/null | grep -o '"status":"UP"')
    
    if [ -z "$health" ]; then
        echo "[$DATE] ❌ $name (port $port) is DOWN" | tee -a $LOG_FILE
        return 1
    else
        echo "[$DATE] ✅ $name (port $port) is UP" | tee -a $LOG_FILE
        return 0
    fi
}

# Check all services
echo "[$DATE] === Polybot Health Check ===" | tee -a $LOG_FILE

FAILED=0
check_service 8080 "Executor" || FAILED=1
check_service 8081 "Strategy" || FAILED=1
check_service 8082 "Analytics" || FAILED=1
check_service 8083 "Ingestor" || FAILED=1
check_service 8084 "Infrastructure" || FAILED=1

# Check continuous slot trader
if pgrep -f "continuous-slot-trader.py" > /dev/null; then
    echo "[$DATE] ✅ Continuous slot trader is RUNNING" | tee -a $LOG_FILE
else
    echo "[$DATE] ❌ Continuous slot trader is DOWN" | tee -a $LOG_FILE
    FAILED=1
fi

# If any service failed, attempt restart
if [ $FAILED -eq 1 ]; then
    echo "[$DATE] ⚠️ Some services failed. Attempting restart..." | tee -a $LOG_FILE
    cd /home/aissac/polybot
    ./stop-all-services.sh 2>/dev/null
    sleep 5
    ./start-all-services.sh
    sleep 15
    nohup python3 scripts/continuous-slot-trader.py > logs/continuous-slot-trader.log 2>&1 &
    echo "[$DATE] 🔄 Services restarted" | tee -a $LOG_FILE
fi

echo "[$DATE] === Check Complete ===" | tee -a $LOG_FILE
echo "" | tee -a $LOG_FILE
