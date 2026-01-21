#!/bin/bash

if [ -f .env ]; then
  export $(grep -v '^#' .env | xargs)
fi
API_PORT=${API_PORT:-3500}

# Start backend in background
echo "Starting backend..."
cd backend
./target/debug/backend > ../backend.log 2>&1 &
BACKEND_PID=$!
cd ..

# Wait for backend to be ready
echo "Waiting for backend to start on port $API_PORT..."
for i in {1..30}; do
    if curl -s http://localhost:$API_PORT/health > /dev/null; then
        echo "Backend is up!"
        break
    fi
    sleep 1
done

# Trigger backtest
echo "Triggering backtest..."
RESPONSE=$(curl -s -X POST http://localhost:$API_PORT/api/backtest/run \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "BTCUSDT",
    "start_date": "2023-01-01T00:00:00Z",
    "end_date": "2023-02-01T00:00:00Z",
    "initial_capital": 10000.0,
    "enable_ai_analysis": false
  }')

echo "Response: $RESPONSE"
BACKTEST_ID=$(echo $RESPONSE | grep -o '"backtest_id":"[^"]*"' | cut -d'"' -f4)

if [ -z "$BACKTEST_ID" ]; then
    echo "Failed to get backtest ID"
    cat backend.log
    kill $BACKEND_PID
    exit 1
fi

echo "Backtest ID: $BACKTEST_ID"

# Poll for completion
echo "Polling for results..."
for i in {1..60}; do
    RESULT=$(curl -s http://localhost:$API_PORT/api/backtest/result/$BACKTEST_ID)
    STATUS=$(echo $RESULT | grep -o '"Running"' || echo "")
    
    if [ -z "$STATUS" ]; then
        # Not running, assuming completed or failed
        echo "Backtest finished!"
        echo "Result: $RESULT"
        break
    fi
    echo "Still running..."
    sleep 1
done

# Kill backend
kill $BACKEND_PID
