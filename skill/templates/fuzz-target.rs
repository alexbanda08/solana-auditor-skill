// Fuzz harness scaffold for Solana / Anchor programs
//
// CUSTOMIZE markers below show every program-specific fill-in point.
//
// TOOL CHOICE (last-verified 2026-06):
//   - Anchor programs: use Trident (https://github.com/Ackee-Blockchain/trident)
//     Trident is a stateful, manually-guided Anchor fuzzer: it derives fuzz
//     accounts + instructions from your IDL and lets you script flow-based
//     instruction sequences with invariant checks between steps. Since 0.11 it
//     runs on its OWN native engine (TridentSVM, built on Anza's SVM API) - it no
//     longer wraps honggfuzz/AFL and is cross-platform (Linux/macOS/Windows).
//     See setup section below.
//   - Non-Anchor / custom pure-Rust functions: use honggfuzz-rs
//     (https://github.com/rust-fuzz/honggfuzz-rs)
//     This file provides the honggfuzz-rs entry point; use it to fuzz an isolated
//     math/parsing routine you can lift out of the program.
//
// REQUIREMENTS (needs cargo + tooling installed):
//
// For honggfuzz-rs (non-Anchor):
//   cargo install honggfuzz --version 0.5.60 --locked   # last-verified 2026-06
//   [dev-dependencies]
//   honggfuzz = "0.5"
//   arbitrary = { version = "1", features = ["derive"] }
//   # If the harness drives a program through litesvm, use the SAME granular
//   # solana-* crate set as templates/litesvm-harness.rs (litesvm is on the
//   # solana 3.x crate line - do NOT pull in solana-sdk 4.x):
//   litesvm                 = "0.13"
//   solana-keypair          = "3.1"
//   solana-signer           = "3"
//   solana-instruction      = "=3.2"
//   solana-transaction      = { version = "3.1", features = ["bincode"] }
//   solana-pubkey           = { version = "4", features = ["curve25519"] }
//   solana-system-interface = "3"
//   # Host-build features: solana-transaction "bincode" (new_signed_with_payer)
//   # and solana-pubkey "curve25519" (find_program_address off-chain).
//
// For Trident (Anchor programs, PREFERRED):
//   cargo install trident-cli       # trident-cli 0.12.0 stable (last-verified 2026-06;
//                                   # pairs with trident-fuzz 0.12.0 - the trident
//                                   # crates version in lockstep; 0.13.0-rc.* are
//                                   # prereleases - do not pin a prerelease)
//   trident init                    # run from workspace root
//   trident fuzz run <target>       # run the generated harness
//   NOTE: re-confirm trident-cli's anchor-lang compatibility for your release
//         (`cargo search trident-cli`); the fuzzer moves independently of the SDK.
//
// Run (honggfuzz-rs):
//   cargo hfuzz run fuzz_program
//
// Corpus output: hfuzz_workspace/fuzz_program/
// Crash reproduction: cargo hfuzz run-debug fuzz_program hfuzz_workspace/fuzz_program/CRASH_*
//
// NOTE: This file does NOT compile standalone (rustc --edition 2021 --test).
//       Build with `cargo hfuzz build` or via the Trident CLI.

#![cfg_attr(not(test), no_main)]

use arbitrary::Arbitrary;
use honggfuzz::fuzz;
use litesvm::LiteSVM;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_system_interface::program as system_program;
use solana_transaction::Transaction;

// ---------------------------------------------------------------------------
// CUSTOMIZE: compiled program ELF (built from source with `cargo build-sbf`)
// ---------------------------------------------------------------------------
const PROGRAM_ELF: &[u8] = include_bytes!(
    // CUSTOMIZE: set to your program's .so path relative to this file
    "../../../target/deploy/placeholder.so"
);

fn program_id() -> Pubkey {
    // CUSTOMIZE: paste your program's declared pubkey
    "11111111111111111111111111111111".parse().unwrap()
}

// ---------------------------------------------------------------------------
// Arbitrary input: the fuzzer generates random instances of this struct.
// Add every field that maps to an instruction argument your program accepts.
// Derive Arbitrary so honggfuzz can generate and mutate automatically.
// ---------------------------------------------------------------------------
#[derive(Debug, Arbitrary)]
struct FuzzInput {
    /// Which instruction discriminator to call (maps to your instruction set)
    instruction_index: u8,

