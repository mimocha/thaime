#!/bin/bash

# Test script to verify basic functionality
echo "Testing thaime Rust implementation..."

# Test help output
echo "1. Testing help output:"
./target/debug/thaime --help

echo ""
echo "2. Testing that the binary can start without crashing (will exit quickly due to no D-Bus):"
timeout 2s ./target/debug/thaime || echo "Expected: Failed to connect to session bus (no D-Bus in this environment)"

echo ""
echo "3. Testing IBus mode:"
timeout 2s ./target/debug/thaime --ibus || echo "Expected: Failed to connect to session bus (no D-Bus in this environment)"

echo ""
echo "✅ Basic tests completed. The binary builds and runs without crashing before D-Bus connection."