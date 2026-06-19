# solana-auditor

A hands-on Solana program audit workflow for Claude Code / Codex: Scope -> Static -> Manual -> Dynamic -> Report, with a litesvm proof-of-concept and a severity-graded findings report.

This skill fills the empty official seed `solanabr/solana-auditor-skill` (Superteam Brasil, "Ship useful agent skills for the Solana AI Kit").

## The problem

Auditing a Solana program is a high-skill, error-prone, multi-tool job. A reviewer has to scope the attack surface, run a handful of static tools (`cargo-audit`, `clippy`, `cargo-deny`, `cargo-geiger`), reason manually about Solana-specific bug classes that have no analogue on the EVM (missing signer/owner checks, PDA canonical-bump misuse, PDA sharing / authority overreach, account type confusion, arbitrary CPI), prove each finding with a runnable exploit, and then write it all up at the right severity. The official seed for this workflow shipped empty. There was no agent-native, opinionated lifecycle to drive it.

## What it does

Drives the full audit lifecycle as an agent workflow:

1. **Scope** - enumerate instructions, accounts, authorities, CPIs, and the trust boundary.
2. **Static** - run and triage `cargo-audit`, `clippy -D warnings`, `cargo-deny`, `cargo-geiger`.
3. **Manual** - work the Solana/Anchor vulnerability taxonomy account-by-account, instruction-by-instruction.
4. **Dynamic** - reproduce findings with a `litesvm` PoC and surface unknowns via fuzzing.
5. **Report** - write a severity-graded report (Critical/High/Medium/Low/Info) with reproduction, impact, and remediation.

Ships an actionable per-program checklist, a `litesvm` PoC harness, a fuzz-target scaffold, a CI audit workflow, and a report skeleton.

## How it complements the kit

This skill owns the auditor **workflow, tooling orchestration, PoC, and reporting**. It deliberately does not duplicate the other security skills bundled with the Solana AI Kit:

- **Formal verification (QEDGen / solana-skills)** - Kani / Lean proofs of invariants. When a finding warrants a machine-checked proof, this skill points there rather than reimplementing it.
- **Trail of Bits / ghostsecurity** - general secure-development guidance. This skill stays operational: it tells you what to run and how to prove a bug, not how to write secure code in the abstract.
- **Fixes and program changes** - delegated to the `solana-dev` skill. The auditor finds and proves; the dev skill remediates.

## How it works

A router plus on-demand leaf docs. `skill/SKILL.md` is a thin Task Routing Guide that maps each intent ("vuln classes", "static tools", "write a report") to exactly one leaf in `skill/references/`. Only the leaf you need is loaded, so context stays lean. Code lives in `skill/templates/`.

## Install

```bash
# Plugin marketplace
/plugin install solana-auditor

# Or run the installer
bash install.sh          # standard install
bash install-custom.sh   # custom path / target dir
```

The skill is discovered by its `SKILL.md` frontmatter and triggers on Solana audit / security-review intents. Fixes are delegated to the `solana-dev` skill; formal proofs to QEDGen.

## Use cases

```text
"Audit my Anchor program in programs/escrow and rank findings by severity."
"Run a static security scan (cargo-audit, clippy, cargo-deny) on this workspace and triage the output."
"I think this instruction is missing an owner check - write a litesvm PoC and a findings report entry for it."
```

## Stack (verified 2026-06)

| Tool | Version |
| --- | --- |
| cargo-audit | 0.22.2 |
| litesvm | 0.13.0 |
| anchor-lang | 1.0.2 |
| Host Rust toolchain | latest stable (~1.95, edition 2021); SBF builds use platform-tools via `cargo build-sbf` |
| cargo clippy | bundled |
| cargo-deny | 0.19.9 |
| cargo-geiger | 0.13.0 |
| trident-cli (Anchor fuzzing) | 0.12.0 (trident-fuzz 0.12.0) |
| honggfuzz-rs | 0.5.60 |

Versions last verified 2026-06; run `cargo search` / `cargo add` to re-confirm at audit time. litesvm is built on the solana 3.x crate line, so litesvm PoC dev-deps use the granular `solana-*` crates (not `solana-sdk` 4.x) - see `skill/references/sdk-versions.md`.

## License

MIT. See `LICENSE`.
