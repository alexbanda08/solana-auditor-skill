---
description: Run cargo-audit, cargo-clippy, cargo-deny, and cargo-geiger against a Solana program crate and triage the output.
---

# /static-scan

Runs the four static analysis tools against a Solana/Anchor program crate and
produces a triaged findings list. Invoke as:

```
/static-scan <PROGRAM_PATH>
```

`PROGRAM_PATH` - absolute or repo-relative path to the program crate root
(the directory containing `Cargo.toml`). All commands below assume
`cd PROGRAM_PATH` first.

For pinned tool versions and install instructions see
`skill/references/static-analysis.md` and `skill/references/sdk-versions.md`.

---

## Step 1: Dependency vulnerability scan (cargo-audit)

1. Ensure cargo-audit is installed:

   ```bash
   cargo install cargo-audit --version "^0.22" --locked
   # Verify: cargo audit --version  -> cargo-audit 0.22.x
   ```

2. Run the scan and capture output:

   ```bash
   cargo audit 2>&1 | tee audit.log
   ```

3. Triage each advisory in `audit.log`:
   - Read the RUSTSEC advisory ID (e.g. `RUSTSEC-2024-XXXX`).
   - Determine if the affected crate is a direct dependency or transitive.
   - Check if the vulnerable code path is reachable from the on-chain program
     (`[lib]` target) or only from dev/build tooling.
   - Classify: **Critical** (reachable, fund-impacting), **High** (reachable,
     DoS or auth bypass), **Medium** (reachable, limited impact), **Low**
     (unreachable or tooling-only), **Info** (unmaintained without known vuln).
   - For each finding record: RUSTSEC ID, crate name + version, classification,
     reachability verdict, and one-line rationale.

4. If `cargo audit` exits non-zero in CI, do not suppress with `--ignore`
   without a written rationale in the audit log. Use `audit.toml` ignore
   entries only for unreachable, documented false positives.

---

## Step 2: Lint scan (cargo-clippy)

5. Run clippy with all warnings promoted to errors:

   ```bash
   cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tee clippy.log
   ```

   For programs with a `no-std` or BPF target, also run:

   ```bash
   cargo clippy --target sbf-solana-solana -- -D warnings 2>&1 | tee clippy-bpf.log
   ```

6. Triage `clippy.log` by lint category:

   | Lint family | Audit relevance |
   | --- | --- |
   | `clippy::integer_arithmetic` | Overflow risk; flag every hit |
   | `clippy::cast_possible_truncation` | Lossy cast = potential overflow |
   | `clippy::unwrap_used` / `clippy::expect_used` | Panic in on-chain code = DoS |
   | `clippy::unused_must_use` | Ignored Result = silent failure |
   | `clippy::wildcard_imports` | Legibility / maintenance risk |
   | All others | Informational unless in security-critical path |

7. For each lint hit in a security-critical path (account validation,
   arithmetic, CPI, PDA derivation):
   - Record: file, line, lint ID, security relevance, preliminary severity.
   - A lone clippy lint is evidence, not a confirmed finding. Confirm with
     manual review in Phase 3 of `/audit-program`.

---

## Step 3: Dependency policy check (cargo-deny)

8. Ensure `deny.toml` exists in the crate root. If absent, create a minimal
   one:

   ```toml
   # deny.toml - minimal audit configuration
   [licenses]
   allow = ["MIT", "Apache-2.0", "Apache-2.0 WITH LLVM-exception", "ISC", "BSD-3-Clause"]

   [bans]
   multiple-versions = "warn"

   [advisories]
   db-path = "~/.cargo/advisory-db"
   db-urls = ["https://github.com/rustsec/advisory-db"]
   vulnerability = "deny"
   unmaintained = "warn"
   unsound = "deny"
   notice = "warn"

   [sources]
   unknown-registry = "deny"
   unknown-git = "warn"
   ```

9. Run:

   ```bash
   cargo deny check 2>&1 | tee deny.log
   ```

10. Triage `deny.log`:
    - `deny::advisories` hits -> merge with cargo-audit findings (same RUSTSEC IDs).
    - `deny::licenses` hits -> note any non-OSI license for the client; escalate
      if a GPL dependency could affect program binary distribution.
    - `deny::bans::multiple-versions` -> flag if a security-sensitive crate
      (e.g. `solana-sdk`, `ring`, `curve25519-dalek`) has multiple versions in
      the tree; version skew can hide patched vs unpatched code paths.
    - `deny::sources` hits -> flag any unknown registry or git source as High
      (supply-chain risk); require explicit client sign-off.

---

## Step 4: Unsafe code scan (cargo-geiger)

11. Install cargo-geiger if not present:

    ```bash
    cargo install cargo-geiger --locked
    # Note: verify current version with `cargo search cargo-geiger`
    ```

12. Run:

    ```bash
    cargo geiger 2>&1 | tee geiger.log
    ```

13. Triage `geiger.log`:
    - For each crate with `unsafe` counts > 0: note crate name, unsafe block
      count, and whether it is a direct or transitive dependency.
    - `unsafe` in the program crate itself (`src/`) is a **High** finding
      candidate; require manual review of every `unsafe` block.
    - `unsafe` in well-known cryptographic crates (e.g. `curve25519-dalek`,
      `ring`) is expected; note but do not flag as a finding.
    - `unsafe` in unknown or small transitive crates is a **Medium** candidate;
      inspect the block manually.

---

## Step 5: Consolidate and hand off

14. Produce a triage summary with one row per tool hit:

    ```
    | ID | Tool | Location | Classification | Confirmed? | Notes |
    | -- | ---- | -------- | -------------- | ---------- | ----- |
    | S-001 | cargo-audit | serde 1.0.100 RUSTSEC-... | Medium | Suspected | Transitive, check reachability |
    | S-002 | clippy | src/lib.rs:42 integer_arithmetic | High | Suspected | In fee calculation, manual review needed |
    ```

15. Save logs (`audit.log`, `clippy.log`, `deny.log`, `geiger.log`) as
    appendix material for the final report.

16. Hand off triage rows to `/audit-program` Phase 3 (manual review) or
    directly to `/write-report` if this is a static-only engagement.

---

## Automation note

The CI counterpart of this command is `templates/ci-audit.yml`, which runs
`cargo audit`, `cargo clippy -D warnings`, and `cargo deny check` on every
push. The CI gate is fail-fast; the triage depth here is deeper and
human-in-the-loop.
