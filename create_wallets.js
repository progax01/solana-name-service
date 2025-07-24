const { Keypair } = require('@solana/web3.js');
const fs = require('fs');

// Create a new wallet for rachit
const rachitWallet = Keypair.generate();

console.log('🔑 Created new wallet for rachit:');
console.log('Public Key:', rachitWallet.publicKey.toString());
console.log('Private Key (first 10 bytes):', Array.from(rachitWallet.secretKey.slice(0, 10)));

// Save the keypair to a file
const keypairData = Array.from(rachitWallet.secretKey);
fs.writeFileSync('/home/admin/Documents/nameservice/rachit-wallet.json', JSON.stringify(keypairData));

console.log('💾 Saved rachit wallet to: /home/admin/Documents/nameservice/rachit-wallet.json');
console.log('🔍 You can now update rachit.gorbage to point to this wallet');