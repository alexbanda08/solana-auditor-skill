---
name: solana-auditor
description: Solana/Anchor program security audit workflow - scope the attack surface, run static analysis (cargo-audit/clippy/deny/geiger), manual vuln review (signer/owner/PDA-bump/CPI/overflow/type-confusion/close-revival), dynamic analysis (litesvm PoC, fuzzing), and severity-graded reporting; use when asked to audit, security-review, find vulnerabilities in, or write a findings report for a Solana program.
user-invocable: true
---

# solana-auditor

A hands-on Solana program audit workflow: **Scope -> Static -> Manual -> Dynamic -> Report**, plus a runnable PoC and a severity-graded findings report.

## Scope and complements

This skill owns the auditor workflow, tooling orchestration, PoC, and reporting. It complements, and does not duplicate, the other security skills in the Solana AI Kit:

- **Formal verification** (QEDGen / solana-skills: Kani/Lean proofs) -> when a finding warrants a machine-checked invariant proof, point to QEDGen; do not reimplement proofs here.
- **Secure-dev guidance** (Trail of Bits / ghostsecurity) -> general guidance; this skill stays operational (what to run, how to prove a bug).
- **Fixes / program changes** -> delegate to the `solana-dev` skill via `references/delegation.md`.

## Source precedence (read before acting)

1. **Evidence per finding.** Every reported issue needs a concrete location, a reproduction (PoC or exact trace), an impact statement, and a severity. Distinguish **confirmed** from **suspected**.
2. **Never claim a program is "safe" or "secure."** An audit reduces risk over a defined scope and time box; it does not certify safety. State scope and limitations.
3. **Use the fixed severity scale** (Critical / High / Medium / Low / Info) from `references/report-template.md`. Do not invent ad-hoc severities.
4. **Delegate.** Fixes and program changes -> `solana-dev`. Formal proofs of invariants -> QEDGen. This skill finds, proves, and reports; it does not patch.
5. **One leaf per task.** Load exactly the leaf the routing table points to; do not bulk-load references.

## Task Routing Guide

| User asks about X | Load this leaf |
| --- | --- |
| audit process / lifecycle / where to start | `references/methodology.md` |
| vuln classes / taxonomy / what to look for | `references/vuln-classes.md` |
| static tools (cargo-audit / clippy / deny / geiger) | `references/static-analysis.md` |
| fuzzing / litesvm / dynamic / PoC | `references/dynamic-analysis.md` |
| Anchor constraints / account checks | `references/anchor-checks.md` |
| severity / findings / report | `references/report-template.md` |
| tool versions | `references/sdk-versions.md` |
| fixes / program changes / formal proofs | `references/delegation.md` |

## Progressive disclosure

- `references/methodology.md` - the 5-phase lifecycle, scoping, time-boxing, deliverables.
- `references/vuln-classes.md` - Solana/Anchor vuln taxonomy: what / why / how-to-detect / fix-pointer.
- `references/static-analysis.md` - running and triaging cargo-audit, clippy, cargo-deny, cargo-geiger.
- `references/dynamic-analysis.md` - litesvm PoC harness, fuzzing (trident / honggfuzz), invariant tests.
- `references/anchor-checks.md` - Anchor account constraints, `#[account(...)]`, `has_one`, `seeds`/`bump`.
- `references/report-template.md` - severity scale + findings report structure.
- `references/sdk-versions.md` - pinned tool versions (last-verified 2026-06).
- `references/delegation.md` - hand-off to `solana-dev` (fixes) and QEDGen (proofs).

Templates in `templates/`: `audit-checklist.md`, `litesvm-harness.rs`, `fuzz-target.rs`, `ci-audit.yml`, `report.md`.

## Agents

| Agent | Use when |
| --- | --- |
| `agents/solana-auditor.md` | Driving the hands-on audit lifecycle: scope, static, manual, dynamic, PoCs, raw findings (opus). |
| `agents/audit-report-writer.md` | Turning raw findings into the severity-graded deliverable report (sonnet). |

## Commands

| Command | Does |
| --- | --- |
| `commands/audit-program.md` | Run the full 5-phase audit (scope, static, manual, dynamic, report) on a program path. |
| `commands/static-scan.md` | Run cargo-audit + clippy + cargo-deny + cargo-geiger and triage. |
| `commands/write-report.md` | Assemble findings into the severity-graded report. |
