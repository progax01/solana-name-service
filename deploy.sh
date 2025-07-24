#!/bin/bash

# Deployment script for Solana Nameservice Program

set -e

PROGRAM_NAME="nameservice"
PROGRAM_SO="target/deploy/${PROGRAM_NAME}.so"
PROGRAM_KEYPAIR="target/deploy/${PROGRAM_NAME}-keypair.json"

# Check if program binary exists
if [ ! -f "$PROGRAM_SO" ]; then
    echo "❌ Program binary not found at $PROGRAM_SO"
    echo "Please run ./build.sh first"
    exit 1
fi

# Get network (default to devnet if not specified)
NETWORK=${1:-devnet}

echo "🚀 Deploying Solana Nameservice Program to $NETWORK..."

# Set Solana config to the specified network
echo "Setting Solana config to $NETWORK..."
solana config set --url $NETWORK

# Check balance
BALANCE=$(solana balance)
echo "Current balance: $BALANCE"

# Deploy the program
echo "Deploying program..."
PROGRAM_ID=$(solana program deploy $PROGRAM_SO --program-id $PROGRAM_KEYPAIR)

if [ $? -eq 0 ]; then
    echo "✅ Deployment successful!"
    echo "Program ID: $PROGRAM_ID"
    echo "Network: $NETWORK"
    echo ""
    echo "You can now interact with your program using the Program ID above."
else
    echo "❌ Deployment failed!"
    exit 1
fi