# Solana / Anchor Vulnerability Taxonomy

Scannable reference for Phase 3 (Manual). Each class: WHAT it is, WHY it bites,
DETECT (how to spot it), FIX (pointer - actual fixes go to ../solana-dev/, see
references/delegation.md). Anchor mechanics live in references/anchor-checks.md.

Convention below: "native" = raw solana-program account handling; "Anchor" =
anchor-lang 1.0 constraints.

---

## 1. Missing signer check
- WHAT: a privileged action runs without verifying the authority signed the tx.
- WHY: anyone can pass the authority's pubkey as a read-only account and act as them.
- DETECT: native handlers using `AccountInfo` for an authority without checking
  `is_signer`; Anchor authorities typed as `AccountInfo`/`UncheckedAccount`
  instead of `Signer`; admin paths gated only on a stored pubkey equality.
- FIX: require `Signer<'info>` (Anchor) or assert `account.is_signer` (native).

## 2. Missing owner check
- WHAT: an account is read/trusted without confirming the owning program.
- WHY: an attacker crafts a look-alike account owned by a program they control
  with attacker-chosen data.
- DETECT: native code deserializing `AccountInfo.data` without checking
  `account.owner == expected_program_id`; Anchor `UncheckedAccount` used for
  program-owned state instead of `Account<'info, T>`.
- FIX: use `Account<'info, T>` (Anchor checks owner automatically) or assert owner
  in native code before deserializing.

## 3. Account data matching / type confusion
- WHAT: two account types share a layout; one is passed where the other is expected.
- WHY: without a discriminator check, struct A's bytes are interpreted as struct B,
  letting an attacker substitute state.
- DETECT: native `try_from_slice` with no type tag; manual deserialization of
  `AccountInfo`; missing or hand-rolled discriminator; reused PDAs across types.
- FIX: Anchor's 8-byte discriminator (auto via `#[account]`) or an explicit type
  tag checked on every load.

## 4. Arbitrary CPI / unchecked program id
- WHAT: a cross-program invocation targets a program id taken from input.
- WHY: attacker points the CPI at a malicious program that mimics the expected
  interface and steals authority or funds.
- DETECT: `invoke`/`invoke_signed` where the target program account is not
  constrained; missing `address = <expected_id>` or `Program<'info, X>` typing;
  token CPIs not pinned to the SPL Token program id.
- FIX: constrain the program account (`Program<'info, Token>`, `address = ...`);
  validate the program id before the CPI.

## 5. PDA canonical bump misuse
- WHAT: a PDA is derived/verified with an attacker-supplied or non-canonical bump.
- WHY: multiple valid bumps exist; using a non-canonical one (or trusting a passed
  bump) lets an attacker forge a "valid" PDA at a different address.
- DETECT: `create_program_address` with a user-passed bump instead of
  `find_program_address`; Anchor `seeds` without `bump` (or `bump = user_input`);
  bumps stored without being re-validated as canonical.
- FIX: derive with `find_program_address`, store the canonical bump, and pin it via
  `seeds = [...], bump = stored_bump` (see references/anchor-checks.md).

## 6. Account reinitialization
- WHAT: an already-initialized account is initialized again.
- WHY: reinit resets state (balances, authorities, flags), enabling theft or
  privilege reset.
- DETECT: `init_if_needed` without guarding existing state; native init that does
  not check an `is_initialized` flag; setters that overwrite without checking.
- FIX: use `init` (fails if exists) where possible; if `init_if_needed` is required,
  guard every field; maintain and check an initialized flag.

## 7. Integer overflow / underflow
- WHAT: arithmetic wraps or panics on out-of-range values.
- WHY: release builds historically wrapped silently; a wrap on balances/supply is
  catastrophic. A panic is a DoS.
- DETECT: raw `+ - *` on `u64`/`u128` balances; missing `checked_*`/`saturating_*`;
  `overflow-checks` not set in the program `Cargo.toml` profile.
- FIX: use `checked_add`/`checked_mul` (handle `None`) or `saturating_*` where
  appropriate; set `overflow-checks = true` for the release profile.

## 8. Missing rent / exemption
- WHAT: an account is created or left below the rent-exempt minimum.
- WHY: non-exempt accounts can be reaped, deleting state, or cause failures.
- DETECT: manual account creation without rent-exempt lamport calc; resizing data
  without topping up lamports; trusting client-provided lamport amounts.
