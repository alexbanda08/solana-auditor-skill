---
description: Run the full 5-phase Solana/Anchor program audit (scope, static, manual, dynamic, report) on a target program path.
---

# /audit-program

Runs the complete audit lifecycle on a Solana/Anchor program. Invoke as:

```
/audit-program <PROGRAM_PATH> [--dry-run]
```

`PROGRAM_PATH` - absolute or repo-relative path to the program crate root
(the directory containing `Cargo.toml`). `--dry-run` prints commands without
executing them.

---

## Phase 1: Scope

1. Read `skill/references/methodology.md` to orient on the engagement model,
   time-box, and deliverable expectations.

2. Enumerate the program surface. From `PROGRAM_PATH` run:

   ```bash
   # List all instruction entry points
   grep -rn "pub fn " src/ --include="*.rs" | grep -v "#\[cfg(test)\]"

   # List all account structs
   grep -rn "#\[derive(Accounts)\]" src/ --include="*.rs" -A 1

   # List all CPI calls
   grep -rn "invoke\|invoke_signed\|CpiContext" src/ --include="*.rs"

   # List all PDAs
   grep -rn "find_program_address\|create_program_address\|seeds\s*=" src/ --include="*.rs"

   # Count lines of Rust source
   find src/ -name "*.rs" | xargs wc -l | tail -1
   ```

3. Record in a `SCOPE.md` scratch file (never committed as deliverable):
   - Program name and version/commit hash
   - Instructions enumerated (name, mutability, signers)
   - External programs called via CPI
   - Upgrade authority status (immutable vs mutable)
   - Audit start date and planned depth (full / targeted)

4. Open `templates/audit-checklist.md` and copy it to `audit-checklist-work.md`
   in the working directory. This is your living checklist for phases 2-4.

---

## Phase 2: Static Analysis

5. Run the full static scan by invoking `/static-scan` (see
   `commands/static-scan.md` for full detail):

   ```bash
   cd PROGRAM_PATH
   cargo audit
   cargo clippy -- -D warnings 2>&1 | tee clippy.log
   cargo deny check 2>&1 | tee deny.log
   # cargo geiger if available
   cargo geiger 2>&1 | tee geiger.log || echo "cargo-geiger not installed, skip"
   ```

6. Triage each tool's output. For every advisory or warning:
   - Classify: confirmed vulnerability / suspected / false positive / informational
   - Record location (crate name, version, advisory ID or lint name)
   - Map to a vuln class from `references/vuln-classes.md` if applicable
   - Mark the corresponding row in `audit-checklist-work.md`

---

## Phase 3: Manual Review

7. Load `skill/references/vuln-classes.md`. Work through each vuln class
   against the program source. For each instruction and account struct:

   a. **Signer checks** - every privileged instruction has a `Signer` constraint
      or explicit `is_signer` check; no unsigned authority bypass possible.

   b. **Owner checks** - every account has `owner = <expected_program_id>` or
      Anchor `#[account(owner = ...)]`; no arbitrary account substitution.

   c. **PDA canonical bump** - `find_program_address` result is stored and
      reused; `create_program_address` is not called with user-supplied bumps.

   d. **Arbitrary CPI** - `program_id` in every CPI is verified against a
      known constant; `remaining_accounts` are not passed unchecked to CPI.

   e. **Account reinitialization** - accounts with `init` or manual
      `assign`/`allocate` check the discriminator; no re-init on existing data.

   f. **Integer overflow** - all arithmetic uses `checked_*`, `saturating_*`,
      or is in a context where overflow is provably impossible.

   g. **Close-account revival** - closed accounts have lamports zeroed and data
      wiped in the same transaction; no revival window between instructions.

   h. **Duplicate mutable accounts** - no two `AccountInfo` args alias the
      same on-chain address in a mutable position.

   i. **Unchecked `remaining_accounts`** - any use of `ctx.remaining_accounts`
      validates owner, signer, and expected type before use.

   j. Load `skill/references/anchor-checks.md` for the full Anchor constraint
      matrix if the program uses Anchor macros.

8. For each issue found: record file, line range, vuln class, severity
   estimate (use `references/report-template.md` scale), and a one-line
   reproduction sketch. Mark the checklist row.

---

## Phase 4: Dynamic Analysis (PoC and Fuzzing)

9. Load `skill/references/dynamic-analysis.md`.

10. For each Confirmed or Suspected finding from Phase 3, build a litesvm
    PoC using `templates/litesvm-harness.rs` as scaffold:

    ```bash
    # Create a dedicated test crate (or add to existing tests/)
    mkdir -p poc/src
    cp skill/templates/litesvm-harness.rs poc/src/lib.rs
    # Edit poc/Cargo.toml - see templates/litesvm-harness.rs header comment
    # Run (requires cargo + network for first fetch):
    cargo test --manifest-path poc/Cargo.toml -- --nocapture
    ```

    The PoC must:
    - Deploy the program under test via `LiteSvm::load_program`
    - Set up the exploit preconditions (attacker keypair, malformed accounts)
    - Submit the transaction
    - Assert the invariant violation (wrong balance, unauthorized write, etc.)
    - Panic with a clear message if the exploit succeeds

11. For logic-heavy instructions or complex state machines, optionally run
    the fuzz harness scaffold from `templates/fuzz-target.rs`. See
    `references/dynamic-analysis.md` for trident/honggfuzz setup.

12. Upgrade Confirmed/Suspected labels based on PoC results:
    - PoC panics on invariant violation -> **Confirmed**
    - PoC does not trigger -> downgrade to **Suspected**, note what would confirm

---

## Phase 5: Report

13. Collect all findings from the checklist and PoC runs. Pass them to
    `agents/audit-report-writer.md` or run `/write-report` directly:

    ```
    /write-report
    ```

14. The report agent loads `references/report-template.md` and
    `templates/report.md` to produce the final severity-graded deliverable.

15. Attach `clippy.log`, `deny.log`, and any PoC source as appendices.

16. State explicitly in the Executive Summary:
    - What was NOT reviewed (out-of-scope instructions, off-chain components)
    - Tool versions used (see `references/sdk-versions.md`)
    - Commit hash or binary hash of the program artifact reviewed

---

## Delegation

- **Fixes / patches** -> do not implement here. Reference `references/delegation.md`
  and hand off to the `solana-dev` skill.
- **Formal invariant proofs** -> reference `references/delegation.md` and point
  to QEDGen / solana-skills.
- **General secure-dev guidance** -> Trail of Bits / ghostsecurity skill.
