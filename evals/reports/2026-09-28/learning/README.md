# Explicit filing examples — 2026-09-28

Synthetic documents only, macOS 27.0, Apple's on-device model. Filing-v4 adds explicit user-approved examples in folder Markdown and scoped duplicate-rule checks. These small, single-pass tests do not establish general accuracy or eliminate earlier refusals. No personal document was processed.

## Example comparison

| Set                               | Before examples      | With examples        |
| --------------------------------- | -------------------- | -------------------- |
| Six comparison cases              | 6/6 expected folders | 6/6 expected folders |
| Two new preference-transfer cases | 1/2 desired folders  | 2/2 desired folders  |

The six-case set includes five identifiable documents and one expected abstention. Both final runs classify all five correctly and abstain on the fragment. Automated title checks are 5/6, including the abstention. The AGENCY statement keeps its original name in both runs. Median classification latency is 2.52 seconds before and 2.65 seconds with examples.

In the transfer set, the new example supplies a preference missing from the baseline: employment certificates from the Aster application source folder belong with that application in Migration. The baseline files both certificates in Work. With the example, the application certificate goes to Migration and the unrelated Incoming certificate stays in Work. This demonstrates use of additional user preference information, not a like-for-like increase in general model capability. The bundle checker withholds the application certificate's generated name, reducing automated title checks from 2/2 to 1/2. Median latency is 3.28 seconds before and 3.00 seconds with examples; single-pass timings include normal variation.

No wrong destination or forbidden filename claim was emitted with the final examples in these eight cases. Earlier general classification failures remain unresolved.

## Rejected example handling

Simply appending positive and negative examples without explicitly scoping them caused two abstentions (4/6 expected outcomes). A short, conditional routing instruction now keeps a negative example local to its named folder and tells the model to consider other destinations. The same six cases then returned to 6/6. Those six are tuning/regression cases; the two transfer cases were authored after choosing that prompt and run subsequently without further prompt changes.

Rules without the generated example heading retain the earlier routing prompt. Reading evidence and filename templates are unchanged. Examples are user descriptions, not document facts or a training dataset; they cannot provide the evidence quote required for renaming. Reports keep actual measured messages and pipeline labels. The installed helper's filing-v4 label was updated after measurement without changing document-classification behavior.

## Broad AI rule auditing: not shipped

Several live audit variants were tested against obvious overlap, distinct categories and explicit exclusions. Initial generation produced false conflicts. A numeric screening step passed the original three pairs but managed only 1/3 new pairs: it missed Car/Insurance overlap and incorrectly flagged Education/Utilities. A simpler boolean screen returned clear on everything, missing real overlaps. A candidate-then-pair-verification variant flagged all six pairs, including distinct and explicitly resolved ones. Generated explanations sometimes contradicted their own verdicts.

These results were not reliable enough for a useful semantic audit, so its model calls and command path were removed from production. Reports retain failed variants. The shipped **Check rules** feature checks duplicate descriptions among enabled siblings deterministically, and explicitly tells users that different wording can still overlap. There is no claim that it detects all semantic conflicts.

## Persistence and checks

**Remember this example** appears after a manual preview correction, including after organizing that corrected item in the current session. Descriptions are editable, with an optional counterexample. **Add to rule draft** performs no write; **Save rules** uses the existing revision-checked, journaled Markdown save. Examples survive restart; the batch does not. Saved changes require fresh document analysis. No document contents, paths or transcripts are automatically persisted.

Rust checks cover read-only duplicate checks, disabled and inherited scope, stale revisions, persistence across restart and rejection of outdated organization approvals. UI checks cover explicit saving, cancellation, editable clarification drafts and stale result handling. No database or additional model provider was introduced.

## Verification

55 Rust tests and 18 UI tests pass. TypeScript/Vite, Clippy with warnings denied, and diff whitespace checks pass. Swift support checks were unchanged; the native helper compiled successfully throughout the live tests and final build.

## Reproduce

```sh
TUCK_EVAL_FIXTURES="$PWD/evals/learning-baseline.json" TUCK_EVAL_REPORT=/private/tmp/tuck-before.json cargo test --manifest-path src-tauri/Cargo.toml live_apple_accuracy_eval -- --ignored --nocapture
TUCK_EVAL_FIXTURES="$PWD/evals/learning-examples.json" TUCK_EVAL_REPORT=/private/tmp/tuck-after.json cargo test --manifest-path src-tauri/Cargo.toml live_apple_accuracy_eval -- --ignored --nocapture
```

Use `learning-transfer-baseline.json` and `learning-transfer-examples.json` for the new preference cases. The removed AI rule-audit prototypes are historical evidence, not a callable production operation.
