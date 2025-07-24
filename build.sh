#!/bin/bash

# Build script for Solana Nameservice Program

echo "Building Solana Nameservice Program..."

# Install dependencies if needed
echo "Installing cargo-build-sbf..."
cargo install cargo-build-sbf

# Build the program
echo "Building program..."
cargo build-sbf

# Check if build was successful
if [ $? -eq 0 ]; then
    echo "✅ Build successful!"
    echo "Program binary located at: target/deploy/nameservice.so"
    echo "Program keypair located at: target/deploy/nameservice-keypair.json"
else
    echo "❌ Build failed!"
    exit 1
fi