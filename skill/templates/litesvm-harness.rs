// litesvm PoC exploit / invariant test harness
// Tool: litesvm 0.12.0 + anchor-lang 1.0.2
//
// PURPOSE: Demonstrate a finding in an isolated, reproducible environment.
//          Replace every TODO block with program-specific values before running.
//
// REQUIREMENTS (needs cargo):
//   [dev-dependencies]
//   litesvm = "0.12"
//   anchor-lang = "1.0"
//   solana-sdk = "4.0"
//
// Run:
//   cargo test --test litesvm_poc -- --nocapture
//
// NOTE: This file does NOT compile standalone (rustc --edition 2021 --test)
//       because it depends on crates. Build with `cargo test`.

use litesvm::LiteSVM;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};

// ---------------------------------------------------------------------------
// CUSTOMIZE: Replace with your program's ELF path (built from source)
// ---------------------------------------------------------------------------
const PROGRAM_ELF: &[u8] = include_bytes!(
    // Example: "../../../target/deploy/my_program.so"
    // TODO: set path to compiled .so
    "../../../target/deploy/placeholder.so"
);

// ---------------------------------------------------------------------------
// CUSTOMIZE: Replace with the program's declared pubkey
// ---------------------------------------------------------------------------
fn program_id() -> Pubkey {
    // TODO: paste your program ID
    "11111111111111111111111111111111".parse().unwrap()
}

// ---------------------------------------------------------------------------
// Helper: fund a keypair with lamports via LiteSVM's airdrop equivalent
// ---------------------------------------------------------------------------
fn fund(svm: &mut LiteSVM, key: &Pubkey, lamports: u64) {
    svm.airdrop(key, lamports).expect("airdrop failed");
}

// ---------------------------------------------------------------------------
// Helper: build and send a transaction; return the result
// ---------------------------------------------------------------------------
fn send_tx(
    svm: &mut LiteSVM,
    ixs: &[Instruction],
    signers: &[&Keypair],
) -> litesvm::types::TransactionResult {
    let payer = signers[0];
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(ixs, Some(&payer.pubkey()), signers, blockhash);
    svm.send_transaction(tx)
}

// ===========================================================================
// TEMPLATE FINDING: Missing signer check -> unauthorized state mutation
//
// Vulnerability class: missing_signer_check (see vuln-classes.md)
// Severity: Critical (example; set per your actual finding)
// Description:
//   The `update_config` instruction does not verify that the `authority`
//   account signed the transaction. Any account can pass an arbitrary pubkey
//   as `authority` and mutate program state without authorization.
// ===========================================================================
#[cfg(test)]
mod poc_missing_signer_check {
    use super::*;

    // TODO: replace with the actual discriminator for `update_config`
    // Anchor discriminator = sha256("global:update_config")[..8]
    // Compute offline: echo -n "global:update_config" | sha256sum | cut -c1-16
    const IX_DISCRIMINATOR: [u8; 8] = [0xde, 0xad, 0xbe, 0xef, 0x00, 0x00, 0x00, 0x01];

