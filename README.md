# Solana Nameservice Program

A Solana program that implements a decentralized naming service supporting usernames with top-level domains (TLDs) `.gorbage`, `.gorb`, and `.wstf`. Each domain maps to three key-value pairs: TLD, wallet address, and metadata URL.

## Features

- **Domain Registration**: Register usernames with supported TLDs
- **Domain Updates**: Update wallet address and metadata URL mappings  
- **Registration Extension**: Extend registration duration
- **Domain Closure**: Close domains and reclaim rent
- **PDA-based Storage**: Uses Program Derived Addresses for secure domain storage
- **Time-based Expiration**: Domains expire after registration period

## Supported TLDs

- `.gorbage`
- `.gorb` 
- `.wstf`

## Data Structure

Each domain record contains:
- `owner`: Solana address of the domain owner
- `expiration_time`: Unix timestamp when registration expires
- `tld`: The top-level domain
- `wallet_address`: Mapped wallet address
- `metadata_url`: URL for metadata (e.g., IPFS link)

## Instructions

### 0. Register
Register a new domain with username, TLD, wallet address, metadata URL, and duration.

### 1. Update
Update the TLD, wallet address, or metadata URL for an owned domain.

### 2. Extend
Extend the registration duration by paying additional fees.

### 3. Close
Close a domain and reclaim the rent-exempt SOL.

## Fees

- **Rent**: One-time rent-exempt amount for account storage
- **Registration Fee**: 0.001 SOL per second of registration duration

## Building

```bash
# Install dependencies
./build.sh
```

## Testing

```bash
# Run all tests
./test.sh
```

## Deployment

```bash
# Deploy to devnet (default)
./deploy.sh

# Deploy to mainnet-beta
./deploy.sh mainnet-beta

# Deploy to testnet
./deploy.sh testnet
```

# Usage Example


# CLI Usage

You can use the CLI to interact with the nameservice program. All commands are run from the project root:

```
node cli.js register <username> <tld> [walletAddress] [metadataUrl] [duration]
```

**Parameters:**
- `<username>`: The username to register (e.g., `gorblin`)
- `<tld>`: The top-level domain (e.g., `.gorbage`, `.gorb`, `.wstf`)
- `[walletAddress]` (optional): The Solana wallet address to associate with the domain. Defaults to your current wallet if omitted.
- `[metadataUrl]` (optional): Metadata URL (e.g., IPFS link). Defaults to `https://example.com` if omitted.
- `[duration]` (optional): Registration duration in seconds. Defaults to `86400` (1 day) if omitted.

**Examples:**

Register domain `gorblin.gorbage` for 1 day (default wallet, default metadata):
```
node cli.js register gorblin .gorbage
```

Register domain with custom wallet address and metadata URL for 2 days:
```
node cli.js register gorblin .gorbage 9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM "https://ipfs.io/ipfs/Qm..." 172800
```

Register domain with only a custom wallet address:
```
node cli.js register gorblin .gorbage 9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM
```

Register domain with only a custom metadata URL:
```
node cli.js register gorblin .gorbage "" "https://ipfs.io/ipfs/Qm..."
```

Register domain with all parameters (username, tld, wallet, metadata, duration):
```
node cli.js register gorblin .gorbage 9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM "https://ipfs.io/ipfs/Qm..." 259200
```

2. Update wallet address and metadata URL

3. Extend registration by 12 hours (43200 seconds)
   - Additional fee: ~0.0432 SOL

4. Close domain to reclaim rent

## Security Features

- PDA derivation prevents unauthorized access
- Owner-only updates and closures
- TLD validation against hardcoded list
- Arithmetic overflow protection
- Input validation and sanitization

## Testing Coverage

The test suite covers:
- Successful domain registration
- Duplicate registration prevention  
- Domain updates by owner
- Registration extension
- Domain closure
- Invalid TLD rejection
- Empty username rejection
- Zero duration rejection
- Non-owner operation rejection