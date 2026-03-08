//! # Solana Counter Program
//!
//! A simple on-chain Solana program that stores a counter value
//! and supports increment, decrement, and reset instructions.
//!
//! This demonstrates:
//! - Solana program structure (entrypoint, processor, state, instructions)
//! - Account data serialization with Borsh
//! - Custom error types
//! - Instruction parsing

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use thiserror::Error;

// ─────────────────────────────────────────────
//  State — stored on-chain in account data
// ─────────────────────────────────────────────

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct CounterAccount {
    pub count: u64,
    pub authority: Pubkey,  // Only this key can reset the counter
}

impl CounterAccount {
    pub const SIZE: usize = 8 + 32; // u64 (8 bytes) + Pubkey (32 bytes)
}

// ─────────────────────────────────────────────
//  Instructions
// ─────────────────────────────────────────────

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum CounterInstruction {
    /// Initialize the counter to 0
    Initialize,
    /// Increment counter by 1
    Increment,
    /// Decrement counter by 1 (min 0)
    Decrement,
    /// Reset counter to 0 (authority only)
    Reset,
    /// Add a specific amount to the counter
    AddAmount { amount: u64 },
}

// ─────────────────────────────────────────────
//  Custom Errors
// ─────────────────────────────────────────────

#[derive(Error, Debug, Copy, Clone)]
pub enum CounterError {
    #[error("Counter would underflow (already at 0)")]
    Underflow,
    #[error("Unauthorized: only the authority can reset")]
    Unauthorized,
    #[error("Counter account is already initialized")]
    AlreadyInitialized,
    #[error("Invalid instruction data")]
    InvalidInstruction,
}

impl From<CounterError> for ProgramError {
    fn from(e: CounterError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

// ─────────────────────────────────────────────
//  Entrypoint
// ─────────────────────────────────────────────

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = CounterInstruction::try_from_slice(instruction_data)
        .map_err(|_| CounterError::InvalidInstruction)?;

    match instruction {
        CounterInstruction::Initialize     => initialize(program_id, accounts),
        CounterInstruction::Increment      => increment(accounts),
        CounterInstruction::Decrement      => decrement(accounts),
        CounterInstruction::Reset          => reset(accounts),
        CounterInstruction::AddAmount { amount } => add_amount(accounts, amount),
    }
}

// ─────────────────────────────────────────────
//  Handlers
// ─────────────────────────────────────────────

fn initialize(_program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let counter_account = next_account_info(account_iter)?;
    let authority = next_account_info(account_iter)?;

    let state = CounterAccount {
        count: 0,
        authority: *authority.key,
    };

    state.serialize(&mut &mut counter_account.data.borrow_mut()[..])?;
    msg!("Counter initialized! Authority: {}", authority.key);
    Ok(())
}

fn increment(accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let counter_account = next_account_info(account_iter)?;

    let mut state = CounterAccount::try_from_slice(&counter_account.data.borrow())?;
    state.count = state.count.saturating_add(1);
    state.serialize(&mut &mut counter_account.data.borrow_mut()[..])?;

    msg!("Counter incremented to: {}", state.count);
    Ok(())
}

fn decrement(accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let counter_account = next_account_info(account_iter)?;

    let mut state = CounterAccount::try_from_slice(&counter_account.data.borrow())?;

    if state.count == 0 {
        return Err(CounterError::Underflow.into());
    }

    state.count -= 1;
    state.serialize(&mut &mut counter_account.data.borrow_mut()[..])?;

    msg!("Counter decremented to: {}", state.count);
    Ok(())
}

fn reset(accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let counter_account = next_account_info(account_iter)?;
    let authority = next_account_info(account_iter)?;

    let mut state = CounterAccount::try_from_slice(&counter_account.data.borrow())?;

    // Check authority
    if state.authority != *authority.key {
        return Err(CounterError::Unauthorized.into());
    }

    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    state.count = 0;
    state.serialize(&mut &mut counter_account.data.borrow_mut()[..])?;

    msg!("Counter reset to 0 by authority: {}", authority.key);
    Ok(())
}

fn add_amount(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let counter_account = next_account_info(account_iter)?;

    let mut state = CounterAccount::try_from_slice(&counter_account.data.borrow())?;
    state.count = state.count.saturating_add(amount);
    state.serialize(&mut &mut counter_account.data.borrow_mut()[..])?;

    msg!("Added {} to counter. New count: {}", amount, state.count);
    Ok(())
}
