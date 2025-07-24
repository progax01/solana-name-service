use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};
use borsh::{BorshDeserialize, BorshSerialize};

// Supported TLDs
const VALID_TLDS: [&str; 3] = [".gorbage", ".gorb", ".wstf"];
const LAMPORTS_PER_SOL: u64 = 1_000_000_000;
const REGISTRATION_FEE_PER_SECOND: u64 = LAMPORTS_PER_SOL / 1000; // 0.001 SOL per second

// Define the NameRecord struct
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct NameRecord {
    pub owner: Pubkey,         // Owner's Solana address
    pub expiration_time: i64,  // Unix timestamp when registration expires
    pub tld: String,          // Top-level domain (e.g., .gorbage)
    pub wallet_address: Pubkey, // Mapped wallet address
    pub metadata_url: String,  // Metadata URL (e.g., IPFS link)
}

// Declare the program entrypoint
entrypoint!(process_instruction);

// Main instruction processing function
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction_type = instruction_data
        .get(0)
        .ok_or(ProgramError::InvalidInstructionData)?;
    match *instruction_type {
        0 => register(program_id, accounts, &instruction_data[1..]),
        1 => update(program_id, accounts, &instruction_data[1..]),
        2 => extend(program_id, accounts, &instruction_data[1..]),
        3 => close(program_id, accounts, &instruction_data[1..]),
        _ => {
            msg!("Invalid instruction type");
            Err(ProgramError::InvalidInstructionData)
        }
    }
}

// Register a new domain
fn register(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let user = next_account_info(accounts_iter)?;           // Signer account
    let pda_account = next_account_info(accounts_iter)?;    // PDA for the domain
    let fee_account = next_account_info(accounts_iter)?;    // Program's fee account
    let system_program = next_account_info(accounts_iter)?; // System program

    // Get current time
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    // Parse instruction data: username, tld, wallet_address, metadata_url, duration
    let (username, rest) = parse_string(instruction_data)?;
    let (tld, rest) = parse_string(rest)?;
    
    if rest.len() < 32 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let (wallet_address_bytes, rest) = rest.split_at(32);
    let wallet_address = Pubkey::new_from_array(
        wallet_address_bytes
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    );
    
    let (metadata_url, rest) = parse_string(rest)?;
    
    if rest.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let duration = u64::from_le_bytes(
        rest[..8]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    );

    // Validate inputs
    if username.is_empty() || duration == 0 {
        msg!("Username cannot be empty and duration must be > 0");
        return Err(ProgramError::InvalidInstructionData);
    }

    // Validate TLD
    if !VALID_TLDS.contains(&tld.as_str()) {
        msg!("Invalid TLD: {}", tld);
        return Err(ProgramError::InvalidInstructionData);
    }

    // Derive PDA with seeds ["naming_service", username, tld]
    let (pda, bump_seed) = Pubkey::find_program_address(
        &[b"naming_service", username.as_bytes(), tld.as_bytes()],
        program_id,
    );
    if pda != *pda_account.key {
        msg!("Invalid PDA for domain: {}.{}", username, tld);
        return Err(ProgramError::InvalidAccountData);
    }

    // Check availability
    let is_new_account = pda_account.data_is_empty();
    if !is_new_account {
        let record = NameRecord::try_from_slice(&pda_account.data.borrow())?;
        if current_time < record.expiration_time {
            msg!("Domain {}.{} is already taken and not expired", username, tld);
            return Err(ProgramError::AccountAlreadyInitialized);
        }
    }

    // Create name record
    let name_record = NameRecord {
        owner: *user.key,
        expiration_time: current_time + duration as i64,
        tld: tld.clone(),
        wallet_address,
        metadata_url,
    };

    // Calculate rent and registration fee
    let account_size = name_record.try_to_vec()?.len();
    let rent = Rent::get()?;
    let rent_exempt_amount = rent.minimum_balance(account_size);
    let registration_fee = duration
        .checked_mul(REGISTRATION_FEE_PER_SECOND)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    if is_new_account {
        // Create account directly using system program create_account instruction
        let seeds = &[
            b"naming_service",
            username.as_bytes(),
            tld.as_bytes(),
            &[bump_seed],
        ];
        
        invoke_signed(
            &system_instruction::create_account(
                user.key,
                pda_account.key,
                rent_exempt_amount,
                account_size as u64,
                program_id,
            ),
            &[user.clone(), pda_account.clone(), system_program.clone()],
            &[seeds],
        )?;
    }

    // Transfer registration fee
    invoke(
        &system_instruction::transfer(user.key, fee_account.key, registration_fee),
        &[user.clone(), fee_account.clone(), system_program.clone()],
    )?;

    // Serialize and write data
    name_record.serialize(&mut &mut pda_account.data.borrow_mut()[..])?;
    msg!("Registered domain: {}.{}", username, tld);
    Ok(())
}

