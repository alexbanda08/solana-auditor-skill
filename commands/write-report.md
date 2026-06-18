---
description: Assemble raw audit findings into a severity-graded security report using the standard report.md format.
---

# /write-report

Assembles raw findings from an audit session into a deliverable,
severity-graded security report. Invoke as:

```
/write-report [--findings <file>]
```

`--findings <file>` - optional path to a markdown or text file containing raw
findings. If omitted, findings are read from the current conversation context
(paste them inline after invoking the command).

This command is executed by `agents/audit-report-writer.md`. It consumes
findings from `/audit-program` phases 2-4 and produces the final report.

---

## Step 1: Load the report contract

1. Read `skill/references/report-template.md` in full. This is the authoritative
   source for:
   - The five-level severity scale (Critical / High / Medium / Low / Info)
   - Required fields per finding (ID, Title, Severity, Status, Location,
     Description, Reproduction, Impact, Remediation)
   - Executive Summary requirements
   - Scope and Limitations section content

2. Open `skill/templates/report.md` as the structural skeleton. The final
   output must conform to this skeleton's section order.

---

## Step 2: Ingest and deduplicate findings

3. Collect all raw findings from the source (conversation context or
   `--findings` file). Sources may include:
   - Triage rows from `/static-scan`
   - Manual review notes from `/audit-program` Phase 3
   - PoC results from `/audit-program` Phase 4
   - Any ad-hoc notes from the auditor

4. Deduplicate: if two raw findings describe the same root cause at the same
   location, merge them into one finding. Record both sources in the
   Description field ("Also identified by cargo-audit RUSTSEC-... and manual
   review of src/lib.rs:42").

5. Group findings by severity bucket in this order:
   Critical -> High -> Medium -> Low -> Info

6. Within each bucket, sort by impact descending (most fund-threatening first).

7. Assign a finding ID to each:
   - Format: `<SEVERITY_PREFIX>-<ZERO_PADDED_INDEX>` within that bucket
   - Prefixes: `CRITICAL`, `HIGH`, `MEDIUM`, `LOW`, `INFO`
   - Example: `CRITICAL-001`, `HIGH-001`, `HIGH-002`, `MEDIUM-001`

---

## Step 3: Write each finding block

8. For every finding, produce a block with exactly these fields. Do not omit
   any field; write "N/A" only if a field is genuinely not applicable.

   ```markdown
   ### [SEVERITY-INDEX] Short Title (noun phrase, under 60 chars)

   **Severity:** Critical | High | Medium | Low | Info
   **Status:** Confirmed | Suspected
   **Location:** `path/to/file.rs` lines X-Y  (or instruction name if no single location)

   **Description:**
   <What the bug is. Why it is exploitable. Reference the vuln class from
   references/vuln-classes.md by name (e.g. "missing signer check").>

   **Reproduction:**
   <Exact steps or PoC code to trigger the issue. For Confirmed findings
   include the litesvm PoC file name or the literal transaction construction.
   For Suspected findings describe what an attacker would need to do and why
   it is believed to succeed.>

   ```rust
   // minimal PoC sketch or reference to templates/litesvm-harness.rs
   ```

   **Impact:**
   <Concrete on-chain consequence: unauthorized fund transfer, account
   takeover, data corruption, DoS, etc. Quantify if possible (e.g. "attacker
   drains up to X SOL from the vault").>

   **Remediation:**
   <Pattern fix, not patch code. Reference the how-to-fix guidance in
   references/vuln-classes.md for this vuln class. Note that implementation
   is delegated to the solana-dev skill (see references/delegation.md).>
   ```

9. For Suspected findings add a Confirmation note:

   ```markdown
   **Confirmation note:** To upgrade to Confirmed, build a litesvm PoC
   (scaffold: templates/litesvm-harness.rs) that demonstrates <specific
   precondition>. Alternatively, trace through <instruction path> manually
   to verify that the signer check is absent in all branches.
   ```

---

## Step 4: Write the Executive Summary

10. The Executive Summary must appear before the Findings section and include:
    - Program name and version (commit hash or binary hash if available)
    - Audit dates (start and completion)
    - Auditors (or "Claude Code solana-auditor skill")
    - Finding count by severity: e.g. "1 Critical, 2 High, 3 Medium, 0 Low, 2 Info"
    - One sentence on overall risk posture. Do NOT use the words "safe",
      "secure", or "no vulnerabilities." Use: "The program presents a
      [high/moderate/low] attack surface given the findings above and the
      scope below."
    - Recommendation to remediate Critical and High findings before any
      mainnet deployment or TVL increase.

---

## Step 5: Write Scope and Limitations

11. The Scope and Limitations section must state:
    - Which program(s) and which instructions were reviewed
    - Which were explicitly out of scope (off-chain components, client code,
      governance, upgrade keys if not reviewed)
    - Tool versions used (load `references/sdk-versions.md` if not already
      in context)
    - Time-box (hours or days) if known
    - Statement: "This audit reduces risk over the defined scope; it does not
      certify the absence of vulnerabilities."

---

## Step 6: Assemble the full report

12. Assemble sections in this order following `templates/report.md`:

    ```
    # Security Audit Report: <Program Name>

    ## Executive Summary
    ## Scope and Limitations
    ## Methodology
    ## Findings
       ### [CRITICAL-001] ...
       ### [HIGH-001] ...
       ...
    ## Appendix A: Static Analysis Logs
    ## Appendix B: PoC Source Files
    ## Appendix C: Tool Versions
    ```

13. Appendix A: embed or reference `audit.log`, `clippy.log`, `deny.log`,
    `geiger.log` produced by `/static-scan`.

14. Appendix B: list every litesvm PoC file by name and path.

15. Appendix C: paste the pinned versions block from `references/sdk-versions.md`.

---

## Step 7: Review before delivery

16. Before finalizing, verify each finding:
    - [ ] Has a finding ID in the correct format
    - [ ] Has a Status (Confirmed or Suspected)
    - [ ] Has a non-empty Reproduction field
    - [ ] Has a non-empty Impact field
    - [ ] Remediation does NOT contain patch code (delegate only)
    - [ ] Severity matches the scale in `references/report-template.md`

17. Verify the Executive Summary does not contain "safe", "secure", or
    "no vulnerabilities found" anywhere.

18. Verify the Scope and Limitations section is present and non-empty.

19. Output the final report to stdout (or write to `AUDIT_REPORT.md` in the
    working directory if the user requests a file).

---

## Delegation

- Remediation implementation -> `references/delegation.md` -> solana-dev skill.
- Formal invariant proofs for fixes -> `references/delegation.md` -> QEDGen.
- New vulnerability discovery during report writing -> pause report, return to
  `agents/solana-auditor.md` for investigation, then resume.
