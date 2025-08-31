#!/bin/bash

# Test script for discovery refresh features

echo "Testing LIFX API Server Discovery Features"
echo "=========================================="

# Set test environment variables
export SECRET_KEY="test_secret_key_123"
export DISCOVERY_REFRESH_INTERVAL="60"  # 60 seconds for testing
export AUTO_DISCOVERY_ENABLED="true"
export RUST_LOG="info"

echo ""
echo "Configuration:"
echo "- Discovery refresh interval: $DISCOVERY_REFRESH_INTERVAL seconds"
echo "- Auto discovery enabled: $AUTO_DISCOVERY_ENABLED"
echo ""

# Start the server in the background
echo "Starting LIFX API server..."
cargo run &
SERVER_PID=$!

# Wait for server to start
sleep 5

echo ""
echo "Testing API endpoints:"
echo "----------------------"

# Test manual discovery endpoint
echo ""
echo "1. Testing POST /v1/discover (manual discovery):"
curl -X POST "http://localhost:8000/v1/discover" \
     -H "Authorization: Bearer $SECRET_KEY" \
     -H "Content-Type: application/json" | jq .

# Test discovery metrics endpoint
echo ""
echo "2. Testing GET /v1/discover/metrics:"
curl -X GET "http://localhost:8000/v1/discover/metrics" \
     -H "Authorization: Bearer $SECRET_KEY" | jq .

# Wait for auto-discovery to trigger (if interval is short enough)
echo ""
echo "3. Waiting 10 seconds to see auto-discovery logs..."
sleep 10

# Get metrics again to see if auto-discovery occurred
echo ""
echo "4. Getting metrics again after wait:"
curl -X GET "http://localhost:8000/v1/discover/metrics" \
     -H "Authorization: Bearer $SECRET_KEY" | jq .

echo ""
echo "5. Testing with auto-discovery disabled:"
# Kill the server
kill $SERVER_PID
wait $SERVER_PID 2>/dev/null

# Restart with auto-discovery disabled
export AUTO_DISCOVERY_ENABLED="false"
echo "Restarting server with AUTO_DISCOVERY_ENABLED=false..."
cargo run &
SERVER_PID=$!
sleep 5

echo ""
echo "6. Getting metrics with auto-discovery disabled:"
curl -X GET "http://localhost:8000/v1/discover/metrics" \
     -H "Authorization: Bearer $SECRET_KEY" | jq .

# Cleanup
echo ""
echo "Cleaning up..."
kill $SERVER_PID
wait $SERVER_PID 2>/dev/null

echo ""
echo "Test completed!"