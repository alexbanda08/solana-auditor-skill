# Solana Program Audit Checklist

Per-program checklist. Mark each item: Y (confirmed safe), N (finding), NA (not applicable).
One row per concern; link to vuln-classes.md for taxonomy detail.
Run through phases in order: Scope -> Static -> Manual -> Dynamic.

---

## Phase 1: Scope and Reconnaissance

| # | Check | Y/N/NA | Where to Look |
|---|-------|--------|---------------|
| 1.1 | Upgrade authority documented; multisig or governance if mainnet | | `declare_id!`, deploy scripts, README |
| 1.2 | All PDAs listed with their seeds and bump source of truth | | Account structs, `seeds =`, `bump =` constraints |
| 1.3 | External programs called via CPI are identified and versioned | | `CpiContext`, `invoke`, `invoke_signed` call sites |
| 1.4 | Token mints and vaults identified; ownership confirmed | | `token::authority`, `mint::authority` constraints |
| 1.5 | Admin/privileged roles enumerated; no unnamed god-key | | Config/State account fields, signer checks |
| 1.6 | All remaining_accounts usage catalogued | | `ctx.remaining_accounts` references |

---

## Phase 2: Static Analysis (run tools first, then triage)

| # | Check | Y/N/NA | Where to Look |
|---|-------|--------|---------------|
| 2.1 | `cargo audit` passes: no known CVEs in dependency tree | | `Cargo.lock`, CI output |
| 2.2 | `cargo clippy -D warnings` passes with no suppressed warnings | | `#[allow(...)]` annotations; remove and re-run |
| 2.3 | `cargo deny check` passes: license + banned crates policy | | `deny.toml` |
| 2.4 | `cargo geiger` shows no unsafe outside intentional sites | | geiger report; each unsafe block documented |
| 2.5 | No `unwrap()` / `expect()` on `Option`/`Result` in hot paths | | grep `\.unwrap()`, `\.expect(` in src/ |
| 2.6 | No floating-point arithmetic in financial calculations | | grep `f32`, `f64` in src/ |
| 2.7 | Dependencies pinned or narrowly ranged in Cargo.toml | | Cargo.toml `[dependencies]` |

---

## Phase 3: Account Validation (manual, instruction-by-instruction)

### 3a. Signer Checks

| # | Check | Y/N/NA | Where to Look |
|---|-------|--------|---------------|
| 3.1 | Every privileged instruction has at least one `Signer` account | | Accounts struct; `#[account(signer)]` or `Signer<'info>` |
| 3.2 | Authority fields compared to the actual signing pubkey | | `constraint = authority.key() == config.authority` or equivalent |
| 3.3 | No instruction allows an arbitrary pubkey to act as authority | | Any `ctx.accounts.authority` used without constraint |

### 3b. Owner and Program ID Checks

| # | Check | Y/N/NA | Where to Look |
|---|-------|--------|---------------|
| 3.4 | All program-owned accounts use `Account<'info, T>` (auto-checks owner) | | Accounts struct; raw `AccountInfo` replaced or validated |
| 3.5 | CPI target program IDs validated; not accepted from user input | | `CpiContext::new`; `program.key() == expected_program_id` |
| 3.6 | Token program passed by user is checked against `spl_token::id()` | | `token_program` account; Anchor `Program<'info, Token>` |

### 3c. Account Data Matching and Type Confusion

| # | Check | Y/N/NA | Where to Look |
|---|-------|--------|---------------|
| 3.7 | Discriminator (Anchor) or custom tag verified on deserialization | | `Account<'info, T>` handles this; raw `try_from_slice` does not |
| 3.8 | Stored relationship fields (authority/owner/mint) are bound to the passed account | | `has_one = authority`; explicit `stored == passed.key()` checks (vuln class 3b) |
| 3.9 | No two instructions accept the same account in conflicting roles | | Cross-instruction account reuse; mutable aliasing |
| 3.10 | Remaining_accounts elements are validated (owner, writable, signer) before use | | Every loop over `ctx.remaining_accounts` |

### 3d. PDA Checks

| # | Check | Y/N/NA | Where to Look |
|---|-------|--------|---------------|
| 3.11 | `bump` stored in account state comes from `find_program_address`; not accepted from user | | `seeds::program`, `bump` constraint vs. stored bump field |
| 3.12 | Canonical bump enforced (`seeds = [...], bump = account.bump`) | | `bump =` vs. `bump` (unconstrained) in Anchor attribute |
| 3.13 | PDA seeds cannot be manipulated to collide with another account | | Seed composition; user-controlled seed segments |
| 3.14 | PDA used as a signer is bound to its domain (seeds include the user/account key); no shared-authority overreach | | `invoke_signed` seeds; global `[b"authority"]` PDAs (vuln class 16) |

### 3e. Reinitialization

| # | Check | Y/N/NA | Where to Look |
|---|-------|--------|---------------|
| 3.15 | Initialized accounts use `init` or explicit `is_initialized` guard | | `init` constraint; manual check in handler body |
| 3.16 | No instruction can overwrite a live account's discriminator or state | | `init_if_needed` usage flagged and reviewed carefully |

