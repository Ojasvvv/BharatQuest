#!/bin/bash
HOST="https://bharatquest.onrender.com"
API_KEY="ilovesushmita"

echo "Running 5 test executions..."

# 1. Success 1
curl -s -X POST "$HOST/v1/execute" -H "X-API-Key: $API_KEY" -H "Content-Type: application/json" -d '{"request_id":"live-1","language":"javascript","code":"const x = 5; x * 2;","timeout_ms":5000,"memory_limit_mb":64}' > /dev/null

# 2. Success 2
curl -s -X POST "$HOST/v1/execute" -H "X-API-Key: $API_KEY" -H "Content-Type: application/json" -d '{"request_id":"live-2","language":"javascript","code":"Math.random();","timeout_ms":5000,"memory_limit_mb":64}' > /dev/null

# 3. RuntimeError
curl -s -X POST "$HOST/v1/execute" -H "X-API-Key: $API_KEY" -H "Content-Type: application/json" -d '{"request_id":"live-3","language":"javascript","code":"undefined_method()","timeout_ms":5000,"memory_limit_mb":64}' > /dev/null

# 4. OutOfFuel (infinite loop)
curl -s -X POST "$HOST/v1/execute" -H "X-API-Key: $API_KEY" -H "Content-Type: application/json" -d '{"request_id":"live-4","language":"javascript","code":"while(true){}","timeout_ms":5000,"memory_limit_mb":64}' > /dev/null

# 5. Rejected / Timeout
curl -s -X POST "$HOST/v1/execute" -H "X-API-Key: $API_KEY" -H "Content-Type: application/json" -d '{"request_id":"live-5","language":"javascript","code":"Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 1000);","timeout_ms":100,"memory_limit_mb":64}' > /dev/null

echo "Waiting for DB flush..."
sleep 2

echo "Fetching History..."
curl -s -X GET "$HOST/v1/metrics/history" -H "X-API-Key: $API_KEY" | jq .
