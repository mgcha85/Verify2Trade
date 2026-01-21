#!/bin/bash

# Ensure cleanup on exit
cleanup() {
    echo "Stopping containers..."
    podman-compose down
}
trap cleanup EXIT

echo "Building and starting services with podman-compose..."
podman-compose up --build -d

# Wait for backend to be ready
echo "Waiting for backend to start (localhost:3000)..."
for i in {1..60}; do
    if curl -s http://localhost:3000/health > /dev/null; then
        echo "Backend is up!"
        break
    fi
    echo "Waiting... ($i/60)"
    sleep 2
done

# Check if backend is actually up
if ! curl -s http://localhost:3000/health > /dev/null; then
    echo "Backend failed to start."
    podman-compose logs backend
    exit 1
fi

# Trigger backtest
echo "Triggering backtest..."
# Using a 1-month period for quick testing: Jan 2023
RESPONSE=$(curl -s -X POST http://localhost:3000/api/backtest/run \
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
    podman-compose logs backend
    exit 1
fi

echo "Backtest ID: $BACKTEST_ID"

# Poll for completion
echo "Polling for results..."
for i in {1..60}; do
    RESULT=$(curl -s http://localhost:3000/api/backtest/result/$BACKTEST_ID)
    STATUS=$(echo $RESULT | grep -o '"Running"' || echo "")
    
    if [ -z "$STATUS" ]; then
        # Not running, assuming completed or failed
        echo "Backtest finished!"
        # Pretty print result if jq is installed, else just print
        if command -v jq &> /dev/null; then
            echo $RESULT | jq .
        else
            echo "Result: $RESULT"
        fi
        break
    fi
    echo "Still running..."
    sleep 1
done

echo "Test completed. Containers will be stopped."
