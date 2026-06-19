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

- Host toolchain: latest stable Rust (~1.95, last-verified 2026-06), edition 2021.
- On-chain builds use the SBF platform-tools toolchain (its own pinned rustc,
  invoked via `cargo build-sbf`), independent of the host stable toolchain. The
  static/dynamic host tools below run on host stable; the program `.so` is built
  with platform-tools.
- Confirm with `rustc --version` and `cargo --version` (host) before any run.

---

## Static analysis (Phase 2 -> references/static-analysis.md)

| Tool         | Version | Notes |
| ------------ | ------- | ----- |
| cargo-audit  | 0.22.2  | RUSTSEC advisory scan over `Cargo.lock`. |
| cargo clippy | bundled | Ships with the host Rust toolchain; no pin. |
| cargo-deny   | 0.19.9  | licenses / bans / advisories / sources gate (does not publish 1.x). |
| cargo-geiger | 0.13.0  | `unsafe` surface; release cadence lags - may not build on the newest host stable. |

Install:

```bash
cargo install cargo-audit  --version 0.22.2 --locked
rustup component add clippy
cargo install cargo-deny   --version 0.19.9 --locked
cargo install cargo-geiger --version 0.13.0 --locked   # may lag the toolchain
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
| litesvm       | 0.13.0  | crate dep, caret "0.13"; in-process SVM for PoC/invariant tests. Built on the solana 3.x crate line (see PoC dev-deps below). |
| anchor-lang   | 1.0.2   | dep "1.0"; for Anchor program targets and trident. (anchor-lang 1.0.2 also tracks the solana 3.x crates.) |
| trident-cli   | 0.12.0  | Anchor program fuzzer; stateful/guided, native TridentSVM engine (no honggfuzz/AFL since 0.11), cross-platform. The trident crates version in lockstep, so 0.12.0 pairs with trident-fuzz 0.12.0 (it depends on trident-client ^0.12.0). 0.13.0-rc.* prereleases exist - do not pin a prerelease. Confirm anchor-lang compat. |
| honggfuzz-rs  | 0.5.60  | coverage-guided fuzzing of pure (non-Anchor) Rust functions. |

PoC/fuzz dev-dependencies. litesvm's public API is built on the solana 3.x crate
line, so a litesvm test crate must use the granular `solana-*` crates below, NOT
`solana-sdk` 4.x. Pulling solana-sdk 4.x into a litesvm test crate yields a v4
`Transaction`/`Message`/`Instruction` that will not unify with the v3 types
litesvm's `send_transaction` accepts (cargo cannot link two majors of
`solana-transaction`/`solana-message`) -> E0308. (`solana-sdk` 4.x is correct for
PROGRAM code; it is wrong for a litesvm PoC dev-dep block.)

```toml
[dev-dependencies]
litesvm                 = "0.13"
solana-keypair          = "3.1"
solana-signer           = "3"
solana-instruction      = "=3.2"   # litesvm 0.13 pins =3.2.0; match exactly
solana-transaction      = { version = "3.1", features = ["bincode"] }
solana-pubkey           = { version = "4", features = ["curve25519"] }
solana-system-interface = "3"
anchor-lang             = "1.0"    # only if the target is an Anchor program
```

Notes on the pins above (the feature flags matter for a HOST test build, not BPF):
- `solana-instruction = "=3.2"` mirrors litesvm 0.13's exact `=3.2.0` requirement;
  using the same crate version guarantees the `Instruction` type unifies. Its
  `Instruction` struct (constructed as a literal in the harness) is behind the
  default `std` feature, which `=3.2` brings.
- `solana-transaction`'s `bincode` feature is required for
  `Transaction::new_signed_with_payer` (used by the harnesses).
- `solana-pubkey`'s `curve25519` feature is required for
  `Pubkey::find_program_address` off-chain (it is always available under
  `target_os = "solana"`, so program code does not need the flag).
- `solana-pubkey` 4.x re-exports `solana_address::Address as Pubkey`; litesvm's
  `airdrop`/`get_account`/`add_program` take `Address`, so a `Pubkey` value passes
  directly. Do not pull `solana-sdk` into this crate.

Install (fuzzers are separate binaries):

```bash
cargo install trident-cli                 # 0.12.0 stable; confirm anchor compat
cargo install honggfuzz --version 0.5.60 --locked
```

Note: LiteSVM/trident/honggfuzz harnesses require `cargo` and a compiled program
`.so`; they do not build or run in a network-free environment without the toolchain.

---

## Maturity flags (honesty)

- trident and honggfuzz-rs move independently of the core SDK; a version that lags
  anchor-lang 1.0.2 may not build against it - verify before depending on it, and
  fall back to a LiteSVM property test if the fuzzer does not resolve.
- cargo-geiger historically trails new Rust releases; if it fails to build on the
  current host stable, substitute a grep sweep for `unsafe`
  (references/static-analysis.md).
- cargo-audit only knows what the RUSTSEC database knows at run time - a clean run
  means "no KNOWN advisory," not "no vulnerable dependency." Update the advisory db
  (cargo audit fetches it) immediately before the run.

---

## Related pins

For program/Anchor/transaction crates used in fixes and PoCs (solana-sdk 4.0.1,
solana-client 4.0.0, anchor-lang 1.0.2, etc.) see the kit-wide version list and the
solana-dev skill via references/delegation.md - this skill does not re-pin the full
build stack, only the audit tooling above.
