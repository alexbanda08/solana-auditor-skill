# Delegation Boundaries

This skill (solana-auditor) owns ONE thing: the audit workflow - Scope -> Static ->
Manual -> Dynamic -> Report, plus PoC construction and the severity-graded report.
It deliberately does NOT own code fixes or formal proofs. Routing those out keeps
this skill focused and avoids duplicating the kit's existing skills.

---

## What this skill OWNS

- The 5-phase methodology (references/methodology.md) and its entry/exit gates.
- The vulnerability taxonomy and detection guidance (references/vuln-classes.md).
- Anchor constraint/account analysis (references/anchor-checks.md).
- Tool orchestration: static (references/static-analysis.md) and dynamic /
  PoC (references/dynamic-analysis.md), plus templates/ (checklist, litesvm-harness,
  fuzz-target, ci-audit, report).
- Severity grading and the findings report (templates/report.md), under the
  evidence rules in rules/audit-rigor.md.

The deliverable of this skill is a REPORT with reproductions, not a patched program
and not a proof artifact.

---

## What goes to ../solana-dev/ (CODE FIXES + program changes)

Once a finding is CONFIRMED and reported, the remediation is a development task,
not an audit task. Hand off to the ../solana-dev/ skill for:

- Writing the fix (adding a Signer constraint, swapping AccountInfo for
  Account<T>, pinning a CPI program id, replacing raw math with checked_*, etc.).
- Anchor program structure, account context design, build/deploy.
- Transaction construction and signing.
- Re-running the program's own test suite after the change.

Why: the auditor must stay an independent reviewer. The report says WHAT is wrong,
WHERE, and the remediation DIRECTION; ../solana-dev/ implements it. Each finding in
templates/report.md ends with a remediation pointer, not a finished patch.

Handoff payload to ../solana-dev/: file:line + commit, the failing invariant, the
LiteSVM PoC that reproduces it, and the recommended fix shape from
references/vuln-classes.md.

---

## What goes to QEDGen (FORMAL proofs: Kani / Lean)

This skill explicitly COMPLEMENTS, and does not duplicate, the kit-bundled
QEDGen/solana-skills (formal verification) and trailofbits/ghostsecurity (security
guidance). For mathematically proving a property holds for ALL inputs, route to the
QEDGen skill:

- Kani bounded model checking of arithmetic/state invariants.
- Lean proofs of protocol-level properties (supply conservation, no-mint-without-burn,
  monotonic accounting, liquidation soundness).

When to escalate (see references/methodology.md "When to escalate"):
- A property must hold universally, not just on tested inputs.
- High-TVL protocol with a subtle/adversarial arithmetic invariant.
- A SUSPECTED overflow/rounding issue that resists reproduction but cannot be ruled out.

Dynamic analysis here proves a bug EXISTS by example (one PoC). Formal verification
proves a bug is ABSENT across a bounded space. They are complementary: use PoCs to
confirm findings fast; escalate to QEDGen when "we tested it" is not strong enough.

---

## One-line routing

- Need a fix / program change / signing -> ../solana-dev/
- Need a proof for all inputs (Kani/Lean) -> QEDGen skill
- Need security best-practice guidance -> trailofbits/ghostsecurity
- Need the audit workflow, a PoC, or a report -> stay here (solana-auditor)