// Update domain mappings
fn update(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let user = next_account_info(accounts_iter)?;         // Signer account
    let pda_account = next_account_info(accounts_iter)?;  // PDA for the domain

    // Get current time
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    // Parse instruction data
    let (username, rest) = parse_string(instruction_data)?;
    let (tld, rest) = parse_string(rest)?;
    
    if rest.len() < 32 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let (wallet_address_bytes, rest) = rest.split_at(32);
    let wallet_address = Pubkey::new_from_array(
        wallet_address_bytes
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    );
    let (metadata_url, _) = parse_string(rest)?;

    // Validate TLD
    if !VALID_TLDS.contains(&tld.as_str()) {
        msg!("Invalid TLD: {}", tld);
        return Err(ProgramError::InvalidInstructionData);
    }

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(
        &[b"naming_service", username.as_bytes(), tld.as_bytes()],
        program_id,
    );
    if pda != *pda_account.key {
        msg!("Invalid PDA for domain: {}.{}", username, tld);
        return Err(ProgramError::InvalidAccountData);
    }

    // Check ownership and expiration
    let mut name_record = NameRecord::try_from_slice(&pda_account.data.borrow())?;
    if name_record.owner != *user.key {
        msg!("Not authorized for domain: {}.{}", username, tld);
        return Err(ProgramError::InvalidAccountData);
    }
    
    if current_time >= name_record.expiration_time {
        msg!("Registration expired for domain: {}.{}", username, tld);
        return Err(ProgramError::InvalidAccountData);
    }

    // Update fields
    name_record.tld = tld.clone();
    name_record.wallet_address = wallet_address;
    name_record.metadata_url = metadata_url;
    name_record.serialize(&mut &mut pda_account.data.borrow_mut()[..])?;
    msg!("Updated domain: {}.{}", username, tld);
    Ok(())
}

// Extend registration duration
fn extend(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let user = next_account_info(accounts_iter)?;           // Signer account
    let pda_account = next_account_info(accounts_iter)?;    // PDA for the domain
    let fee_account = next_account_info(accounts_iter)?;    // Fee account
    let system_program = next_account_info(accounts_iter)?; // System program

    // Get current time
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    // Parse instruction data
    let (username, rest) = parse_string(instruction_data)?;
    let (tld, rest) = parse_string(rest)?;
    
    if rest.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let additional_duration = u64::from_le_bytes(
        rest[..8]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    );

    if additional_duration == 0 {
        msg!("Additional duration must be > 0");
        return Err(ProgramError::InvalidInstructionData);
    }

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(
        &[b"naming_service", username.as_bytes(), tld.as_bytes()],
        program_id,
    );
    if pda != *pda_account.key {
        msg!("Invalid PDA for domain: {}.{}", username, tld);
        return Err(ProgramError::InvalidAccountData);
    }

    // Check ownership
    let mut name_record = NameRecord::try_from_slice(&pda_account.data.borrow())?;
    if name_record.owner != *user.key {
        msg!("Not authorized to extend domain: {}.{}", username, tld);
        return Err(ProgramError::InvalidAccountData);
    }

    // Calculate and transfer fee
    let additional_fee = additional_duration
        .checked_mul(REGISTRATION_FEE_PER_SECOND)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    invoke(
        &system_instruction::transfer(user.key, fee_account.key, additional_fee),
        &[user.clone(), fee_account.clone(), system_program.clone()],
    )?;

    // Extend expiration
    if current_time < name_record.expiration_time {
        name_record.expiration_time = name_record.expiration_time
            .checked_add(additional_duration as i64)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    } else {
        name_record.expiration_time = current_time
            .checked_add(additional_duration as i64)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }
    
    name_record.serialize(&mut &mut pda_account.data.borrow_mut()[..])?;
    msg!("Extended registration for domain: {}.{}", username, tld);
    Ok(())
}

// Close a domain
fn close(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let user = next_account_info(accounts_iter)?;        // Signer account
    let pda_account = next_account_info(accounts_iter)?; // PDA for the domain

    // Parse instruction data
    let (username, rest) = parse_string(instruction_data)?;
    let (tld, _) = parse_string(rest)?;

    // Derive PDA
    let (pda, _) = Pubkey::find_program_address(
        &[b"naming_service", username.as_bytes(), tld.as_bytes()],
        program_id,
    );
    if pda != *pda_account.key {
        msg!("Invalid PDA for domain: {}.{}", username, tld);
        return Err(ProgramError::InvalidAccountData);
    }

    // Check ownership
    let name_record = NameRecord::try_from_slice(&pda_account.data.borrow())?;
    if name_record.owner != *user.key {
        msg!("Not authorized to close domain: {}.{}", username, tld);
        return Err(ProgramError::InvalidAccountData);
    }

    // Refund lamports to user
    let lamports = pda_account.lamports();
    **pda_account.lamports.borrow_mut() = 0;
    **user.lamports.borrow_mut() = user.lamports()
        .checked_add(lamports)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // Clear account data
    pda_account.realloc(0, false)?;
    msg!("Closed domain: {}.{}", username, tld);
    Ok(())
}

// Helper function to parse length-prefixed strings
fn parse_string(data: &[u8]) -> Result<(String, &[u8]), ProgramError> {
    if data.len() < 4 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let len = u32::from_le_bytes(
        data[..4]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?
    ) as usize;
    
    if data.len() < 4 + len {
        return Err(ProgramError::InvalidInstructionData);
    }
    
    let (str_data, rest) = data[4..].split_at(len);
    let s = String::from_utf8(str_data.to_vec())
        .map_err(|_| ProgramError::InvalidInstructionData)?;
    Ok((s, rest))
}

use std::convert::TryInto;