    /// Raw argument bytes (borsh-serialized args region; fuzzer mutates freely)
    args: Vec<u8>,

    /// Whether to pass a signer for the authority account
    provide_signer: bool,

    /// Whether to pass the correct program ID for CPIs
    correct_program_id: bool,

    /// Arbitrary u64 values for numeric instruction args (amounts, fees, etc.)
    amounts: [u64; 4],

    // CUSTOMIZE: add more fields matching your program's instruction surface
}

// ---------------------------------------------------------------------------
// Instruction dispatch: map FuzzInput.instruction_index to a real instruction.
// Fill in one arm per instruction in your program.
// ---------------------------------------------------------------------------
fn build_instruction(program_id: Pubkey, input: &FuzzInput, accounts: &FuzzAccounts) -> Instruction {
    // CUSTOMIZE: IX_COUNT must equal the number of instructions in your program
    let ix_index = input.instruction_index as usize % IX_COUNT;

    match ix_index {
        0 => build_initialize_ix(program_id, input, accounts),
        1 => build_deposit_ix(program_id, input, accounts),
        2 => build_withdraw_ix(program_id, input, accounts),
        // CUSTOMIZE: add all remaining instructions
        _ => build_initialize_ix(program_id, input, accounts), // fallback
    }
}

const IX_COUNT: usize = 3; // CUSTOMIZE: set to actual instruction count

// CUSTOMIZE: replace these stub builders with real instruction construction
fn build_initialize_ix(program_id: Pubkey, input: &FuzzInput, accounts: &FuzzAccounts) -> Instruction {
    // Anchor discriminator for "initialize" (compute with sha256):
    //   echo -n "global:initialize" | sha256sum | cut -c1-16
    let discriminator: [u8; 8] = [0xaf, 0xaf, 0x6d, 0x1f, 0x0d, 0x98, 0x9b, 0xed];

    let mut data = discriminator.to_vec();
    // Truncate/pad fuzzer args to match expected instruction size
    let padded = pad_or_truncate(&input.args, 8); // CUSTOMIZE: set to actual args byte length
    data.extend_from_slice(&padded);

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(accounts.state, false),
            AccountMeta::new(accounts.payer.pubkey(), input.provide_signer),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

fn build_deposit_ix(program_id: Pubkey, input: &FuzzInput, accounts: &FuzzAccounts) -> Instruction {
    let discriminator: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02]; // CUSTOMIZE: real discriminator
    let mut data = discriminator.to_vec();
    data.extend_from_slice(&input.amounts[0].to_le_bytes());

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(accounts.state, false),
            AccountMeta::new(accounts.payer.pubkey(), true),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

fn build_withdraw_ix(program_id: Pubkey, input: &FuzzInput, accounts: &FuzzAccounts) -> Instruction {
    let discriminator: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03]; // CUSTOMIZE: real discriminator
    let mut data = discriminator.to_vec();
    data.extend_from_slice(&input.amounts[1].to_le_bytes());

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(accounts.state, false),
            AccountMeta::new(accounts.payer.pubkey(), true),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

// ---------------------------------------------------------------------------
// Pre-computed accounts used across all fuzz iterations.
// Keeping them stable (not re-derived per iteration) speeds up the harness.
// ---------------------------------------------------------------------------
struct FuzzAccounts {
    payer: Keypair,
    state: Pubkey,
    // CUSTOMIZE: add vault, token accounts, oracles, etc.
}

impl FuzzAccounts {
    fn new(program_id: Pubkey) -> Self {
        let payer = Keypair::new();
        let (state, _bump) = Pubkey::find_program_address(&[b"state"], &program_id);
        Self { payer, state }
    }
}

// ---------------------------------------------------------------------------
// Invariant checks: run after every fuzzed transaction.
// A panic here = honggfuzz records a crash = finding to triage.
// ---------------------------------------------------------------------------
fn check_invariants(svm: &LiteSVM, accounts: &FuzzAccounts) {
    // INVARIANT 1: Program-owned accounts must remain owned by the program.
    if let Some(acc) = svm.get_account(&accounts.state) {
        assert!(
            acc.owner == program_id() || acc.owner == system_program::id(),
            "INVARIANT VIOLATED: state account owner changed unexpectedly: {:?}",
            acc.owner
        );
    }

    // INVARIANT 2: Token vault balance must be >= sum of recorded user deposits.
    // CUSTOMIZE: implement by reading on-chain state and comparing to vault balance.

    // INVARIANT 3: No instruction should be able to drain the program's lamports to zero.
    // CUSTOMIZE: check program account lamports > rent-exempt minimum.

    // Add program-specific invariants here.
}

// ---------------------------------------------------------------------------
// Utility: pad or truncate a byte slice to exactly `len` bytes.
// ---------------------------------------------------------------------------
fn pad_or_truncate(src: &[u8], len: usize) -> Vec<u8> {
    let mut out = vec![0u8; len];
    let copy_len = src.len().min(len);
    out[..copy_len].copy_from_slice(&src[..copy_len]);
    out
}

// ---------------------------------------------------------------------------
// Main fuzz loop (honggfuzz-rs entry point)
// ---------------------------------------------------------------------------
fn main() {
    // Initialize the SVM and load program once; reuse across iterations.
    // NOTE: LiteSVM is not Send, so this is single-threaded.
    let mut svm = LiteSVM::new();
    // CUSTOMIZE: uncomment once PROGRAM_ELF path is set:
    // svm.add_program(program_id(), PROGRAM_ELF);

    let accounts = FuzzAccounts::new(program_id());
    svm.airdrop(&accounts.payer.pubkey(), 100_000_000_000).ok();

    // CUSTOMIZE: run any one-time initialization instruction here before the fuzz loop.

    fuzz!(|input: FuzzInput| {
        // Clone or checkpoint SVM state so each iteration starts from a known baseline.
        // litesvm 0.13 implements Clone; use it to isolate iterations:
        let mut iter_svm = svm.clone();

        let ix = build_instruction(program_id(), &input, &accounts);

        let blockhash = iter_svm.latest_blockhash();
        let signers: Vec<&Keypair> = if input.provide_signer {
            vec![&accounts.payer]
        } else {
            vec![&accounts.payer] // always need fee payer; authority varies
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&accounts.payer.pubkey()),
            &signers,
            blockhash,
        );

        // Send; we intentionally allow errors (they are expected for bad inputs).
        // We only care about panics in invariant checks, which indicate real bugs.
        let _ = iter_svm.send_transaction(tx);

        // Run invariant checks after every iteration.
        check_invariants(&iter_svm, &accounts);
    });
}

