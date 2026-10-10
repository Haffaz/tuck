# Filing-v3 prompt comparison — 2026-09-28

Synthetic documents only, macOS 27.0, Apple's on-device model. Small single-pass comparisons are regression evidence, not a general accuracy estimate. The saved checked-v2 helpers ran through the same Rust evaluation harness. No personal documents were processed.

## Outcomes

| Suite             | Checked-v2 | Filing-v3 | Median latency  |
| ----------------- | ---------- | --------- | --------------- |
| Existing 12 cases | 9/12       | 9/12      | 2.78 s → 2.77 s |
| Focused 5 cases   | 2/5        | 3/5       | 3.96 s → 2.88 s |
| New 6 cases       | 3/6        | 3/6       | 2.11 s → 2.43 s |

The existing suite includes two expected abstentions; the focused and new suites each include one. Count these separately from identifiable documents when interpreting results. Reports include every decision, title check, refusal and timing. A passing test runner means the evaluation completed, not that all documents were classified correctly.

No incorrect destination or forbidden title claim was emitted in the final three suites. Among identifiable cases, expected folders were selected for 7/10 in the existing set, 2/4 in the focused set and 2/5 in the new set. All expected abstentions remained correct. Remaining identifiable cases require manual choice.

Naming is more conservative: the new Cedar Services receipt retains its original name, while checked-v2 supplied the correct `Receipt` template. Automated title checks therefore fell from 3/6 to 2/6 on the new set, including its expected abstention. The AGENCY bank statement also keeps its original name. These are recorded tradeoffs, not an across-the-board quality improvement. General folder refusals, the later-chunk refusal, the unidentified maintenance log and the new injection case's abstention remain unresolved.

The final generic-label validation was tightened after the model comparison and covered by deterministic tests; it does not alter any source labels in these fixtures. The UI explanation for a withheld name was also clarified after measurement. Reports preserve the actual messages observed during each run.

## What changed

Source-folder matching has a single task: find a filing preference explicitly naming a supplied source group. Root preferences are supplied separately. The model returns a bounded numeric destination ID; code maps it to an allowed path and verifies a distinctive shared name with word boundaries. Generic source labels cannot establish a bundle association. This is validation after semantic selection, not keyword-based folder routing.

Bundle proposals still receive a fresh model check against separately scoped full rules, alternatives and document evidence. Its concise numeric verdict checks the folder and filename separately. A stated source-group filing preference takes priority over the attachment topic. A rejected name keeps the original filename; a rejected folder or model error requires manual choice. Other document routing and checker prompts remain at checked-v2.

The AGENCY source-folder bank statement now reaches the correct bundle folder; its checker conservatively rejects renaming, so the original name stays. This does not fix the separate AGENCY case with evidence in the document rather than a source-folder association. Refusals elsewhere remain possible.

## Rejected experiments

- Broadly rewriting both source matching and semantic routing reduced the existing suite from 9/12 to 4/12 and misrouted a payslip to Migration. Rejected (`candidate-*`).
- Numeric IDs in general routing scored 9/12 but misrouted home insurance to Home. Equal totals concealed a regression; rejected (`ids-*`).
- Source IDs alone moved the bundle refusal to the checker (`source-only`). Changing only the check schema still refused (`check-numeric`). Consistent, shorter instructions with the numeric check completed (`check-simple`).

A separate synthetic diagnostic requested Apple's refusal explanation. It generated text containing the expected destination, despite the original call failing. That explanation is another generation, not the original output or proof of the refusal's cause. No diagnostic logging was added to production and guardrails remain enabled.

The six new fixtures were authored before their first live run, then used for a baseline/candidate comparison after selecting the prompt. They become regression cases after this evaluation. One run cannot establish reliability across model updates or unfamiliar document collections.

## Build verification

52 Rust tests, 41 Swift support checks and 14 UI tests passed; TypeScript/Vite build, Clippy with warnings denied, and diff whitespace checks passed. Live model behavior is measured separately above. Existing macOS 27 deprecation warnings remain for APIs retained for macOS 26 compatibility.

## Reproduce

```sh
TUCK_EVAL_FIXTURES="$PWD/evals/prompt-validation.json" TUCK_EVAL_REPORT=/private/tmp/tuck-prompts.json cargo test --manifest-path src-tauri/Cargo.toml live_apple_accuracy_eval -- --ignored --nocapture
```

Use `evals/documents.json` or `evals/checker.json` for the other suites. `TUCK_EVAL_READER` selects a saved reader beside its matching Apple helper; `TUCK_EVAL_LABEL` labels the report. All file operations still require explicit user approval.