    fn build_update_config_ix(
        program_id: Pubkey,
        config_account: Pubkey,
        attacker: Pubkey,
        // TODO: add any extra accounts your instruction requires
    ) -> Instruction {
        // Build the instruction data:
        //   [discriminator (8 bytes)] [serialized args (borsh)]
        // TODO: serialize your actual instruction arguments via borsh
        let mut data = IX_DISCRIMINATOR.to_vec();
        // Example extra arg: a new authority pubkey to install
        data.extend_from_slice(attacker.as_ref());

        Instruction {
            program_id,
            accounts: vec![
                // TODO: map to your program's Accounts struct field order
                AccountMeta::new(config_account, false), // config - writable
                AccountMeta::new_readonly(attacker, false), // authority - NOT a signer (the bug)
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data,
        }
    }

    #[test]
    fn attacker_mutates_config_without_signing() {
        // --- Setup ---
        let mut svm = LiteSVM::new();

        // Load the compiled program into the local validator
        // TODO: uncomment once PROGRAM_ELF path is set
        // svm.add_program(program_id(), PROGRAM_ELF);

        let legitimate_admin = Keypair::new();
        let attacker = Keypair::new();

        fund(&mut svm, &legitimate_admin.pubkey(), 10_000_000_000);
        fund(&mut svm, &attacker.pubkey(), 10_000_000_000);

        // --- Precondition: derive the config PDA or use a known address ---
        // TODO: replace with actual PDA derivation matching your program
        let (config_pda, _bump) = Pubkey::find_program_address(
            &[b"config"],
            &program_id(),
        );

        // --- Initialize the config (legitimately) ---
        // TODO: build and send the `initialize` instruction first
        // let init_ix = build_initialize_ix(program_id(), config_pda, legitimate_admin.pubkey());
        // send_tx(&mut svm, &[init_ix], &[&legitimate_admin]).expect("init should succeed");

        // --- Precondition: read the authority field before the attack ---
        // let config_before = svm.get_account(&config_pda).expect("config must exist");
        // let authority_before = parse_authority(&config_before.data); // TODO: implement parser
        // assert_eq!(authority_before, legitimate_admin.pubkey(), "setup: admin is authority");

        // --- EXPLOIT: attacker sends update_config without signing as authority ---
        let exploit_ix = build_update_config_ix(
            program_id(),
            config_pda,
            attacker.pubkey(),
        );

        let result = send_tx(&mut svm, &[exploit_ix], &[&attacker]);

        // --- ASSERTION: the tx must FAIL if the program is correct ---
        // If this assertion fails, the vulnerability is confirmed exploitable.
        assert!(
            result.is_err(),
            "FINDING CONFIRMED: unauthorized mutation succeeded - signer check is missing"
        );

        // If you expect the exploit to succeed (demonstrating the bug),
        // flip to:
        //   assert!(result.is_ok(), "exploit succeeded - signer check absent");
        //   let config_after = svm.get_account(&config_pda).expect("...");
        //   let authority_after = parse_authority(&config_after.data);
        //   assert_eq!(authority_after, attacker.pubkey(), "authority hijacked");
    }
}

// ===========================================================================
// TEMPLATE FINDING 2: Account reinitialization bypass
//
// Vulnerability class: account_reinitialization (see vuln-classes.md)
// ===========================================================================
#[cfg(test)]
mod poc_reinitialization {
    use super::*;

    #[test]
    fn double_initialize_overwrites_state() {
        let mut svm = LiteSVM::new();
        // TODO: svm.add_program(program_id(), PROGRAM_ELF);

        let user_a = Keypair::new();
        let user_b = Keypair::new();
        fund(&mut svm, &user_a.pubkey(), 10_000_000_000);
        fund(&mut svm, &user_b.pubkey(), 10_000_000_000);

        // TODO: derive the shared account PDA
        let (shared_account, _bump) =
            Pubkey::find_program_address(&[b"vault", user_a.pubkey().as_ref()], &program_id());

        // Step 1: user_a initializes the account legitimately
        // TODO: build init_ix for user_a
        // send_tx(&mut svm, &[init_ix_a], &[&user_a]).expect("first init ok");

        // Step 2: user_b calls initialize again with different params
        // TODO: build init_ix for user_b targeting same PDA
        // let result = send_tx(&mut svm, &[init_ix_b], &[&user_b]);

        // Step 3: if reinitialization is possible, user_b now owns user_a's account
        // assert!(result.is_ok(), "FINDING: reinitialization succeeded");
        // let account = svm.get_account(&shared_account).unwrap();
        // let owner = parse_owner(&account.data); // TODO
        // assert_eq!(owner, user_b.pubkey(), "account ownership hijacked");

        // Scaffold guard: this PoC is not wired to a program yet. Remove this
        // line once you have filled in the steps above; until then the test
        // is skipped, NOT silently passing as evidence.
        eprintln!("SCAFFOLD: reinitialization PoC not wired - fill in steps above");
        let _ = (user_a, user_b, shared_account);
    }
}

// ===========================================================================
// TEMPLATE: Generic invariant test (use for non-exploit correctness checks)
//
// Pattern: set up state -> perform operation -> assert invariant holds
// Useful for: token balance conservation, PDA ownership, rounding direction
// ===========================================================================
#[cfg(test)]
mod invariant_tests {
    use super::*;

    #[test]
    fn token_balance_is_conserved_across_swap() {
        let mut svm = LiteSVM::new();
        // TODO: svm.add_program(program_id(), PROGRAM_ELF);

        // TODO: set up token mints, vaults, user token accounts
        // let balance_before = get_token_balance(&svm, &vault);

        // TODO: invoke the swap instruction

        // let balance_after = get_token_balance(&svm, &vault);
        // assert_eq!(
        //     balance_before,
        //     balance_after + fee,
        //     "token balance must be conserved (output + fee == input)"
        // );

        // Scaffold guard: not wired to a program yet. Replace with the real
        // invariant assertion above before treating this test as evidence.
        eprintln!("SCAFFOLD: invariant test not wired - fill in setup + assertion above");
        let _ = &svm;
    }
}
