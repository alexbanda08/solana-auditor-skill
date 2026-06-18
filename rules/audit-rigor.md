---
globs:
  - "**/*.md"
---

# Audit Rigor Standard

These rules apply to all Markdown files produced during a solana-auditor skill session
(methodology notes, finding write-ups, report drafts, checklist completions, PoC notes).
Claude MUST enforce them at generation time. Violation is a quality gate failure.

---

## 1. Never assert a program is "safe" or "secure"

Do NOT write phrases such as:
- "the program is safe"
- "no vulnerabilities found"
- "this code is secure"
- "the account is safe to use"
- "no issues detected"

A completed audit reduces known risk within a defined scope. It does not certify absence
of all vulnerabilities. Use precise language:

  WRONG: "The escrow program is secure."
  RIGHT: "No issues were identified in the escrow program within the audit scope
          (commit abc1234, components: escrow.rs, state.rs). Untested areas: admin
          upgrade path, oracle feed validation."

---

## 2. Every finding requires evidence, reproduction steps, and a severity rating

A finding write-up MUST include ALL of the following fields before it is considered complete:

  - Title         : short noun phrase naming the issue
  - Severity      : one of Critical / High / Medium / Low / Info  (see scale below)
  - Status        : Confirmed | Suspected | Acknowledged | Fixed | Won't Fix
  - Location      : file path + line number(s) or account/instruction name
  - Description   : what the vulnerability is and why it is dangerous
  - Evidence      : code snippet, log output, or on-chain transaction showing the issue exists
  - Reproduction  : numbered steps an independent auditor can follow to reproduce
  - Impact        : what an attacker can achieve (fund loss, data corruption, DoS, etc.)
  - Remediation   : concrete fix pointer (code change, constraint addition, delegation to solana-dev)

Omitting any field is a draft, not a finding. Mark incomplete write-ups with [DRAFT - missing: <field>].

---

## 3. Use the fixed severity scale - no invented levels

| Level    | Criteria                                                                              |
|----------|---------------------------------------------------------------------------------------|
| Critical | Direct fund loss or complete account takeover; exploitable without preconditions      |
| High     | Significant fund loss or privilege escalation; requires attacker-controlled account   |
| Medium   | Partial fund loss, griefing, or logic bypass; limited scope or preconditions required |
| Low      | Best-practice deviation; no direct exploit path identified                            |
| Info     | Informational observation; no security impact; style or optimization note             |

Do NOT use: "Major", "Minor", "Severe", "Moderate", "Negligible", or numeric scores
without mapping to this scale.

---

## 4. Distinguish Confirmed vs Suspected findings

- Confirmed  : you have a working reproduction (PoC test passing, on-chain tx, or
               static evidence that is unambiguous).
- Suspected  : the code pattern is present and is a known risk class, but a full
               end-to-end exploit has not been demonstrated in this engagement.

Always state which applies. Do NOT promote a Suspected finding to Confirmed without evidence.

---

## 5. Scope discipline

Every document that states a finding or a clean-bill outcome MUST declare:
- The audit scope (repository, commit hash or tag, file list)
- What was NOT audited (explicitly out-of-scope items)

Template line:
  Scope: <repo> @ <commit/tag> | In: <files/components> | Out: <excluded items>

---

## 6. Citation and traceability

- Reference source lines: `src/processor.rs:142-158`
- Reference Solana/Anchor docs or advisory IDs where applicable
- Do not cite paywalled sources without a free mirror or direct quote

---

## 7. Remediation pointers - delegate, do not duplicate

For fixes that require program code changes, write:
  "See ../solana-dev/ for implementation guidance on <fix type>."

For formal proof requirements (Kani/Lean), write:
  "Formal verification of this invariant should use QEDGen/solana-skills."

Do not author replacement program code inline in audit documents.

---

## 8. Prohibited language in audit documents

Do not use:
- "simply", "just", "obviously", "trivially" (minimizes severity perception)
- "might be vulnerable" without stating the specific condition
- "TODO", "TBD", "fill in later" in any published finding
- Passive constructions that obscure agency: prefer "an attacker can drain funds"
  over "funds could potentially be drained"
