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

console.log('Using wallet:', payer.publicKey.toString());
console.log('Program ID:', PROGRAM_ID.toString());

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

async function testRegisterDomain() {
  try {
    console.log('\n🔍 Testing domain registration...');
    
    let username = 'test';
    const tld = '.gorbage';
    const walletAddress = payer.publicKey;
    const metadataUrl = 'https://example.com';
    const duration = 86400; // 1 day
    
    let [pda, bump] = derivePda(username, tld);
    console.log(`PDA: ${pda.toString()}, Bump: ${bump}`);
    
    // Check if account already exists
    const existingAccount = await connection.getAccountInfo(pda);
    if (existingAccount) {
      console.log('⚠️ Account already exists, trying different username...');
      username = 'test' + Date.now().toString().slice(-4);
      [pda, bump] = derivePda(username, tld);
      console.log(`New username: ${username}, New PDA: ${pda.toString()}`);
    }
    
    // Prepare instruction data for registration
    let data = Buffer.alloc(1);
    data[0] = 0; // Register instruction
    
    data = Buffer.concat([
      data,
      encodeString(username),
      encodeString(tld),
      walletAddress.toBuffer(),
      encodeString(metadataUrl),
      Buffer.alloc(8) // duration
    ]);
    data.writeBigUInt64LE(BigInt(duration), data.length - 8);
    
    console.log('Instruction data prepared, length:', data.length);
    
    const instruction = new TransactionInstruction({
      keys: [
        { pubkey: payer.publicKey, isSigner: true, isWritable: true },
        { pubkey: pda, isSigner: false, isWritable: true },
        { pubkey: payer.publicKey, isSigner: false, isWritable: true }, // fee account (using payer)
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      programId: PROGRAM_ID,
      data: data,
    });
    
    const transaction = new Transaction().add(instruction);
    const { blockhash } = await connection.getLatestBlockhash();
    transaction.recentBlockhash = blockhash;
    transaction.feePayer = payer.publicKey;
    
    // Sign and send transaction
    console.log('Signing transaction...');
    transaction.sign(payer);
    
    console.log('Sending transaction...');
    const rawTransaction = transaction.serialize();
    const signature = await connection.sendRawTransaction(rawTransaction, {
      skipPreflight: false,
      preflightCommitment: 'confirmed'
    });
    
    console.log('Transaction sent, signature:', signature);
    
    // Wait for confirmation
    console.log('Waiting for confirmation...');
    const confirmation = await connection.confirmTransaction({
      signature: signature,
      blockhash: blockhash,
      lastValidBlockHeight: (await connection.getLatestBlockhash()).lastValidBlockHeight
    });
    
    if (confirmation.value.err) {
      console.log('❌ Transaction failed:', confirmation.value.err);
    } else {
      console.log('✅ Transaction confirmed!');
      
      // Check the created account
      const accountInfo = await connection.getAccountInfo(pda);
      if (accountInfo) {
        console.log('✅ Domain registered successfully!');
        console.log('Account data length:', accountInfo.data.length);
        console.log('Account owner:', accountInfo.owner.toString());
        console.log('Account lamports:', accountInfo.lamports);
      }
    }
    
    return { success: true, signature, pda };
    
  } catch (error) {
    console.log('❌ Error:', error.message);
    return { success: false, error: error.message };
  }
}

async function testQueryDomain(username = 'test', tld = '.gorbage') {
  try {
    console.log(`\n🔍 Querying domain: ${username}${tld}`);
    
    const [pda] = derivePda(username, tld);
    console.log('PDA:', pda.toString());
    
    const accountInfo = await connection.getAccountInfo(pda);
    
    if (!accountInfo) {
      console.log('❌ Domain not found');
      return { success: false, error: 'Domain not found' };
    }
    
    console.log('✅ Domain found!');
    console.log('Owner:', accountInfo.owner.toString());
    console.log('Data length:', accountInfo.data.length);
    console.log('Lamports:', accountInfo.lamports);
    console.log('First 50 bytes (hex):', accountInfo.data.slice(0, 50).toString('hex'));
    
    return { success: true, accountInfo };
    
  } catch (error) {
    console.log('❌ Query failed:', error.message);
    return { success: false, error: error.message };
  }
}

async function main() {
  console.log('🚀 Direct Naming Service Test');
  console.log('==============================');
  
  // Check balance
  const balance = await connection.getBalance(payer.publicKey);
  console.log('Wallet balance:', (balance / 1e9).toFixed(6), 'SOL');
  
  if (balance < 0.1 * 1e9) {
    console.log('❌ Insufficient balance for testing');
    return;
  }
  
  // Test registration
  const registerResult = await testRegisterDomain();
  
  if (registerResult.success) {
    // Test querying
    await testQueryDomain();
  }
  
  console.log('\n✅ Testing complete!');
}

main().catch(console.error);