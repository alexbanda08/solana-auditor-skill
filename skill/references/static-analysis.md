# Static Analysis: Tooling and Triage

Static analysis is Phase 2 (see references/methodology.md). It is cheap, automated
breadth: catch known-bad dependencies and footgun patterns before spending manual
time. Static tools do NOT understand program logic - they cannot find a missing
signer check or a CPI to an unconstrained program. Treat every hit as a CANDIDATE
to confirm in Manual/Dynamic, never as a finished finding.

Install commands and pinned versions: references/sdk-versions.md.

---

## cargo audit (0.22.2) - vulnerable dependencies

What it catches: dependencies with a published RUSTSEC advisory (known CVEs, yanked
crates, unmaintained crates). It reads `Cargo.lock`, so the lockfile must exist and
reflect the audited commit.

```bash
cargo audit                       # advisories against Cargo.lock
cargo audit --deny warnings       # fail CI on unmaintained/yanked warnings too
cargo audit --json > audit.json   # machine-readable for the report appendix
```

Triage:
- A hit names the crate, the advisory id (RUSTSEC-YYYY-NNNN), and patched versions.
- Confirm the vulnerable code path is actually reachable from the program. A
  transitive dev-dependency advisory is usually Info; a runtime dependency with a
  reachable bug can be High.
- `unmaintained`/`unsound` warnings are signals, not proof - record as Info unless
  you can tie them to an exploit path.
- Fix is a version bump or `[patch]`; that work goes to ../solana-dev/ via
  references/delegation.md, not into the audit deliverable.

---

## cargo clippy -D warnings - lints and footguns

What it catches: Rust footguns that often indicate Solana bugs - `unwrap`/`expect`
on attacker-influenced input (panic = DoS), lossy casts (`as u64` truncation),
needless clones, and arithmetic patterns. `-D warnings` makes every lint a hard
error so nothing is silently ignored.

```bash
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy -p <program-crate> -- -D warnings -W clippy::arithmetic_side_effects
```

Triage:
- `clippy::arithmetic_side_effects` flags raw `+ - * /` on integers. In on-chain
  code these are overflow candidates; cross-check against the integer-overflow
  class in references/vuln-classes.md (release builds wrap by default).
- `unwrap_used` / `expect_used` (pedantic) on instruction input -> reachable panic
  -> DoS candidate. Confirm the input is attacker-controlled before rating.
- `cast_possible_truncation` / `cast_sign_loss` -> silent value corruption in fee
  or amount math.
- A genuinely safe lint may be suppressed with a narrow `#[allow(...)]` plus a
  written justification. Record any suppression in the report's coverage notes.

---

## cargo deny check - licenses, bans, advisories

What it catches: a configurable supply-chain policy gate covering four areas -
`advisories` (RUSTSEC, overlaps cargo audit), `bans` (forbidden crates, duplicate
versions of the same crate), `licenses` (disallowed/unknown SPDX licenses), and
`sources` (crates from unapproved registries/git).

```bash
cargo deny check                  # all checks
cargo deny check advisories
cargo deny check bans licenses
```

Triage:
- Requires a `deny.toml`. If absent, generate one (`cargo deny init`) and record
  that the policy is auditor-supplied, not the project's.
- Duplicate-version bans matter on-chain: two versions of `solana-program` or
  `anchor-lang` in one tree is a real correctness/ABI risk - escalate to Manual.
- License hits are usually compliance Info, not security. State them as such.
- `sources` hits (a crate pulled from an unexpected git URL) can be supply-chain
  High - verify the source before dismissing.

---

## cargo geiger - unsafe surface (verify availability)

What it catches: counts `unsafe` blocks/functions across the dependency tree and
the program crate. It quantifies the surface that the borrow checker does NOT
protect; it does not judge whether any given `unsafe` is wrong. Note: cargo-geiger
release cadence lags the toolchain - pin 0.13.0 (last-verified 2026-06) and confirm
it builds against your host stable before relying on it (see
references/sdk-versions.md); if it does not, fall back to a grep sweep for `unsafe`.

```bash
cargo geiger --all-features                 # unsafe usage table
cargo geiger -p <program-crate> --output-format Ascii
```

Triage:
- Focus on `unsafe` in the program crate and any in-house helper crates, not the
  whole ecosystem. Standard `unsafe` deep in well-known crates is expected.
- For each in-scope `unsafe`: what invariant must hold, who can violate it, and is
  that reachable from an instruction? Raw pointer / transmute over account data is
  a type-confusion candidate (references/vuln-classes.md).
- Zero unsafe in the program crate is good hygiene but NOT a safety guarantee -
  most Solana bugs are safe-Rust logic errors. Do not let a clean geiger run imply
  the program is secure (see rules/audit-rigor.md).

---

## Grep sweep - high-signal patterns

Tools miss patterns that are not lints. Run a targeted grep to build the Manual
candidate list (ripgrep shown):

```bash
rg -n "unwrap\(|expect\(|panic!|\bas (u|i)(8|16|32|64|128)\b" programs/
rg -n "remaining_accounts|invoke\(|invoke_signed\(|AccountInfo" programs/
rg -n "find_program_address|create_program_address|\.bump|seeds" programs/
rg -n "close|try_borrow|init_if_needed|UncheckedAccount|/// CHECK" programs/
```

Each hit maps to a vuln class in references/vuln-classes.md. The grep produces
hypotheses, not findings - confirm in Manual and, where exploitable, prove with a
LiteSVM PoC in Dynamic (references/dynamic-analysis.md).

---

## Output handling

- Capture raw logs (`audit.json`, clippy output, `cargo deny` output) and attach
  or reference them in the report appendix - reproducibility is mandatory.
- Re-run static tools on the exact frozen commit from Phase 1; stale runs against
  `main` are not evidence.
- Static analysis closing rule: a clean static run lowers some risk and produces a
  candidate list. It is never sufficient to call a program safe.
