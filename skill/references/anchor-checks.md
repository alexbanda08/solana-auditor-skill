# Anchor Account Checks (anchor-lang 1.0.2)

What Anchor enforces automatically vs what you must still verify by hand. Use this
in Phase 3 (Manual) alongside templates/audit-checklist.md. Maps to the taxonomy in
references/vuln-classes.md.

Mental model: Anchor's `#[derive(Accounts)]` constraints are checks that run at
deserialization, in declaration order, BEFORE your handler body. Anything not
expressed as a constraint (or not covered by a typed wrapper) is NOT checked.

---

## Account wrapper types: what each guarantees

- `Signer<'info>`: account signed the tx (`is_signer == true`). Does NOT check
  owner or any data. Use for authorities/payers. (Taxonomy 1.)
- `Account<'info, T>`: owner == declaring program AND 8-byte discriminator matches
  `T` AND data deserializes into `T`. The workhorse for program-owned state.
  (Taxonomy 2, 3.)
- `Program<'info, T>`: account is executable AND its key == `T::id()`. Use this for
  CPI targets so the program id is pinned. (Taxonomy 4.)
- `AccountInfo<'info>` / `UncheckedAccount<'info>`: NO checks at all. Anchor 1.0
  requires `/// CHECK:` docs on these. Every one is an audit hotspot - the
  developer is asserting they validated it manually; verify that claim.
- `SystemAccount<'info>`: owner == System Program.
- `Sysvar<'info, Clock>` (etc.): key == the canonical sysvar id. Prevents sysvar
  spoofing (Taxonomy 9). Prefer `Clock::get()` where possible.
- `InterfaceAccount<'info, T>` / `Interface<'info, T>`: like Account/Program but
  accepts a set of allowed program ids (e.g. Token + Token-2022). Confirm the
  allowed set is what the protocol intends.

What `Account<T>` does NOT do: it does not check the account is the RIGHT instance
(e.g. that a `Vault` belongs to this `User`). That is what `has_one`/`seeds` are for.

---

## Constraints and what they verify

- `mut`: marks writable. Absence of `mut` on a mutated account is a logic bug
  (Anchor will reject the write), but presence is also a flag: a needlessly-mutable
  account widens surface.
- `signer` (attribute form): same as `Signer` typing; assert is_signer.
- `has_one = authority`: checks `self.<field>.authority == authority.key()`. Verify
  the named target account is itself a checked `Signer`/`Account`, else the link is
  to an unverified key. (Taxonomy 1, relationship binding.)
- `seeds = [...]` + `bump`: re-derives the PDA and checks it equals the passed
  account key. With bare `bump`, Anchor uses the CANONICAL bump (calls
  find_program_address). (Taxonomy 5.)
- `seeds = [...]` + `bump = stored_bump`: checks against a SPECIFIC bump you supply
  - only safe if `stored_bump` is the canonical bump that was persisted at init.
  Never pass a user-controlled bump here. (Taxonomy 5.)
- `seeds::program = other_program`: PDA derived under a different program id; verify
  that program id is itself pinned/trusted.
- `constraint = <expr>`: arbitrary boolean. Use for cross-account invariants, e.g.
  `constraint = from.key() != to.key()` (Taxonomy 11), or
  `constraint = pool.mint == user_ata.mint`.
- `address = EXPECTED`: pins the account key to a constant (e.g. an admin pubkey or
  program id). (Taxonomy 4, 15.)
- `owner = some_program`: checks the account owner (beyond the default for
  `Account<T>`); used with `AccountInfo`/`UncheckedAccount`.
- `realloc = N, realloc::payer = p, realloc::zero = bool`: resizing. Audit:
  `realloc::zero` must be `true` when GROWING into reused memory to avoid stale
  data leakage; ensure rent top-up (Taxonomy 8).

Order matters: constraints evaluate top-to-bottom; a later check cannot save you
from a side effect of an earlier `init`. Read the struct as a sequence.

---

## init vs init_if_needed (Taxonomy 6)

- `init`: creates the account, funds rent exemption (`payer`, `space`), assigns
  owner, writes the discriminator. FAILS if the account already exists. This is the
  safe default.
- `space = N`: you must size it correctly. Under-sizing -> runtime failure;
  over-trusting a client value -> grief. For `Account<T>`, include the 8-byte
  discriminator. Verify the arithmetic.
- `init_if_needed`: creates only if missing; otherwise loads the existing account
  and RUNS YOUR HANDLER AGAINST IT. This is the reinit footgun: a second call hits
  the "already exists" branch with attacker-chosen timing. Requires the
  `init-if-needed` Cargo feature.
  - AUDIT: every field the handler writes must be safe to write on an
    already-initialized account, or the handler must branch on initialized state.
    Treat any `init_if_needed` as SUSPECTED-reinit until proven safe; build a
    LiteSVM PoC that calls the instruction twice (templates/litesvm-harness.rs).

---

## close (Taxonomy 10)

- `close = recipient`: Anchor zeroes the account data, writes the CLOSED sentinel
  discriminator, and sends lamports to `recipient`. This is the correct close.
- AUDIT: confirm close is actually `close = ...` and not a hand-rolled lamport
  transfer (which leaves data intact and allows revival). Check that nothing in the
  same tx re-funds the account back to rent exemption. Check the recipient is
  constrained (not attacker-chosen if that matters).

---

## remaining_accounts (Taxonomy 14)

Anchor applies ZERO constraints to `ctx.remaining_accounts`. They are raw
`AccountInfo`. For each one your handler consumes you must manually check: owner,
discriminator/type (re-deserialize via `Account::try_from`), expected key or PDA
derivation, signer status if needed, and index/length assumptions. Audit the loop
as if it were native code.

---

## Quick auditor pass (per Accounts struct)

1. Every authority/payer is `Signer` (or `signer`).            (Taxonomy 1)
2. Every program-owned state is `Account<T>`, not Unchecked.   (Taxonomy 2, 3)
3. Every CPI target is `Program<T>` / `address`-pinned.        (Taxonomy 4)
4. Every PDA has `seeds` + canonical `bump` (or stored bump).  (Taxonomy 5)
5. Each `/// CHECK:` Unchecked account has a justified manual check - find it.
6. `init_if_needed` and hand-rolled close are SUSPECTED until PoC'd. (6, 10)
7. Cross-account links use `has_one`/`constraint`; distinct-key checks present. (11)
8. `remaining_accounts` fully validated in-handler.            (Taxonomy 14)

Fixes -> ../solana-dev/ (references/delegation.md). Never conclude "Anchor handles
it" without naming the exact constraint/type that does.
