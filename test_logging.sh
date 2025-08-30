#!/bin/bash

echo "Testing logging at different levels..."

echo -e "\n=== Test 1: RUST_LOG=error ===" 
export SECRET_KEY=test123
export RUST_LOG=error
timeout 1 cargo run 2>&1 | grep -E '(ERROR|error|INFO|info|DEBUG|debug|WARN|warn)' | head -5 || true

echo -e "\n=== Test 2: RUST_LOG=warn ===" 
export RUST_LOG=warn
timeout 1 cargo run 2>&1 | grep -E '(ERROR|error|INFO|info|DEBUG|debug|WARN|warn)' | head -5 || true

echo -e "\n=== Test 3: RUST_LOG=info ===" 
export RUST_LOG=info
timeout 1 cargo run 2>&1 | grep -E '(ERROR|error|INFO|info|DEBUG|debug|WARN|warn)' | head -5 || true

echo -e "\n=== Test 4: RUST_LOG=debug ===" 
export RUST_LOG=debug
timeout 1 cargo run 2>&1 | grep -E '(ERROR|error|INFO|info|DEBUG|debug|WARN|warn)' | head -5 || true

echo -e "\nAll logging levels tested!"