### 3f. Close-Account and Revival

| # | Check | Y/N/NA | Where to Look |
|---|-------|--------|---------------|
| 3.17 | Closed accounts use `close = destination` (Anchor) wiping discriminator | | `#[account(close = ...)]` constraint |
| 3.18 | Closed account pubkeys cannot be reused in same tx to bypass checks | | Same-tx ordering; revival attacks documented |

### 3g. Duplicate Mutable Accounts

| # | Check | Y/N/NA | Where to Look |
|---|-------|--------|---------------|
| 3.19 | No two writable accounts in one instruction can resolve to same pubkey | | Anchor `constraint = a.key() != b.key()` guards |
| 3.20 | Token source != token destination enforced on transfer instructions | | `constraint = src.key() != dst.key()` |

---

## Phase 4: Arithmetic and Economics

| # | Check | Y/N/NA | Where to Look |
|---|-------|--------|---------------|
| 4.1 | All additions use `checked_add` or `saturating_add`; no silent overflow | | grep `+ `, look for unchecked integer ops |
| 4.2 | All multiplications use `checked_mul`; division checked for zero denominator | | grep `* `, `/ ` in src/ |
| 4.3 | Rounding direction favors protocol, not user (especially in fee/collateral math) | | `/ denom` vs. `checked_div`; ceil vs. floor intent |
| 4.4 | Price/oracle inputs validated: stale, confidence interval, zero-price guard | | Pyth `PriceFeed`, `publish_time`, `conf`; Switchboard checks |
| 4.5 | No single-block TWAP or spot price used for collateral valuation | | Oracle usage sites; TWAP window length |

---

## Phase 5: CPI Safety

| # | Check | Y/N/NA | Where to Look |
|---|-------|--------|---------------|
| 5.1 | Arbitrary CPI disallowed: program id hardcoded or validated | | `invoke` / `invoke_signed` / `CpiContext` program argument |
| 5.2 | CPI return values / success codes checked | | `invoke(...)` result not discarded; `?` propagation |
| 5.3 | Reentrancy: program does not call back into itself via CPI | | CPI chain; recursive invocation path |
| 5.4 | Sysvar accounts use `Sysvar::get()` (not raw account data injection) | | `Clock::get()`, `Rent::get()` preferred over `clock: AccountInfo` |

---

## Phase 6: Token and Lamport Flows

| # | Check | Y/N/NA | Where to Look |
|---|-------|--------|---------------|
| 6.1 | Token vault authority is a PDA owned by program, not an EOA | | `token_vault.owner == program_id` via Anchor `authority` |
| 6.2 | Lamport transfers via `system_program::transfer` only; no raw lamport manipulation without rent check | | `system_instruction::transfer`; direct lamport writes |
| 6.3 | Rent-exempt minimum enforced on all created accounts | | `Rent::get()?.minimum_balance(space)`; `rent_exempt` constraint |
| 6.4 | No funds can be drained via close + reinitialize in same block | | Close flows; time-lock or epoch guard present |

---

## Phase 7: Upgrade and Admin Controls

| # | Check | Y/N/NA | Where to Look |
|---|-------|--------|---------------|
| 7.1 | Upgrade authority set to a multisig or locked post-audit if mainnet | | `solana program show <id>` output in scope doc |
| 7.2 | Admin migration paths (owner swap, config update) require multisig quorum | | Admin instructions; DAO/timelock integration |
| 7.3 | Emergency pause / kill-switch present and tested | | Pause flag in config; instruction gating on `!config.paused` |

---

## Phase 8: Dynamic / PoC Verification

| # | Check | Y/N/NA | Where to Look |
|---|-------|--------|---------------|
| 8.1 | litesvm harness exercised for each Critical/High finding (see litesvm-harness.rs) | | PoC test output; exploit tx confirmed or refuted |
| 8.2 | Fuzz target run for at least N iterations on instruction data (see fuzz-target.rs) | | Fuzzer corpus; crashes triaged |
| 8.3 | CI pipeline (ci-audit.yml) integrated and green on audit branch | | GitHub Actions; all jobs passing |
| 8.4 | All findings confirmed as exploitable OR downgraded to "suspected" with reasoning | | report.md findings table |

---

## Checklist Summary

| Phase | Total | Y | N (Findings) | NA |
|-------|-------|---|---------------|----|
| 1. Scope | 6 | | | |
| 2. Static | 7 | | | |
| 3. Account Validation | 20 | | | |
| 4. Arithmetic | 5 | | | |
| 5. CPI Safety | 4 | | | |
| 6. Token/Lamport Flows | 4 | | | |
| 7. Admin Controls | 3 | | | |
| 8. Dynamic | 4 | | | |
| **TOTAL** | **53** | | | |

**Auditor:** ___________________  
**Date:** ___________________  
**Program ID:** ___________________  
**Commit hash:** ___________________
