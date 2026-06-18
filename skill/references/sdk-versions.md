# Tool Versions (last-verified 2026-06)

Pinned versions for the audit toolchain. ALWAYS re-confirm before an engagement -
advisory databases and tool releases move fast. Verify with:

```bash
cargo search <crate>            # latest published version
cargo add <crate> --dry-run     # resolved version against your toolchain
cargo <tool> --version          # installed binary version
```

last-verified 2026-06. Run the commands above to confirm the latest at audit time.

---

## Toolchain

- Rust toolchain: 1.96, edition 2021.
- Confirm with `rustc --version` and `cargo --version` before any run; static and
  dynamic tools below are validated against this toolchain.

---

## Static analysis (Phase 2 -> references/static-analysis.md)

| Tool         | Version | Notes |
| ------------ | ------- | ----- |
| cargo-audit  | 0.22.2  | RUSTSEC advisory scan over `Cargo.lock`. |
| cargo clippy | bundled | Ships with the Rust 1.96 toolchain; no pin. |
| cargo-deny   | verify  | licenses / bans / advisories / sources gate; confirm latest. |
| cargo-geiger | verify  | `unsafe` surface; release cadence lags - confirm it builds on Rust 1.96. |

Install:

```bash
cargo install cargo-audit --version 0.22.2 --locked
rustup component add clippy
cargo install cargo-deny --locked        # verify resolved version
cargo install cargo-geiger --locked      # verify; may lag the toolchain
```

Run (full commands and triage in references/static-analysis.md):

```bash
cargo audit
cargo clippy --all-targets --all-features -- -D warnings
cargo deny check
cargo geiger --all-features
```

---

## Dynamic analysis (Phase 4 -> references/dynamic-analysis.md)

| Tool          | Version | Notes |
| ------------- | ------- | ----- |
| litesvm       | 0.12.0  | crate dep, caret "0.12"; in-process SVM for PoC/invariant tests. |
| anchor-lang   | 1.0.2   | dep "1.0"; for Anchor program targets and trident. |
| trident       | verify  | Anchor program fuzzer; younger than core SDK - confirm anchor-lang compat. |
| honggfuzz-rs  | verify  | coverage-guided fuzzing of pure Rust functions; confirm latest. |

Cargo.toml (dev-dependencies for the PoC/fuzz harnesses):

```toml
[dev-dependencies]
litesvm = "0.12"
solana-sdk = "4.0"      # see kit-wide pins; for building txs in PoCs
anchor-lang = "1.0"     # only if the target is an Anchor program
```

Install (fuzzers are separate binaries):

```bash
cargo install trident-cli --locked       # verify resolved version + anchor compat
cargo install honggfuzz --locked         # verify resolved version
```

Note: LiteSVM/trident/honggfuzz harnesses require `cargo` and a compiled program
`.so`; they do not build or run in a network-free environment without the toolchain.

---

## Maturity flags (honesty)

- trident and honggfuzz-rs move independently of the core SDK; a version that lags
  anchor-lang 1.0.2 may not build against it - verify before depending on it, and
  fall back to a LiteSVM property test if the fuzzer does not resolve.
- cargo-geiger historically trails new Rust releases; if it fails to build on Rust
  1.96, substitute a grep sweep for `unsafe` (references/static-analysis.md).
- cargo-audit only knows what the RUSTSEC database knows at run time - a clean run
  means "no KNOWN advisory," not "no vulnerable dependency." Update the advisory db
  (cargo audit fetches it) immediately before the run.

---

## Related pins

For program/Anchor/transaction crates used in fixes and PoCs (solana-sdk 4.0.1,
solana-client 4.0.0, anchor-lang 1.0.2, etc.) see the kit-wide version list and the
solana-dev skill via references/delegation.md - this skill does not re-pin the full
build stack, only the audit tooling above.