// ===========================================================================
// TRIDENT SETUP NOTES (preferred for Anchor programs) - last-verified 2026-06
// ===========================================================================
// Trident is now a multi-crate project: trident-cli 0.12.0 (stable) drives
// trident-fuzz 0.12.0 - the trident crates version in lockstep (trident-cli
// 0.12.0 depends on trident-client ^0.12.0). (0.13.0-rc.* prereleases exist;
// do not pin a prerelease.)
//
// 1. Install:  cargo install trident-cli
// 2. From workspace root:  trident init
//    Trident reads your Anchor IDL and generates:
//      trident-tests/fuzz_tests/<target>/test_fuzz.rs  <- generated harness
//      Trident.toml
// 3. Customize the FuzzInstruction impls in the generated harness to set
//    account constraints (e.g., reuse PDAs across iterations) and to script
//    flow-based instruction sequences (Trident is a stateful, guided fuzzer).
// 4. Run:  trident fuzz run <target>
// 5. Reproduce a crash:
//    trident fuzz debug <target> <path-to-crash-file>
//
// Engine: since 0.11, Trident runs on its OWN native execution engine
// (TridentSVM, built on Anza's SVM API). It does NOT wrap honggfuzz or AFL++
// anymore and is cross-platform (Linux/macOS/Windows) with consistent behavior.
//
// Key Trident concepts to customize:
//   - get_accounts(): deterministically derive or create accounts per iteration.
//   - get_data():     control the instruction argument space.
//   - check():        assert post-instruction invariants (panics = findings).
//
// For non-Anchor pure-Rust functions, the honggfuzz-rs harness above is the
// right approach; verify honggfuzz-rs's current release (0.5.60, 2026-06) first.
