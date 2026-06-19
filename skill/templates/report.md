# Solana Program Security Audit Report

---

## Engagement Summary

| Field | Value |
|---|---|
| **Program Name** | `<Program Name>` |
| **Program ID** | `<base58 pubkey>` |
| **Audit Commit** | `<git sha>` |
| **Audit Date Range** | YYYY-MM-DD to YYYY-MM-DD |
| **Lead Auditor** | `<name / handle>` |
| **Report Version** | 1.0 |
| **Status** | Draft / Final |

---

## Scope

- Repository: `<org/repo>` branch `<branch>`
- Crates in scope: `<list>`
- Crates out of scope: `<list>`
- Excluded: tests, migrations, off-chain tooling (unless explicitly included)

---

## Severity Scale

| Severity | Criteria |
|---|---|
| **Critical** | Direct loss of user funds, unauthorized privilege escalation, or systemic protocol compromise; exploitable with no preconditions. |
| **High** | Significant loss of funds or privilege under realistic conditions; exploitable with minor preconditions or partial trust. |
| **Medium** | Limited fund loss, degraded protocol behavior, or findings that require specific conditions to exploit. |
| **Low** | Best-practice deviations, hardening gaps, or findings with no direct economic impact. |
| **Info** | Code quality, documentation, or non-security observations. |

Status per finding: **Confirmed** (PoC demonstrates exploit), **Suspected** (evidence but no PoC), **Acknowledged**, **Fixed**, **Won't Fix**.

---

## Findings Summary

| ID | Title | Severity | Status |
|---|---|---|---|
| SOL-001 | Missing signer check on `update_config` instruction | Critical | Confirmed |
| SOL-002 | PDA bump accepted from user input in `create_vault` | High | Suspected |
| SOL-003 | Integer overflow in fee calculation on large deposits | Medium | Confirmed |
| SOL-004 | Token source equals destination not enforced on swap | Low | Confirmed |
| SOL-005 | Upgrade authority is a hot wallet on mainnet | Info | Acknowledged |

---

## Detailed Findings

---

### SOL-001: Missing Signer Check on `update_config` Instruction

**Severity:** Critical
**Status:** Confirmed

#### Description

The `update_config` instruction does not verify that the `authority` account is a transaction signer. The account is declared as `AccountInfo` rather than `Signer<'info>`, allowing any caller to pass an arbitrary pubkey as `authority` without co-signing the transaction.

#### Vulnerable Code

File: `programs/my_program/src/instructions/update_config.rs`, line 47.

```rust
// VULNERABLE: authority is AccountInfo, not Signer<'info>
#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(mut, has_one = authority)]
    pub config: Account<'info, Config>,
    /// CHECK: no signer constraint applied -- MISSING
    pub authority: AccountInfo<'info>,
}
```

#### Impact

An attacker calls `update_config` with their own pubkey as `authority`. The `has_one` constraint passes because the attacker controls which value they supply. They can overwrite all config fields: fee recipients, max caps, and the authority field itself. Complete protocol takeover with no preconditions.

#### Reproduction

PoC: see `templates/litesvm-harness.rs`, module `poc_missing_signer_check`.

Steps:
1. Deploy the program at commit `<sha>` to a local validator.
2. Initialize a config account with `legitimate_admin` as authority.
3. Submit `update_config` signed only by `attacker`, passing `attacker.pubkey()` as `authority`.
4. Observe: transaction succeeds; `config.authority` is now `attacker.pubkey()`.

#### Remediation

Change `authority` account type to `Signer<'info>`:

```rust
#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(mut, has_one = authority)]
    pub config: Account<'info, Config>,
    pub authority: Signer<'info>,   // enforce signing
}
```

For fix implementation, delegate to `../solana-dev/` (see `references/delegation.md`).

---

### SOL-002: PDA Bump Accepted from User Input in `create_vault`

**Severity:** High
**Status:** Suspected

#### Description

The `create_vault` instruction accepts a `bump` argument from the caller and stores it directly without verifying it matches the canonical bump from `Pubkey::find_program_address`. A non-canonical bump may cause PDA collisions or break downstream `invoke_signed` calls.

#### Vulnerable Code

File: `programs/my_program/src/instructions/create_vault.rs`, line 82.

```rust
// SUSPECTED: bump comes from ix args, not from the constraint
pub fn create_vault(ctx: Context<CreateVault>, bump: u8) -> Result<()> {
    ctx.accounts.vault.bump = bump;   // user-controlled
    Ok(())
}
```

#### Impact

Non-canonical bump stored in vault state causes `invoke_signed` to fail to reproduce the correct PDA, potentially locking funds. In edge cases, a crafted bump may collide with another program-derived address, enabling account confusion.

#### Evidence

Static: no `bump = vault.bump` constraint in the account macro; bump flows from instruction data to stored state unchecked.

#### Reproduction

PoC not yet completed. Steps:
1. Call `create_vault` with seeds `[b"vault", user.pubkey()]` and bump `255` (non-canonical).
2. Inspect stored `vault.bump`.
3. Attempt a downstream withdrawal; verify if `invoke_signed` fails or succeeds unexpectedly.

