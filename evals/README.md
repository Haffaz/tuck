# Apple pipeline evaluations

Run from the repository root:

```sh
TUCK_EVAL_REPORT=/private/tmp/tuck-apple-eval.json \
  cargo test --manifest-path src-tauri/Cargo.toml live_apple_accuracy_eval -- --ignored --nocapture
```

Requires Apple Intelligence enabled on the Mac. These are synthetic text fixtures only. The test calls the actual classifier with saved rules and bounded Apple document tools, but does not move documents. The test passing means the evaluation ran, not that model quality met a release threshold. No OCR quality claim is made.

## Initial Apple-only baseline

Recorded in [results/apple-baseline.json](results/apple-baseline.json).

| Measure | Result |
| --- | --- |
| Expected folder | 11/12 |
| Automated title checks, including two expected review cases | 5/12 |
| Automated title checks for the ten substantive documents | 3/10 |
| Expected unclear cases sent to review | 2/2 |
| Forbidden title claim | 1/12 |
| Median end-to-end classification time | 0.984 seconds |

Manual review accepts `Rent Agreement` as an equivalent tenancy title; the strict concept list did not count that wording. With that judgment, substantive title acceptance is 4/10. Keep the original automated metric unchanged so this interpretation is visible.

Observed failures:

- Residence permit title omitted the card distinction.
- Bank statement, payslip, and home-insurance titles were too broad.
- An employment-contract attachment was named after its application bundle.
- An electricity invoice was assigned to Finances instead of Home.
- An instruction embedded in an insurance footer caused the title `Annual Bonus`; a keyword rule still chose Insurance.
- The two review outcomes followed tool/validation failures; they do not establish that model confidence is well calibrated.

This is a small, known development set. Keyword rules contribute strongly to folder accuracy. No comparison to the historical direct-model benchmark should be presented as a measured improvement, because that benchmark used different prompts and omitted the current pipeline's rules and tools. Create held-out cases before claiming broader accuracy gains.

Next experiments are described in [the AI accuracy plan](../docs/ai-accuracy.md). Keep raw outputs, explicit acceptance criteria, manual-review adjustments, latency, and abstention rates with future reports.

## Semantic folder instructions — 2026-09-27

Recorded in [results/apple-semantic-instructions.json](results/apple-semantic-instructions.json).

| Measure | Result |
| --- | --- |
| Expected folder | 9/12 |
| Automated title checks, including two expected review cases | 4/12 |
| Automated title checks for substantive documents | 2/10 |
| Expected unclear cases sent to review | 2/2 |
| Forbidden title claim | 1/12 |
| Median classification time | 1.417 seconds |

The old baseline's keyword overrides have been removed. The new prompts enable semantic custom filing and source-folder associations, but this run regresses on the known general set: a residence card went to Migration/AGENCY, an electricity invoice to Finances, and an injected insurance footer produced Work / Annual Bonus. Many titles remain too generic. These are unresolved model-quality limits, not passing quality checks. This is a known development set; no broad accuracy improvement is claimed.

Separate live assertions passed for a AGENCY employment attachment routed to Migration using only parent-folder context, disabling that instruction, a tenancy description without literal keyword matching, legacy keyword migration, an unrelated Incoming parent, a AGENCY filename without body/parent context, Swedish banking, insurance, unreadable review, and reading evidence in text chunk 5. These tests use synthetic text and do not move personal documents.
