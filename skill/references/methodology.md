# Audit Methodology: 5-Phase Lifecycle

The audit workflow is `Scope -> Static -> Manual -> Dynamic -> Report`. Each phase
has explicit entry and exit criteria. Do not advance until exit criteria are met.
This skill owns the workflow; for formal proofs see references/delegation.md
(QEDGen), for code fixes see references/delegation.md (../solana-dev/).

Golden rules (from rules/audit-rigor.md): never assert a program is "safe" or
"secure"; every finding needs evidence + reproduction + a severity; distinguish
CONFIRMED (you reproduced it) from SUSPECTED (you reasoned it). Time-box; record
what you did NOT review.

---

## Phase 1: Scope

Goal: bound the engagement and freeze the target. Avoid auditing a moving repo.

Entry criteria:
- Source access (git URL + exact commit hash, NOT "main").
- Stated trust model: who are the actors, what is privileged, what is the
  upgrade authority, what funds/accounts are at risk.
- A build that compiles: `anchor build` or `cargo build-sbf` succeeds.

Activities:
- Pin the commit. Record `solana --version`, `anchor --version`, Rust toolchain.
- Inventory: programs, instructions, account types, PDAs, CPIs, external
  programs called, oracles/price feeds, admin/governance paths.
- Map the attack surface: every instruction handler and who can call it.
- Identify the IDL/interface and any deployed program id on mainnet/devnet.
- Define out-of-scope items explicitly (e.g. frontend, off-chain keeper).

Exit criteria:
- Frozen commit hash recorded in the report header.
- Instruction + account inventory complete.
- Trust model and out-of-scope list written down.

---

## Phase 2: Static

Goal: cheap, automated breadth. Catch known-bad patterns before manual review.

Entry criteria: Scope frozen; toolchain installed (see references/sdk-versions.md).

Activities (see references/static-analysis.md for exact commands):
- `cargo audit` -> known-vuln dependencies (RUSTSEC advisories).
- `cargo clippy --all-targets -- -D warnings` -> lints, footguns.
- `cargo deny check` -> licenses, bans, advisories, duplicate deps.
- `cargo geiger` -> `unsafe` usage surface (verify availability).
- Grep sweep for high-signal patterns: `unwrap`, `unchecked`, `remaining_accounts`,
  `invoke(` / `invoke_signed(`, `Pubkey::find_program_address`, `close`,
  manual `try_borrow`, `AccountInfo` raw access.

Exit criteria:
- Tool outputs captured (raw logs attached or referenced).
- Each finding triaged: false positive, informational, or candidate for Manual.
- Clean clippy (or every remaining warning justified in writing).

---

## Phase 3: Manual

Goal: the core of the audit. Reason about logic the tools cannot.

Entry criteria: Static complete; candidate list from Phase 2 in hand.

Activities:
- Walk every instruction against the taxonomy in references/vuln-classes.md.
- Check every account against references/anchor-checks.md (signer, owner, type,
  has_one, seeds/bump, init/init_if_needed, close).
- Trace each CPI: is the target program id constrained? Are signer seeds correct?
- Trace arithmetic: overflow, rounding direction, decimals, fee math.
- Trace state machine: can an instruction be replayed, reordered, or reentered?
- Check authority/upgrade paths and any `remaining_accounts` consumption.
- For each suspected issue, write a hypothesis: precondition -> action -> impact.

Exit criteria:
- Every instruction and account reviewed (checklist in templates/audit-checklist.md
  fully ticked or explicitly N/A).
- Each finding labelled CONFIRMED or SUSPECTED with a written rationale.
- A prioritized list of items to reproduce in Phase 4.

---

## Phase 4: Dynamic

Goal: turn SUSPECTED into CONFIRMED (or refute) with executable proof.

Entry criteria: Manual complete; reproduction targets identified.

Activities (see references/dynamic-analysis.md):
- Write a LiteSVM PoC per finding (templates/litesvm-harness.rs): construct the
  malicious tx, assert the exploit succeeds or the invariant breaks.
- Fuzz high-risk handlers and math (templates/fuzz-target.rs; trident/honggfuzz).
- Run any existing program tests; add invariant/property tests for gaps.
- Capture exact reproduction: inputs, accounts, expected vs actual, logs.

Exit criteria:
- Every CONFIRMED finding has a runnable PoC and observed output.
- SUSPECTED findings that could not be reproduced are downgraded and noted as such.
- Reproduction steps are deterministic and recorded.

---

## Phase 5: Report

Goal: communicate findings so they can be fixed and verified.

Entry criteria: Dynamic complete; PoCs and evidence collected.

Activities (use templates/report.md):
- One entry per finding: title, severity, location (file:line + commit),
  description, impact, reproduction (PoC link), remediation pointer, status.
- Apply the fixed severity scale (Critical/High/Medium/Low/Info) consistently;
  rationalize each rating (impact x likelihood).
- Summarize scope, methodology, and what was NOT reviewed.
- Point remediation at ../solana-dev/ (see references/delegation.md); never write
  the fix inside the audit deliverable yourself.

Exit criteria:
- Report compiles the full finding set with severities and reproductions.
- Coverage statement (reviewed vs out-of-scope) present.
- No claim that the program is "safe"; only findings + residual-risk notes.

---

## When to escalate to formal verification (QEDGen)

Manual + Dynamic give confidence by example; they cannot prove absence of a bug.
Escalate to formal methods (Kani bounded model checking, Lean proofs) via the
QEDGen skill (see references/delegation.md) when:
- A safety property must hold for ALL inputs, not just tested ones (e.g. "total
  supply is conserved across every instruction", "no path mints without burn").
- Arithmetic invariants are subtle and adversarial (AMM curve, interest accrual,
  liquidation math) where a single rounding path can drain funds.
- The protocol holds high TVL and a SUSPECTED-but-unreproduced overflow remains.
- A property is naturally expressible as an assertion over a bounded state space
  that Kani can exhaust.

Do NOT escalate routine checks (signer/owner/has_one) - those are decided in
Manual. Formal verification complements this workflow; it does not replace the
PoC-driven loop above.
