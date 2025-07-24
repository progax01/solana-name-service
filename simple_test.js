const {
  Connection,
  PublicKey,
  Keypair,
  Transaction,
  TransactionInstruction,
  SystemProgram,
  sendAndConfirmTransaction,
} = require('@solana/web3.js');
const fs = require('fs');

// Configuration
const PROGRAM_ID = new PublicKey('6ugEPNmbdvuxaHbAV53iGdhvepRyYrzp5oNwwiQW8PbS');
const RPC_URL = 'https://rpc.gorbchain.xyz';
const connection = new Connection(RPC_URL, 'confirmed');

// Load keypair
const keypairPath = '/home/admin/.config/solana/gor-testnet.json';
const secret = JSON.parse(fs.readFileSync(keypairPath, 'utf8'));
const payer = Keypair.fromSecretKey(new Uint8Array(secret));

console.log('Using wallet:', payer.publicKey.toString());
console.log('Program ID:', PROGRAM_ID.toString());

// Helper function to encode string with length prefix
function encodeString(str) {
  const strBytes = Buffer.from(str, 'utf8');
  const lengthBytes = Buffer.alloc(4);
  lengthBytes.writeUInt32LE(strBytes.length, 0);
  return Buffer.concat([lengthBytes, strBytes]);
}

// Helper function to derive PDA
function derivePda(username, tld) {
  return PublicKey.findProgramAddressSync(
    [Buffer.from('naming_service'), Buffer.from(username), Buffer.from(tld)],
    PROGRAM_ID
  );
}

async function testRegister() {
  console.log('\n🔍 Testing domain registration...');
  
  const username = 'test';
  const tld = '.gorbage';
  const walletAddress = payer.publicKey; // Use payer's own address
  const metadataUrl = 'https://example.com';
  const duration = 86400; // 1 day
  
  const [pda, bump] = derivePda(username, tld);
  console.log('PDA:', pda.toString());
  console.log('Bump:', bump);
  
  // Create a fee account (just use the payer for simplicity)
  const feeAccount = payer.publicKey;
  
  // Prepare instruction data
  let data = Buffer.alloc(1);
  data[0] = 0; // Register instruction
  
  data = Buffer.concat([
    data,
    encodeString(username),
    encodeString(tld),
    walletAddress.toBuffer(),
    encodeString(metadataUrl),
    Buffer.alloc(8) // duration as 8 bytes
  ]);
  
  // Write duration as little-endian 64-bit integer
  data.writeBigUInt64LE(BigInt(duration), data.length - 8);
  
  console.log('Instruction data length:', data.length);
  console.log('Instruction data (hex):', data.toString('hex'));
  
  const instruction = new TransactionInstruction({
    keys: [
      { pubkey: payer.publicKey, isSigner: true, isWritable: true },
      { pubkey: pda, isSigner: false, isWritable: true },
      { pubkey: feeAccount, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    programId: PROGRAM_ID,
    data: data,
  });
  
  const transaction = new Transaction().add(instruction);
  
  try {
    console.log('Sending transaction...');
    const signature = await sendAndConfirmTransaction(connection, transaction, [payer], {
      commitment: 'confirmed',
      preflightCommitment: 'confirmed'
    });
    console.log('✅ Registration successful!');
    console.log('Transaction signature:', signature);
    
    // Query the created account
    setTimeout(async () => {
      const accountInfo = await connection.getAccountInfo(pda);
      if (accountInfo) {
        console.log('✅ Account created successfully!');
        console.log('Account data length:', accountInfo.data.length);
        console.log('Account owner:', accountInfo.owner.toString());
        console.log('Account lamports:', accountInfo.lamports);
      } else {
        console.log('❌ Account not found after registration');
      }
    }, 2000);
    
  } catch (error) {
    console.log('❌ Registration failed:', error.message);
    if (error.logs) {
      console.log('Transaction logs:', error.logs);
    }
  }
}

async function main() {
  console.log('🚀 Simple Naming Service Test');
  console.log('==============================');
  
  // Check balance first
  const balance = await connection.getBalance(payer.publicKey);
  console.log('Wallet balance:', balance / 1e9, 'SOL');
  
  if (balance < 1e9) { // Less than 1 SOL
    console.log('❌ Insufficient balance for testing');
    return;
  }
  
  await testRegister();
}

main().catch(console.error);