#### Remediation

Use Anchor canonical bump constraint:

```rust
#[account(
    init,
    payer = payer,
    seeds = [b"vault", payer.key().as_ref()],
    bump,
    space = 8 + Vault::INIT_SPACE,
)]
pub vault: Account<'info, Vault>,

pub fn create_vault(ctx: Context<CreateVault>) -> Result<()> {
    ctx.accounts.vault.bump = ctx.bumps.vault;  // from constraint, not args
    Ok(())
}
```

---

### SOL-003: Integer Overflow in Fee Calculation on Large Deposits

**Severity:** Medium
**Status:** Confirmed

#### Description

Fee calculation in `deposit.rs` multiplies `amount * fee_bps` as plain `u64` without `checked_mul`. For amounts above `u64::MAX / fee_bps`, the multiplication wraps silently, producing a near-zero fee.

#### Vulnerable Code

File: `programs/my_program/src/instructions/deposit.rs`, line 31.

```rust
// VULNERABLE: unchecked multiplication
let fee = amount * fee_bps / 10_000;
```

#### Impact

At current SOL prices the overflow threshold (~1.8 billion SOL) is economically unrealistic. If the program is extended to handle 6-decimal tokens such as USDC, the threshold drops by 10^6 and becomes reachable. Severity escalates to High in that scenario.

#### Reproduction

Standalone Rust (no external crates needed):

```rust
fn main() {
    let amount: u64 = u64::MAX / 100;
    let fee_bps: u64 = 300;
    let fee_bad = amount.wrapping_mul(fee_bps) / 10_000;
    let fee_safe = amount.checked_mul(fee_bps)
        .and_then(|x| x.checked_div(10_000))
        .unwrap_or(u64::MAX);
    println!("overflow fee: {}  safe fee: {}", fee_bad, fee_safe);
    assert_ne!(fee_bad, fee_safe);
}
```

#### Remediation

```rust
let fee = amount
    .checked_mul(fee_bps)
    .ok_or(ErrorCode::ArithmeticOverflow)?
    .checked_div(10_000)
    .ok_or(ErrorCode::ArithmeticOverflow)?;
```

---

### SOL-004: Token Source Equals Destination Not Enforced on Swap

**Severity:** Low
**Status:** Confirmed

#### Description

The `swap` instruction does not assert that source and destination token accounts are different. Passing the same account for both produces a no-op SPL transfer while the program may still record the swap event, potentially inflating volume metrics or triggering spurious rewards.

#### Vulnerable Code

File: `programs/my_program/src/instructions/swap.rs` - Accounts struct has no key-inequality constraint.

#### Remediation

```rust
#[account(
    mut,
    constraint = src_token.key() != dst_token.key() @ ErrorCode::SameSourceDestination
)]
pub src_token: Account<'info, TokenAccount>,
```

---

### SOL-005: Upgrade Authority Is a Hot Wallet on Mainnet

**Severity:** Info
**Status:** Acknowledged

#### Description

`solana program show <program_id>` shows upgrade authority as a single-signature hot wallet. A compromised key allows an attacker to deploy a malicious upgrade with no additional quorum.

#### Recommendation

Transfer upgrade authority to a multisig (e.g., Squads v4) or burn it if the program is considered final. Use a hardware-wallet-backed keypair at minimum. See `references/delegation.md` for admin control flows.

---

## Methodology

The audit followed the workflow in `references/methodology.md`:
1. Scope and reconnaissance
2. Static analysis (`cargo audit`, `cargo clippy`, `cargo deny`, `cargo geiger`)
3. Manual review per `templates/audit-checklist.md`
4. Dynamic PoC via litesvm (`templates/litesvm-harness.rs`)
5. Report with severity grading

Formal verification (Kani / Lean proofs): delegated to QEDGen/solana-skills.
Remediation implementation: see `references/delegation.md` -> `../solana-dev/`.

---

## Out of Scope

- Off-chain indexer and UI components
- Economic game-theory analysis beyond direct arithmetic
- Formal proofs (delegated to QEDGen/solana-skills)

---

## Appendix A: Tool Versions Used

| Tool | Version |
|---|---|
| Host Rust toolchain | latest stable (~1.95); SBF builds via platform-tools (`cargo build-sbf`) |
| anchor-lang | 1.0.2 |
| cargo-audit | 0.22.2 |
| litesvm | 0.13.0 |
| cargo clippy | (rustup component) |
| cargo-deny | 0.19.9 |
| cargo-geiger | 0.13.0 |
| trident-cli | 0.12.0 (trident-fuzz 0.12.0) |
| honggfuzz-rs | 0.5.60 |

See `references/sdk-versions.md` for canonical version list. last-verified 2026-06;
re-confirm at audit time.

---

## Appendix B: Disclaimer

This report reflects the state of the codebase at the audited commit. It is not a guarantee that the program is free of vulnerabilities. New code, dependency updates, or configuration changes may introduce new findings. "Confirmed" means a PoC was demonstrated; "suspected" means static evidence was found but PoC was not completed. Never assert a program is "safe" or "secure" based on this report alone.
