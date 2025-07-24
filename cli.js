#!/usr/bin/env node

const {
  Connection,
  PublicKey,
  Keypair,
  Transaction,
  TransactionInstruction,
  SystemProgram,
} = require('@solana/web3.js');
const fs = require('fs');

// Configuration
const PROGRAM_ID = new PublicKey('6ugEPNmbdvuxaHbAV53iGdhvepRyYrzp5oNwwiQW8PbS');
const RPC_ENDPOINT = 'https://rpc.gorbchain.xyz';
const WS_ENDPOINT = 'wss://rpc.gorbchain.xyz/ws/';
const connection = new Connection(RPC_ENDPOINT, {
  commitment: 'confirmed',
  wsEndpoint: WS_ENDPOINT,
  disableRetryOnRateLimit: false,
});

// Load keypair
const keypairPath = '/home/admin/.config/solana/gor-testnet.json';
const secret = JSON.parse(fs.readFileSync(keypairPath, 'utf8'));
const payer = Keypair.fromSecretKey(new Uint8Array(secret));

// Helper functions
function encodeString(str) {
  const strBytes = Buffer.from(str, 'utf8');
  const lengthBytes = Buffer.alloc(4);
  lengthBytes.writeUInt32LE(strBytes.length, 0);
  return Buffer.concat([lengthBytes, strBytes]);
}

function derivePda(username, tld) {
  return PublicKey.findProgramAddressSync(
    [Buffer.from('naming_service'), Buffer.from(username), Buffer.from(tld)],
    PROGRAM_ID
  );
}

async function sendTransaction(instruction) {
  const transaction = new Transaction().add(instruction);
  const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
  transaction.recentBlockhash = blockhash;
  transaction.feePayer = payer.publicKey;
  transaction.sign(payer);
  
  const signature = await connection.sendRawTransaction(transaction.serialize(), {
    skipPreflight: false,
    preflightCommitment: 'confirmed'
  });
  
  const confirmation = await connection.confirmTransaction({
    signature: signature,
    blockhash: blockhash,
    lastValidBlockHeight: lastValidBlockHeight
  });
  
  return { signature, confirmation };
}

