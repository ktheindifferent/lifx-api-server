#!/bin/bash

# Security test script for LIFX API Server authentication
# This script performs comprehensive security testing of the authentication system

set -e

echo "==================================="
echo "LIFX API Server Security Test Suite"
echo "==================================="

# Configuration
SERVER_URL="http://localhost:8000"
TEST_SECRET="test_secret_key_123"
ENDPOINT="/v1/lights/all"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print test results
print_result() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✓${NC} $2"
    else
        echo -e "${RED}✗${NC} $2"
    fi
}

# Function to test authentication
test_auth() {
    local description="$1"
    local auth_header="$2"
    local expected_code="$3"
    
    if [ -z "$auth_header" ]; then
        response=$(curl -s -o /dev/null -w "%{http_code}" "$SERVER_URL$ENDPOINT" 2>/dev/null || true)
    else
        response=$(curl -s -o /dev/null -w "%{http_code}" -H "$auth_header" "$SERVER_URL$ENDPOINT" 2>/dev/null || true)
    fi
    
    if [ "$response" = "$expected_code" ]; then
        print_result 0 "$description (Expected: $expected_code, Got: $response)"
        return 0
    else
        print_result 1 "$description (Expected: $expected_code, Got: $response)"
        return 1
    fi
}

# Check if server is running
echo -e "\n${YELLOW}Checking if server is running...${NC}"
if ! curl -s -f "$SERVER_URL" > /dev/null 2>&1; then
    echo -e "${RED}Server is not running on $SERVER_URL${NC}"
    echo "Please start the server with: SECRET_KEY=$TEST_SECRET cargo run"
    exit 1
fi
echo -e "${GREEN}Server is running${NC}"

# Test 1: Missing Authorization Header
echo -e "\n${YELLOW}Test 1: Missing Authorization Header${NC}"
test_auth "Request without auth header should return 401" "" "401"

# Test 2: Invalid Token
echo -e "\n${YELLOW}Test 2: Invalid Token${NC}"
test_auth "Request with wrong token should return 401" "Authorization: Bearer wrong_token" "401"

# Test 3: Valid Token (if server is running with TEST_SECRET)
echo -e "\n${YELLOW}Test 3: Valid Token${NC}"
echo "Note: This test assumes server is running with SECRET_KEY=$TEST_SECRET"
test_auth "Request with valid token should succeed" "Authorization: Bearer $TEST_SECRET" "200" || true

# Test 4: Malformed Headers
echo -e "\n${YELLOW}Test 4: Malformed Authorization Headers${NC}"
test_auth "Bearer without token" "Authorization: Bearer" "401"
test_auth "Basic auth instead of Bearer" "Authorization: Basic dGVzdDp0ZXN0" "401"
test_auth "Token without Bearer prefix" "Authorization: $TEST_SECRET" "401"
test_auth "Lowercase bearer" "Authorization: bearer $TEST_SECRET" "401"
test_auth "Extra spaces" "Authorization:  Bearer  $TEST_SECRET" "401"

# Test 5: Rate Limiting
echo -e "\n${YELLOW}Test 5: Rate Limiting (5 attempts max)${NC}"
echo "Making multiple failed auth attempts..."
for i in {1..7}; do
    response=$(curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer wrong" "$SERVER_URL$ENDPOINT" 2>/dev/null || true)
    if [ $i -le 5 ]; then
        if [ "$response" = "401" ]; then
            echo -e "  Attempt $i: ${GREEN}401 (as expected)${NC}"
        else
            echo -e "  Attempt $i: ${RED}$response (expected 401)${NC}"
        fi
    else
        if [ "$response" = "429" ]; then
            echo -e "  Attempt $i: ${GREEN}429 (rate limited as expected)${NC}"
        else
            echo -e "  Attempt $i: ${RED}$response (expected 429)${NC}"
        fi
    fi
done

# Test 6: Response Headers
echo -e "\n${YELLOW}Test 6: Security Response Headers${NC}"
echo "Checking 401 response headers..."
headers=$(curl -s -I "$SERVER_URL$ENDPOINT" 2>/dev/null | grep -i "www-authenticate" || true)
if [ -n "$headers" ]; then
    print_result 0 "WWW-Authenticate header present in 401 response"
    echo "  $headers"
else
    print_result 1 "WWW-Authenticate header missing in 401 response"
fi

echo "Checking 429 response headers..."
# Make requests to trigger rate limiting
for i in {1..6}; do
    curl -s -o /dev/null -H "Authorization: Bearer wrong" "$SERVER_URL$ENDPOINT" 2>/dev/null || true
done
headers=$(curl -s -I -H "Authorization: Bearer wrong" "$SERVER_URL$ENDPOINT" 2>/dev/null | grep -i "retry-after" || true)
if [ -n "$headers" ]; then
    print_result 0 "Retry-After header present in 429 response"
    echo "  $headers"
else
    print_result 1 "Retry-After header missing in 429 response"
fi

echo -e "\n${YELLOW}==================================="
echo -e "Security Testing Complete"
echo -e "===================================${NC}"

echo -e "\n${YELLOW}Recommendations:${NC}"
echo "1. Ensure all endpoints require authentication"
echo "2. Monitor rate limiting effectiveness"
echo "3. Log failed authentication attempts"
echo "4. Consider implementing token expiration"
echo "5. Use HTTPS in production"
echo "6. Implement request signing for additional security"
echo "7. Add IP allowlisting for critical operations"