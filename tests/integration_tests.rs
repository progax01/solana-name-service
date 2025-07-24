use borsh::BorshDeserialize;
use nameservice::{NameRecord, process_instruction};
use solana_program::{
    clock::Clock,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    rent::Rent,
    system_program,
};
use solana_program_test::*;
use solana_sdk::{
    account::Account,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

const PROGRAM_ID: Pubkey = Pubkey::new_from_array([1u8; 32]);

#[tokio::test]
async fn test_register_domain_success() {
    let program_test = ProgramTest::new("nameservice", PROGRAM_ID, processor!(process_instruction));
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let username = "gorblin";
    let tld = ".gorbage"; 
    let wallet_address = Keypair::new().pubkey();
    let metadata_url = "https://ipfs.io/ipfs/QmHash";
    let duration = 86400u64; // 1 day

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(
        &[b"naming_service", username.as_bytes(), tld.as_bytes()],
        &PROGRAM_ID,
    );

    // Create fee account
    let fee_account = Keypair::new();

    // Prepare instruction data
    let mut instruction_data = vec![0u8]; // Register instruction
    instruction_data.extend_from_slice(&(username.len() as u32).to_le_bytes());
    instruction_data.extend_from_slice(username.as_bytes());
    instruction_data.extend_from_slice(&(tld.len() as u32).to_le_bytes());
    instruction_data.extend_from_slice(tld.as_bytes());
    instruction_data.extend_from_slice(&wallet_address.to_bytes());
    instruction_data.extend_from_slice(&(metadata_url.len() as u32).to_le_bytes());
    instruction_data.extend_from_slice(metadata_url.as_bytes());
    instruction_data.extend_from_slice(&duration.to_le_bytes());

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &instruction_data,
        vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(pda, false),
            AccountMeta::new(fee_account.pubkey(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    banks_client.process_transaction(transaction).await.unwrap();

    // Verify PDA account was created with correct data
    let pda_account = banks_client.get_account(pda).await.unwrap().unwrap();
    let name_record = NameRecord::try_from_slice(&pda_account.data).unwrap();
    
    assert_eq!(name_record.owner, payer.pubkey());
    assert_eq!(name_record.tld, tld);
    assert_eq!(name_record.wallet_address, wallet_address);
    assert_eq!(name_record.metadata_url, metadata_url);
    assert!(name_record.expiration_time > 0);
}

#[tokio::test]
async fn test_register_already_taken_domain() {
    let program_test = ProgramTest::new("nameservice", PROGRAM_ID, processor!(process_instruction));
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let username = "gorblin";
    let tld = ".gorbage";
    let wallet_address = Keypair::new().pubkey();
    let metadata_url = "https://ipfs.io/ipfs/QmHash";
    let duration = 86400u64;

    // First registration
    let (pda, _) = Pubkey::find_program_address(
        &[b"naming_service", username.as_bytes(), tld.as_bytes()],
        &PROGRAM_ID,
    );

    let fee_account = Keypair::new();

    let mut instruction_data = vec![0u8];
    instruction_data.extend_from_slice(&(username.len() as u32).to_le_bytes());
    instruction_data.extend_from_slice(username.as_bytes());
    instruction_data.extend_from_slice(&(tld.len() as u32).to_le_bytes());
    instruction_data.extend_from_slice(tld.as_bytes());
    instruction_data.extend_from_slice(&wallet_address.to_bytes());
    instruction_data.extend_from_slice(&(metadata_url.len() as u32).to_le_bytes());
    instruction_data.extend_from_slice(metadata_url.as_bytes());
    instruction_data.extend_from_slice(&duration.to_le_bytes());

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &instruction_data,
        vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(pda, false),
            AccountMeta::new(fee_account.pubkey(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    banks_client.process_transaction(transaction).await.unwrap();

    // Try to register the same domain again
    let another_user = Keypair::new();
    let wallet_address2 = Keypair::new().pubkey();
    
    let mut instruction_data2 = vec![0u8];
    instruction_data2.extend_from_slice(&(username.len() as u32).to_le_bytes());
    instruction_data2.extend_from_slice(username.as_bytes());
    instruction_data2.extend_from_slice(&(tld.len() as u32).to_le_bytes());
    instruction_data2.extend_from_slice(tld.as_bytes());
    instruction_data2.extend_from_slice(&wallet_address2.to_bytes());
    instruction_data2.extend_from_slice(&(metadata_url.len() as u32).to_le_bytes());
    instruction_data2.extend_from_slice(metadata_url.as_bytes());
    instruction_data2.extend_from_slice(&duration.to_le_bytes());

    let instruction2 = Instruction::new_with_bytes(
        PROGRAM_ID,
        &instruction_data2,
        vec![
            AccountMeta::new(another_user.pubkey(), true),
            AccountMeta::new(pda, false),
            AccountMeta::new(fee_account.pubkey(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    );

    let transaction2 = Transaction::new_signed_with_payer(
        &[instruction2],
        Some(&another_user.pubkey()),
        &[&another_user],
        recent_blockhash,
    );

    // This should fail
    let result = banks_client.process_transaction(transaction2).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_update_domain() {
    let program_test = ProgramTest::new("nameservice", PROGRAM_ID, processor!(process_instruction));
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let username = "gorblin";
    let tld = ".gorb";
    let wallet_address = Keypair::new().pubkey();
    let metadata_url = "https://ipfs.io/ipfs/QmHash";
    let duration = 86400u64;

    // First register the domain
    let (pda, _) = Pubkey::find_program_address(
        &[b"naming_service", username.as_bytes(), tld.as_bytes()],
        &PROGRAM_ID,
    );

    let fee_account = Keypair::new();

    let mut register_data = vec![0u8];
    register_data.extend_from_slice(&(username.len() as u32).to_le_bytes());
    register_data.extend_from_slice(username.as_bytes());
    register_data.extend_from_slice(&(tld.len() as u32).to_le_bytes());
    register_data.extend_from_slice(tld.as_bytes());
    register_data.extend_from_slice(&wallet_address.to_bytes());
    register_data.extend_from_slice(&(metadata_url.len() as u32).to_le_bytes());
    register_data.extend_from_slice(metadata_url.as_bytes());
    register_data.extend_from_slice(&duration.to_le_bytes());

    let register_instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &register_data,
        vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(pda, false),
            AccountMeta::new(fee_account.pubkey(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    );

    let register_transaction = Transaction::new_signed_with_payer(
        &[register_instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    banks_client.process_transaction(register_transaction).await.unwrap();

    // Now update the domain
    let new_wallet_address = Keypair::new().pubkey();
    let new_metadata_url = "https://ipfs.io/ipfs/QmNewHash";

    let mut update_data = vec![1u8]; // Update instruction
    update_data.extend_from_slice(&(username.len() as u32).to_le_bytes());
    update_data.extend_from_slice(username.as_bytes());
    update_data.extend_from_slice(&(tld.len() as u32).to_le_bytes());
    update_data.extend_from_slice(tld.as_bytes());
    update_data.extend_from_slice(&new_wallet_address.to_bytes());
    update_data.extend_from_slice(&(new_metadata_url.len() as u32).to_le_bytes());
    update_data.extend_from_slice(new_metadata_url.as_bytes());

    let update_instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &update_data,
        vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(pda, false),
        ],
    );

    let update_transaction = Transaction::new_signed_with_payer(
        &[update_instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    banks_client.process_transaction(update_transaction).await.unwrap();

    // Verify the update
    let pda_account = banks_client.get_account(pda).await.unwrap().unwrap();
    let name_record = NameRecord::try_from_slice(&pda_account.data).unwrap();
    
    assert_eq!(name_record.wallet_address, new_wallet_address);
    assert_eq!(name_record.metadata_url, new_metadata_url);
    assert_eq!(name_record.owner, payer.pubkey()); // Owner should remain the same
}

#[tokio::test]
async fn test_extend_registration() {
    let program_test = ProgramTest::new("nameservice", PROGRAM_ID, processor!(process_instruction));
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let username = "gorblin";
    let tld = ".wstf";
    let wallet_address = Keypair::new().pubkey();
    let metadata_url = "https://ipfs.io/ipfs/QmHash";
    let duration = 86400u64;

    // Register domain first
    let (pda, _) = Pubkey::find_program_address(
        &[b"naming_service", username.as_bytes(), tld.as_bytes()],
        &PROGRAM_ID,
    );

    let fee_account = Keypair::new();

    let mut register_data = vec![0u8];
    register_data.extend_from_slice(&(username.len() as u32).to_le_bytes());
    register_data.extend_from_slice(username.as_bytes());
    register_data.extend_from_slice(&(tld.len() as u32).to_le_bytes());
    register_data.extend_from_slice(tld.as_bytes());
    register_data.extend_from_slice(&wallet_address.to_bytes());
    register_data.extend_from_slice(&(metadata_url.len() as u32).to_le_bytes());
    register_data.extend_from_slice(metadata_url.as_bytes());
    register_data.extend_from_slice(&duration.to_le_bytes());

    let register_instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &register_data,
        vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(pda, false),
            AccountMeta::new(fee_account.pubkey(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    );

    let register_transaction = Transaction::new_signed_with_payer(
        &[register_instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    banks_client.process_transaction(register_transaction).await.unwrap();

    // Get initial expiration time
    let pda_account = banks_client.get_account(pda).await.unwrap().unwrap();
    let initial_record = NameRecord::try_from_slice(&pda_account.data).unwrap();
    let initial_expiration = initial_record.expiration_time;

    // Extend registration
    let additional_duration = 43200u64; // 12 hours

    let mut extend_data = vec![2u8]; // Extend instruction
    extend_data.extend_from_slice(&(username.len() as u32).to_le_bytes());
    extend_data.extend_from_slice(username.as_bytes());
    extend_data.extend_from_slice(&(tld.len() as u32).to_le_bytes());
    extend_data.extend_from_slice(tld.as_bytes());
    extend_data.extend_from_slice(&additional_duration.to_le_bytes());

    let extend_instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &extend_data,
        vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(pda, false),
            AccountMeta::new(fee_account.pubkey(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    );

    let extend_transaction = Transaction::new_signed_with_payer(
        &[extend_instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    banks_client.process_transaction(extend_transaction).await.unwrap();

    // Verify extension
    let pda_account = banks_client.get_account(pda).await.unwrap().unwrap();
    let extended_record = NameRecord::try_from_slice(&pda_account.data).unwrap();
    
    assert_eq!(
        extended_record.expiration_time,
        initial_expiration + additional_duration as i64
    );
}

#[tokio::test]
async fn test_close_domain() {
    let program_test = ProgramTest::new("nameservice", PROGRAM_ID, processor!(process_instruction));
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let username = "gorblin";
    let tld = ".gorbage";
    let wallet_address = Keypair::new().pubkey();
    let metadata_url = "https://ipfs.io/ipfs/QmHash";
    let duration = 86400u64;

    // Register domain first
    let (pda, _) = Pubkey::find_program_address(
        &[b"naming_service", username.as_bytes(), tld.as_bytes()],
        &PROGRAM_ID,
    );

    let fee_account = Keypair::new();

    let mut register_data = vec![0u8];
    register_data.extend_from_slice(&(username.len() as u32).to_le_bytes());
    register_data.extend_from_slice(username.as_bytes());
    register_data.extend_from_slice(&(tld.len() as u32).to_le_bytes());
    register_data.extend_from_slice(tld.as_bytes());
    register_data.extend_from_slice(&wallet_address.to_bytes());
    register_data.extend_from_slice(&(metadata_url.len() as u32).to_le_bytes());
    register_data.extend_from_slice(metadata_url.as_bytes());
    register_data.extend_from_slice(&duration.to_le_bytes());

    let register_instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &register_data,
        vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(pda, false),
            AccountMeta::new(fee_account.pubkey(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    );

    let register_transaction = Transaction::new_signed_with_payer(
        &[register_instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    banks_client.process_transaction(register_transaction).await.unwrap();

    // Get initial balance
    let initial_balance = banks_client.get_balance(payer.pubkey()).await.unwrap();

    // Close domain
    let mut close_data = vec![3u8]; // Close instruction
    close_data.extend_from_slice(&(username.len() as u32).to_le_bytes());
    close_data.extend_from_slice(username.as_bytes());
    close_data.extend_from_slice(&(tld.len() as u32).to_le_bytes());
    close_data.extend_from_slice(tld.as_bytes());

    let close_instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &close_data,
        vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(pda, false),
        ],
    );

    let close_transaction = Transaction::new_signed_with_payer(
        &[close_instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    banks_client.process_transaction(close_transaction).await.unwrap();

    // Verify account is closed
    let pda_account = banks_client.get_account(pda).await.unwrap();
    assert!(pda_account.is_none());

    // Verify balance increased (rent refunded)
    let final_balance = banks_client.get_balance(payer.pubkey()).await.unwrap();
    assert!(final_balance > initial_balance);
}

#[tokio::test]
async fn test_invalid_tld() {
    let program_test = ProgramTest::new("nameservice", PROGRAM_ID, processor!(process_instruction));
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let username = "gorblin";
    let invalid_tld = ".invalid";
    let wallet_address = Keypair::new().pubkey();
    let metadata_url = "https://ipfs.io/ipfs/QmHash";
    let duration = 86400u64;

    let (pda, _) = Pubkey::find_program_address(
        &[b"naming_service", username.as_bytes(), invalid_tld.as_bytes()],
        &PROGRAM_ID,
    );

    let fee_account = Keypair::new();

    let mut instruction_data = vec![0u8];
    instruction_data.extend_from_slice(&(username.len() as u32).to_le_bytes());
    instruction_data.extend_from_slice(username.as_bytes());
    instruction_data.extend_from_slice(&(invalid_tld.len() as u32).to_le_bytes());
    instruction_data.extend_from_slice(invalid_tld.as_bytes());
    instruction_data.extend_from_slice(&wallet_address.to_bytes());
    instruction_data.extend_from_slice(&(metadata_url.len() as u32).to_le_bytes());
    instruction_data.extend_from_slice(metadata_url.as_bytes());
    instruction_data.extend_from_slice(&duration.to_le_bytes());

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &instruction_data,
        vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(pda, false),
            AccountMeta::new(fee_account.pubkey(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    // This should fail due to invalid TLD
    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_empty_username() {
    let program_test = ProgramTest::new("nameservice", PROGRAM_ID, processor!(process_instruction));
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let username = "";
    let tld = ".gorbage";
    let wallet_address = Keypair::new().pubkey();
    let metadata_url = "https://ipfs.io/ipfs/QmHash";
    let duration = 86400u64;

    let (pda, _) = Pubkey::find_program_address(
        &[b"naming_service", username.as_bytes(), tld.as_bytes()],
        &PROGRAM_ID,
    );

    let fee_account = Keypair::new();

    let mut instruction_data = vec![0u8];
    instruction_data.extend_from_slice(&(username.len() as u32).to_le_bytes());
    instruction_data.extend_from_slice(username.as_bytes());
    instruction_data.extend_from_slice(&(tld.len() as u32).to_le_bytes());
    instruction_data.extend_from_slice(tld.as_bytes());
    instruction_data.extend_from_slice(&wallet_address.to_bytes());
    instruction_data.extend_from_slice(&(metadata_url.len() as u32).to_le_bytes());
    instruction_data.extend_from_slice(metadata_url.as_bytes());
    instruction_data.extend_from_slice(&duration.to_le_bytes());

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &instruction_data,
        vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(pda, false),
            AccountMeta::new(fee_account.pubkey(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    // This should fail due to empty username
    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_zero_duration() {
    let program_test = ProgramTest::new("nameservice", PROGRAM_ID, processor!(process_instruction));
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let username = "gorblin";
    let tld = ".gorbage";
    let wallet_address = Keypair::new().pubkey();
    let metadata_url = "https://ipfs.io/ipfs/QmHash";
    let duration = 0u64; // Invalid duration

    let (pda, _) = Pubkey::find_program_address(
        &[b"naming_service", username.as_bytes(), tld.as_bytes()],
        &PROGRAM_ID,
    );

    let fee_account = Keypair::new();

    let mut instruction_data = vec![0u8];
    instruction_data.extend_from_slice(&(username.len() as u32).to_le_bytes());
    instruction_data.extend_from_slice(username.as_bytes());
    instruction_data.extend_from_slice(&(tld.len() as u32).to_le_bytes());
    instruction_data.extend_from_slice(tld.as_bytes());
    instruction_data.extend_from_slice(&wallet_address.to_bytes());
    instruction_data.extend_from_slice(&(metadata_url.len() as u32).to_le_bytes());
    instruction_data.extend_from_slice(metadata_url.as_bytes());
    instruction_data.extend_from_slice(&duration.to_le_bytes());

    let instruction = Instruction::new_with_bytes(
        PROGRAM_ID,
        &instruction_data,
        vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(pda, false),
            AccountMeta::new(fee_account.pubkey(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    // This should fail due to zero duration
    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_err());
}