# Dynamic Analysis: PoC, Fuzzing, Replay

Dynamic analysis is Phase 4 (see references/methodology.md). Its job is to turn a
SUSPECTED finding into a CONFIRMED one with executable proof - or to refute it.
Every Critical/High finding in the report should carry a runnable PoC; a finding
you cannot reproduce is downgraded and labelled SUSPECTED (rules/audit-rigor.md).

The two workhorses are LiteSVM (fast, deterministic exploit PoCs) and a fuzzer
(trident for Anchor, or honggfuzz-rs for raw harnesses). Versions and install:
references/sdk-versions.md. These tests require `cargo` to build and a compiled
program `.so`; they do not run offline without the toolchain.

---

## LiteSVM (0.12) - fast exploit/invariant PoC

LiteSVM is an in-process SVM: no validator, no RPC, no ledger. It loads your
program `.so` and lets you build, sign, and send transactions in microseconds,
inspect accounts directly, and assert on results. This is the default tool for a
per-finding PoC. Scaffold: templates/litesvm-harness.rs.

Minimal exploit PoC shape:

```rust
use litesvm::LiteSVM;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

let mut svm = LiteSVM::new();
svm.add_program_from_file(program_id, "target/deploy/program.so").unwrap();

let attacker = Keypair::new();
svm.airdrop(&attacker.pubkey(), 10_000_000_000).unwrap();

// 1. Set up victim state (init accounts the legit way).
// 2. Build the MALICIOUS instruction (e.g. omit the expected signer,
//    swap in an account owned by the wrong program, pass a fake PDA bump).
let tx = Transaction::new_signed_with_payer(
    &[malicious_ix],
    Some(&attacker.pubkey()),
    &[&attacker],
    svm.latest_blockhash(),
);

// 3. Assert the EXPLOIT outcome, not just "it errored".
let res = svm.send_transaction(tx);
assert!(res.is_ok(), "expected program to (wrongly) accept the tx");
// 4. Read state and prove the invariant broke (funds moved, authority changed).
let bal = svm.get_account(&victim_vault).unwrap().lamports;
assert_eq!(bal, 0, "vault drained -> finding CONFIRMED");
```

PoC discipline:
- Prove the SECURITY outcome (funds moved, authority changed, supply inflated), not
  merely that a call returned `Ok`. A passing tx that does nothing is not a finding.
- Pair each exploit PoC with a NEGATIVE control: the same flow done legitimately
  should be rejected, showing the gap is the missing check, not your setup.
- Make it deterministic: fixed keypairs/seeds where reproduction matters, no wall
  clock, no network. Record exact inputs and observed vs expected output.
- One PoC per finding; name the test after the finding id so the report links cleanly.

Use LiteSVM (not a local validator) for the audit loop: it is fast enough to keep
one test per finding. Use `solana-test-validator` only when you must exercise real
sysvars, slots, or CPIs into deployed mainnet programs the harness cannot stub.

---

## Fuzzing - trident (Anchor) / honggfuzz-rs

Fuzzing explores inputs you would not write by hand. Use it for arithmetic-heavy
and stateful handlers where a single edge case (overflow, rounding, ordering) drains
funds. Scaffold and tool choice rationale: templates/fuzz-target.rs.

Tool choice:
- trident (Anchor program fuzzer) - generates instruction sequences against an
  Anchor program and checks invariants between steps. Preferred when the target is
  Anchor and you want sequence-level coverage. Verify the current version supports
  your anchor-lang (see references/sdk-versions.md); it is younger than the core SDK.
- honggfuzz-rs - coverage-guided fuzzing of a pure Rust function. Preferred for an
  isolated math/parsing routine (fee curve, tick math, deserializer) you can lift
  out of the program and feed raw bytes/values.

What to assert (invariants, not just "no panic"):
- Conservation: total in == total out across an instruction (no mint without burn).
- Monotonicity / bounds: a balance or index never goes negative or past a cap.
- Round-trip: `deserialize(serialize(x)) == x` for account/state codecs.
- No panic on adversarial input (a reachable panic is a DoS finding by itself).

A fuzzer finds a counterexample; you then minimize it and replay it as a LiteSVM
PoC so the report has a deterministic reproduction. If a class of bug must hold for
ALL inputs (not just those fuzzed), escalate to formal verification via QEDGen
(references/delegation.md) - fuzzing gives confidence by example, not a proof.

---

## Transaction replay

When a finding concerns an already-deployed program or a real incident, replay the
concrete transaction(s) instead of synthesizing inputs:

- Fetch the failing/suspicious tx and its account states at that slot from RPC
  (`getTransaction`, `getAccountInfo` with the slot).
- Reconstruct the accounts in LiteSVM (set their data/owner/lamports directly via
  `set_account`) and resend the instruction to observe behavior under a controlled
  SVM where you can mutate one variable at a time.
- Use this to bisect WHICH account/field triggers the bug and to validate that a
  proposed remediation (handed to ../solana-dev/) actually closes it.

---

## Exit criteria (Phase 4)

- Every CONFIRMED finding: a runnable PoC + observed output recorded.
- Every SUSPECTED finding that did not reproduce: downgraded and noted as such.
- Reproduction steps deterministic; raw output captured for the report appendix.
- PoCs and fuzz targets map to templates/litesvm-harness.rs and
  templates/fuzz-target.rs and are referenced by finding id in the report
  (references/report-template.md).
