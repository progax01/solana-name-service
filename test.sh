#!/bin/bash

# Test script for Solana Nameservice Program

echo "🧪 Running tests for Solana Nameservice Program..."

# Run unit and integration tests
echo "Running Cargo tests..."
cargo test

if [ $? -eq 0 ]; then
    echo "✅ All tests passed!"
else
    echo "❌ Some tests failed!"
    exit 1
fi

echo ""
echo "Test coverage complete. Check the output above for any failures."