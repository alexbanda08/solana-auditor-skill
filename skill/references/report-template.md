# Reporting: Severity Scale and Finding Format

Reporting is Phase 5 (see references/methodology.md). The report is the deliverable:
it must let a reader reproduce each issue, understand its impact, and route the fix.
The fillable skeleton is templates/report.md - this leaf defines the scale and the
required structure. Governing rules: rules/audit-rigor.md (evidence per finding,
confirmed vs suspected, never assert "safe").

---

## Severity scale (fixed - do not invent new levels)

Severity = Impact x Likelihood, then placed on this five-level scale. Always state
both halves so the rating is auditable.

- **Critical** - Direct, attacker-triggerable loss or seizure of funds/authority,
  or a protocol-wide break, with low precondition. Examples: unauthenticated drain
  of a vault, mint of unlimited tokens, takeover of the upgrade/admin authority.
  Expectation: fix before any deployment.

- **High** - Loss of funds or critical state corruption, but gated by a non-trivial
  precondition (specific state, privileged-but-not-admin role, race window). Or a
  Critical-class bug whose exploit path is partially mitigated.

- **Medium** - Limited or conditional loss, denial of service against a single
  user/instruction, or a correctness bug that degrades but does not break the
  protocol. Often requires unusual conditions or attacker cost near the gain.

- **Low** - Minor, hard-to-exploit, or low-impact issues: small rounding leakage,
  recoverable DoS, missing defense-in-depth that no current path reaches.

- **Info** - No direct security impact: code quality, deviation from best practice,
  unused checks, documentation/spec mismatches, gas/CU inefficiency.

Rating discipline:
- Rate the realistic on-chain attacker, not a theoretical one with admin keys.
- If impact is high but you could not reproduce it, keep the finding SUSPECTED and
  say so in Status; do not inflate severity to compensate for missing proof.
- A panic reachable from instruction input is at least Medium (DoS), higher if it
  bricks shared state.

---

## Finding format (one block per finding)

Every finding MUST contain these fields, in this order:

```
### [SEV-NNN] <Short imperative title>

- Severity: <Critical | High | Medium | Low | Info>
- Status: <Confirmed | Suspected>  (Confirmed requires a runnable PoC)
- Location: <path/file.rs:Lstart-Lend> @ <commit hash>
- Class: <vuln class from references/vuln-classes.md, e.g. "missing signer check">

Description:
  What the code does and why it is wrong. Reference the exact constraint or check
  that is missing or incorrect.

Impact:
  Concrete consequence in protocol terms (funds drained, authority seized, user
  DoS). State the attacker and the preconditions.

Reproduction:
  Deterministic steps. Link the PoC (templates/litesvm-harness.rs derived) or the
  fuzz counterexample / replayed tx. Include observed vs expected output.

Remediation:
  WHAT must change (the missing check / corrected math), as a pointer - not a
  patch. Hand implementation to ../solana-dev/ via references/delegation.md. If the
  invariant warrants a machine-checked proof, note QEDGen.
```

Use a stable id scheme (e.g. `SEV-001`, `SEV-002`) so PoCs, the checklist, and the
fix tickets all reference the same finding.

---

## Executive summary (top of report)

A non-specialist (project lead, sponsor) should grasp risk from this section alone:

- Target: program name, repo, frozen commit hash, deployed program id(s).
- Scope: what was reviewed and - explicitly - what was NOT (rules/audit-rigor.md
  requires the coverage statement). Time box.
- Methodology: the 5-phase lifecycle, tools used (versions from
  references/sdk-versions.md).
- Findings table: count by severity + a one-line title each.

Example findings table:

```
| ID      | Severity | Status    | Title                                  |
| ------- | -------- | --------- | -------------------------------------- |
| SEV-001 | Critical | Confirmed | Vault withdraw missing signer check    |
| SEV-002 | High     | Confirmed | Fee math truncates via u64 cast        |
| SEV-003 | Medium   | Suspected | Close handler may allow account revival|
```

---

## Report hygiene

- Order findings by severity (Critical first), stable id within a level.
- Do NOT write "the program is safe/secure." State: findings, residual risk, and
  the coverage boundary (rules/audit-rigor.md).
- Attach raw evidence (tool logs, PoC source, observed output) as an appendix so
  the report is independently reproducible.
- Keep a Status field current as fixes land: a remediated finding moves to
  "Fixed (re-tested @ <commit>)" only after you re-run its PoC against the patch.
- Sections and severity buckets must match templates/report.md exactly.