- FIX: size lamports via `Rent::minimum_balance(len)`; Anchor `init` handles this
  for created accounts - verify it for manual/realloc paths.

## 9. Sysvar spoofing
- WHAT: a sysvar (Clock, Rent, Instructions, etc.) is read from an unverified account.
- WHY: attacker passes a fake account in the sysvar slot, lying about time or the
  instruction context.
- DETECT: sysvars read via `AccountInfo` without an id check; instruction
  introspection (`sysvar::instructions`) not validated; reliance on `Clock` for
  security decisions.
- FIX: use Anchor `Sysvar<'info, Clock>` / typed sysvar accounts, or assert the
  account id equals the canonical sysvar id; prefer `Clock::get()`.

## 10. Close-account / revival
- WHAT: an account is "closed" but can be reused (revived) within the same or a
  later transaction.
- WHY: draining lamports alone does not clear data or prevent reuse; a revived
  account carries stale state and can be re-funded back to rent exemption.
- DETECT: native close that only transfers lamports without zeroing data and
  writing the closed discriminator; Anchor close paths bypassed; CPIs that re-fund
  a just-closed account; reuse of closed PDAs.
- FIX: use Anchor's `close = recipient` (zeroes data, sets CLOSED discriminator);
  in native code zero the data and assign to the system program.

## 11. Duplicate mutable accounts
- WHAT: the same account is passed in two slots that are assumed distinct.
- WHY: a handler that debits "from" and credits "to" can be fed from == to,
  double-counting or bypassing checks (classic balance inflation).
- DETECT: two mutable accounts of the same type with no `key != key` check; Anchor
  without a `constraint = a.key() != b.key()`; transfer/swap handlers.
- FIX: assert distinct keys (`constraint = from.key() != to.key()`); validate
  relationships before mutation.

## 12. Arithmetic rounding
- WHAT: division/rounding direction favors the user instead of the protocol.
- WHY: consistent round-in-user-favor lets repeated tiny ops extract value
  (dust attacks, share-price manipulation, first-depositor inflation).
- DETECT: integer division where rounding direction is unstated; share/asset
  conversions; fee calc that truncates against the protocol; mixed-decimal math.
- FIX: round in the protocol's favor (floor on payout, ceil on charge); use u128
  intermediates; seed/guard pools against first-depositor share inflation.

## 13. Oracle / price manipulation
- WHAT: a price feed is trusted without staleness/confidence/source validation, or
  a spot price (AMM reserves) is used as an oracle.
- WHY: stale or manipulable prices enable underpriced borrows, bad liquidations,
  and drains.
- DETECT: Pyth/Switchboard reads without staleness (publish-time) and confidence
  checks; using a single DEX pool reserve ratio as price; no sanity bounds.
- FIX: validate publish time vs `Clock`, enforce max confidence interval, bound
  deviation; prefer TWAP / multi-source over spot reserves.

## 14. Unchecked remaining_accounts
- WHAT: `ctx.remaining_accounts` are consumed without validating owner/type/order.
- WHY: Anchor does NOT apply constraints to remaining_accounts; an attacker
  controls them fully.
- DETECT: iteration over `remaining_accounts` with deserialize-and-trust; index
  assumptions about ordering; no owner/key/signer checks inside the loop.
- FIX: manually validate each: owner, discriminator/type, expected key/PDA, signer
  status, and length/order; document the expected layout.

## 15. Upgrade-authority risk
- WHAT: the program upgrade authority (or a powerful admin) can change code/state.
- WHY: a single key (or unrenounced authority) is a centralization and rug vector;
  a compromised admin can drain.
- DETECT: program deployed with a live single-key upgrade authority; admin
  instructions without timelock/multisig; no event/logging on authority changes;
  authority stored but never checked on sensitive ops.
- FIX: multisig/governance or renounced upgrade authority; timelock sensitive
  admin actions; document the trust assumption explicitly in the report (this is
  often an Info/Low disclosure rather than a code bug, but always disclose it).

---

For each class you flag: record file:line + commit, label CONFIRMED vs SUSPECTED,
attach a PoC where reproduced (references/dynamic-analysis.md), and route the fix
to ../solana-dev/ (references/delegation.md). Never mark a program "safe" because
a class is absent - mark only that you reviewed for it.