// Command functions
async function register(username, tld, walletAddress, metadataUrl, duration) {
  console.log(`🔍 Registering domain: ${username}${tld}`);
  
  const [pda] = derivePda(username, tld);
  const targetWallet = walletAddress ? new PublicKey(walletAddress) : payer.publicKey;
  
  // Check if domain already exists
  const existingAccount = await connection.getAccountInfo(pda);
  if (existingAccount) {
    console.log('❌ Domain already registered');
    return false;
  }
  
  let data = Buffer.alloc(1);
  data[0] = 0; // Register instruction
  
  data = Buffer.concat([
    data,
    encodeString(username),
    encodeString(tld),
    targetWallet.toBuffer(),
    encodeString(metadataUrl || 'https://example.com'),
    Buffer.alloc(8)
  ]);
  data.writeBigUInt64LE(BigInt(duration || 86400), data.length - 8);
  
  const instruction = new TransactionInstruction({
    keys: [
      { pubkey: payer.publicKey, isSigner: true, isWritable: true },
      { pubkey: pda, isSigner: false, isWritable: true },
      { pubkey: payer.publicKey, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    programId: PROGRAM_ID,
    data: data,
  });
  
  try {
    const { signature } = await sendTransaction(instruction);
    console.log('✅ Domain registered successfully!');
    console.log('Transaction:', signature);
    console.log('PDA:', pda.toString());
    return true;
  } catch (error) {
    console.log('❌ Registration failed:', error.message);
    return false;
  }
}

async function query(username, tld) {
  console.log(`🔍 Querying domain: ${username}${tld}`);
  
  const [pda] = derivePda(username, tld);
  
  try {
    const accountInfo = await connection.getAccountInfo(pda);
    
    if (!accountInfo) {
      console.log('❌ Domain not found');
      return null;
    }
    
    console.log('✅ Domain found!');
    console.log('PDA:', pda.toString());
    console.log('Owner Program:', accountInfo.owner.toString());
    console.log('Data Length:', accountInfo.data.length);
    console.log('Lamports:', accountInfo.lamports);
    
    // Basic data parsing (first 32 bytes is owner pubkey)
    if (accountInfo.data.length >= 32) {
      const ownerBytes = accountInfo.data.slice(0, 32);
      const owner = new PublicKey(ownerBytes);
      console.log('Domain Owner:', owner.toString());
    }
    
    return accountInfo;
  } catch (error) {
    console.log('❌ Query failed:', error.message);
    return null;
  }
}

async function update(username, tld, newWalletAddress, newMetadataUrl) {
  console.log(`🔄 Updating domain: ${username}${tld}`);
  
  const [pda] = derivePda(username, tld);
  const targetWallet = new PublicKey(newWalletAddress);
  
  let data = Buffer.alloc(1);
  data[0] = 1; // Update instruction
  
  data = Buffer.concat([
    data,
    encodeString(username),
    encodeString(tld),
    targetWallet.toBuffer(),
    encodeString(newMetadataUrl)
  ]);
  
  const instruction = new TransactionInstruction({
    keys: [
      { pubkey: payer.publicKey, isSigner: true, isWritable: true },
      { pubkey: pda, isSigner: false, isWritable: true },
    ],
    programId: PROGRAM_ID,
    data: data,
  });
  
  try {
    const { signature } = await sendTransaction(instruction);
    console.log('✅ Domain updated successfully!');
    console.log('Transaction:', signature);
    return true;
  } catch (error) {
    console.log('❌ Update failed:', error.message);
    return false;
  }
}

async function extend(username, tld, additionalDuration) {
  console.log(`⏰ Extending domain: ${username}${tld}`);
  
  const [pda] = derivePda(username, tld);
  
  let data = Buffer.alloc(1);
  data[0] = 2; // Extend instruction
  
  data = Buffer.concat([
    data,
    encodeString(username),
    encodeString(tld),
    Buffer.alloc(8)
  ]);
  data.writeBigUInt64LE(BigInt(additionalDuration), data.length - 8);
  
  const instruction = new TransactionInstruction({
    keys: [
      { pubkey: payer.publicKey, isSigner: true, isWritable: true },
      { pubkey: pda, isSigner: false, isWritable: true },
      { pubkey: payer.publicKey, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    programId: PROGRAM_ID,
    data: data,
  });
  
  try {
    const { signature } = await sendTransaction(instruction);
    console.log('✅ Domain extended successfully!');
    console.log('Transaction:', signature);
    return true;
  } catch (error) {
    console.log('❌ Extension failed:', error.message);
    return false;
  }
}

async function close(username, tld) {
  console.log(`🗑️ Closing domain: ${username}${tld}`);
  
  const [pda] = derivePda(username, tld);
  
  let data = Buffer.alloc(1);
  data[0] = 3; // Close instruction
  
  data = Buffer.concat([
    data,
    encodeString(username),
    encodeString(tld)
  ]);
  
  const instruction = new TransactionInstruction({
    keys: [
      { pubkey: payer.publicKey, isSigner: true, isWritable: true },
      { pubkey: pda, isSigner: false, isWritable: true },
    ],
    programId: PROGRAM_ID,
    data: data,
  });
  
  try {
    const { signature } = await sendTransaction(instruction);
    console.log('✅ Domain closed successfully!');
    console.log('Transaction:', signature);
    return true;
  } catch (error) {
    console.log('❌ Close failed:', error.message);
    return false;
  }
}

async function resolveDomain(username, tld) {
  const [pda] = derivePda(username, tld);
  
  try {
    const accountInfo = await connection.getAccountInfo(pda);
    if (!accountInfo || accountInfo.data.length < 32) {
      return null;
    }
    
    // Parse the wallet address from domain data (first 32 bytes after owner)
    const ownerBytes = accountInfo.data.slice(0, 32);
    const owner = new PublicKey(ownerBytes);
    
    // The wallet address is stored after owner + expiration time + tld
    // Let's parse it more carefully
    let offset = 32; // Skip owner (32 bytes)
    offset += 8; // Skip expiration time (8 bytes)
    
    // Skip TLD string (4 bytes length + string)
    const tldLength = accountInfo.data.readUInt32LE(offset);
    offset += 4 + tldLength;
    
    // Next 32 bytes should be wallet address
    if (offset + 32 <= accountInfo.data.length) {
      const walletBytes = accountInfo.data.slice(offset, offset + 32);
      const walletAddress = new PublicKey(walletBytes);
      return walletAddress;
    }
    
    return owner; // Fallback to owner
  } catch (error) {
    console.log('❌ Domain resolution failed:', error.message);
    return null;
  }
}

async function transferToUser(fromDomain, toDomain, amount) {
  console.log(`💸 Transferring ${amount} SOL from ${fromDomain} to ${toDomain}`);
  
  // Parse domain names
  const fromParts = fromDomain.split('.');
  const toParts = toDomain.split('.');
  
  if (fromParts.length !== 2 || toParts.length !== 2) {
    console.log('❌ Invalid domain format. Use: username.tld');
    return false;
  }
  
  const [fromUsername, fromTldPart] = fromParts;
  const [toUsername, toTldPart] = toParts;
  const fromTld = '.' + fromTldPart;
  const toTld = '.' + toTldPart;
  
  // Resolve domain addresses
  console.log(`🔍 Resolving ${fromDomain}...`);
  const fromAddress = await resolveDomain(fromUsername, fromTld);
  if (!fromAddress) {
    console.log(`❌ Could not resolve ${fromDomain}`);
    return false;
  }
  
  console.log(`🔍 Resolving ${toDomain}...`);
  const toAddress = await resolveDomain(toUsername, toTld);
  if (!toAddress) {
    console.log(`❌ Could not resolve ${toDomain}`);
    return false;
  }
  
  console.log(`From address: ${fromAddress.toString()}`);
  console.log(`To address: ${toAddress.toString()}`);
  
  // Check if we own the from domain (or if it's set to our wallet)
  if (!fromAddress.equals(payer.publicKey)) {
    console.log(`❌ Cannot transfer from ${fromDomain} - not owned by current wallet`);
    console.log(`Domain points to: ${fromAddress.toString()}`);
    console.log(`Current wallet: ${payer.publicKey.toString()}`);
    return false;
  }
  
  // Convert amount to lamports
  const lamports = Math.floor(amount * 1e9);
  
  // Check balance
  const balance = await connection.getBalance(payer.publicKey);
  if (balance < lamports) {
    console.log(`❌ Insufficient balance. Have: ${(balance / 1e9).toFixed(6)} SOL, Need: ${amount} SOL`);
    return false;
  }
  
  // Create transfer instruction
  const transferInstruction = SystemProgram.transfer({
    fromPubkey: payer.publicKey,
    toPubkey: toAddress,
    lamports: lamports,
  });
  
  try {
    const { signature } = await sendTransaction(transferInstruction);
    console.log('✅ Transfer successful!');
    console.log('Transaction:', signature);
    console.log(`Transferred ${amount} SOL from ${fromDomain} to ${toDomain}`);
    
    // Show updated balances
    const newFromBalance = await connection.getBalance(fromAddress);
    const newToBalance = await connection.getBalance(toAddress);
    console.log(`${fromDomain} balance: ${(newFromBalance / 1e9).toFixed(6)} SOL`);
    console.log(`${toDomain} balance: ${(newToBalance / 1e9).toFixed(6)} SOL`);
    
    return true;
  } catch (error) {
    console.log('❌ Transfer failed:', error.message);
    return false;
  }
}

// CLI interface
async function main() {
  const args = process.argv.slice(2);
  const command = args[0];
  
  if (!command) {
    console.log(`
🚀 Solana Naming Service CLI
============================

Usage: node cli.js <command> [arguments]

Commands:
  register <username> <tld> [walletAddress] [metadataUrl] [duration]
    - Register a new domain
    - TLDs: .gorbage, .gorb, .wstf
    - Duration in seconds (default: 86400)
    
  query <username> <tld>
    - Query domain information
    
  update <username> <tld> <newWalletAddress> <newMetadataUrl>
    - Update domain wallet address and metadata
    
  extend <username> <tld> <additionalDuration>
    - Extend domain registration
    
  close <username> <tld>
    - Close domain and reclaim rent
    
  transfer <fromDomain> <toDomain> <amount>
    - Transfer SOL between domain addresses
    - Example: anurag.gorbage rachit.gorbage 1.5

Examples:
  node cli.js register myname .gorbage
  node cli.js query myname .gorbage
  node cli.js update myname .gorbage 9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM "https://ipfs.io/new"
  node cli.js extend myname .gorbage 86400
  node cli.js close myname .gorbage
  node cli.js transfer anurag.gorbage rachit.gorbage 1.5

Current wallet: ${payer.publicKey.toString()}
Program ID: ${PROGRAM_ID.toString()}
`);
    return;
  }
  
  console.log(`Using wallet: ${payer.publicKey.toString()}`);
  
  const balance = await connection.getBalance(payer.publicKey);
  console.log(`Balance: ${(balance / 1e9).toFixed(6)} SOL\n`);
  
  switch (command) {
    case 'register':
      if (args.length < 3) {
        console.log('Usage: register <username> <tld> [walletAddress] [metadataUrl] [duration]');
        return;
      }
      await register(args[1], args[2], args[3], args[4], parseInt(args[5]));
      break;
      
    case 'query':
      if (args.length < 3) {
        console.log('Usage: query <username> <tld>');
        return;
      }
      await query(args[1], args[2]);
      break;
      
    case 'update':
      if (args.length < 5) {
        console.log('Usage: update <username> <tld> <newWalletAddress> <newMetadataUrl>');
        return;
      }
      await update(args[1], args[2], args[3], args[4]);
      break;
      
    case 'extend':
      if (args.length < 4) {
        console.log('Usage: extend <username> <tld> <additionalDuration>');
        return;
      }
      await extend(args[1], args[2], parseInt(args[3]));
      break;
      
    case 'close':
      if (args.length < 3) {
        console.log('Usage: close <username> <tld>');
        return;
      }
      await close(args[1], args[2]);
      break;
      
    case 'transfer':
      if (args.length < 4) {
        console.log('Usage: transfer <fromDomain> <toDomain> <amount>');
        console.log('Example: transfer anurag.gorbage rachit.gorbage 1.5');
        return;
      }
      await transferToUser(args[1], args[2], parseFloat(args[3]));
      break;
      
    default:
      console.log('❌ Unknown command:', command);
      console.log('Run without arguments to see usage help.');
  }
}

if (require.main === module) {
  main().catch(console.error);
}