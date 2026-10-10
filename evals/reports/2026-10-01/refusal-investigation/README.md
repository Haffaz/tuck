# Apple refusal investigation — 2026-10-01

## Scope and status

All inputs were synthetic and inference stayed on this Mac (macOS 27.0). No personal documents were moved. After explicit user approval, the production source now uses Apple's content-transformation mode for the five document roles below. Automatic approval review initially blocked the change because it skips runtime content checks; user approval resolved that blocker. The [reviewed patch](proposed-transformation-mode.patch) has been applied. The final evaluation uses the rebuilt production helper, not the experimental helper. The installed release app was not rebuilt or relaunched.

## Observations and causes

1. The existing guided-output pipeline refused on ordinary bank, insurance and category prompts. Diagnostic refusal explanations said “May contain sensitive content” and sometimes included malformed attempted JSON/schema fragments. These were diagnostic material only, never accepted as model results.
2. Reducing the bank match from seven destinations to three succeeded in a controlled probe. Shorter rules, fewer schema fields and schema-wording changes did not reliably fix the full task. This implicates prompt/output-path interaction; it does not establish a seven-folder limit or prove an Apple framework defect.
3. Ordinary string generation with a concise JSON contract greatly reduced these failures. It also made unsupported folder choices in early variants, so strict output validation and a separate semantic verification call were necessary. Verification uses a fresh session of the same model; it is not an independent authority.
4. Default runtime checks still blocked harmless synthetic content, including a bank statement during category setup. Apple's refusal explanations do not reveal an exact triggering token, so the internal cause remains unknown.
5. Additional-read handoff could keep the cover page dominant and allow an application-generated omission marker to become a quote. Source quote generation now excludes that marker; latest page content comes first; a single explicit source page/chunk reference can resolve an unspecified read. A focused retest actually read chunk 5 and correctly described the maintenance report.

## Implemented source fixes

- Small per-role string-generation prompts, followed by strict JSON/YES-NO parsing. Refusals and malformed responses remain errors.
- Model-selected quote/source IDs map only to supplied evidence. IDs may be integers or canonical decimal strings; booleans, arbitrary strings and out-of-range references fail.
- A fresh folder verification call follows every supported match. Rejection or error prevents automatic filing; optional naming occurs only afterward.
- Explicit uncertainty stays uncertain even when its explanation is empty.
- Read capabilities reflect actual document type and remaining evidence. Current-page handoff and source quote handling were corrected.
- Provider interface now includes verification; role/helper identity invalidates stale description caches. Existing serial execution, time/read budgets and Rust file checks remain.

## Same 16 controlled cases

There are 14 clear documents and two intentionally unclear inputs: 12 regression texts, two rendered images, one scanned PDF and a later-chunk text document. Some content is repeated across modalities. Expected folders were set in fixtures before grading; semantic names were checked against the synthetic source. Broad correct labels such as “insurance policy” are acceptable. Exact phrase checks in JSON are diagnostic, not the final semantic grade.

| Outcome | Original pipeline | Source fixes, default guardrails | Temporary transformation mode |
| --- | ---: | ---: | ---: |
| Clear documents correctly filed | 2/14 | 10/14 | 12/14 |
| Clear documents incorrectly filed | 0 | 0 | 0 |
| Clear documents needing input | 12 | 4 | 2 |
| Unclear documents needing input | 2/2 | 2/2 | 2/2 |
| Median full document time | 3.41 s | 4.78 s | 3.72 s |

These are single runs of a small known regression set, not a held-out accuracy estimate. Baseline latency includes early failures and is not directly comparable to successful full processing. JSON reports include stage outputs and timing. The default-guardrail full run predates the final page-reference fix; [the focused page retest](page-retest.json) verifies that fix. The temporary transformation run includes it.

The default-guardrail category setup test still failed because only two of three synthetic descriptions were accepted. Replaying the original 11-description category request succeeded after prompt changes. The temporary transformation-mode category setup passed all three documents, created safe category folders and preserved source hashes.

## Final production verification

The approved source was rebuilt and all 16 fixtures were rerun. Results: **12/14 clear documents correctly filed**, 0 wrong automatic destinations, 2 clear documents needing input. Both intentionally unclear documents stayed in the fallback. Median document time was 3.81 seconds. All stage error counts were {'describe': 0, 'match': 0, 'verify_match': 0, 'name': 0}. No refusal or guardrail error occurred in this run. These observations do not guarantee future inputs cannot be refused.

See [per-document decisions and stage outputs](approved-production.json), [category setup test](approved-category-setup.log) and [helper identity](approved-build.json). The production helper keeps default policies for rule suggestions, language checks and legacy operations; transformation mode is selected only for `describe`, `categories`, `match`, `verify_match` and `name`.

## Applied Apple mode — explicitly approved

Apple documents `SystemLanguageModel(guardrails: .permissiveContentTransformations)` for transforming source text, including tagging. It applies to **string generation only**: runtime content checks are skipped, while the model can still refuse. Guided generation keeps default behavior. See [Apple safety documentation](https://developer.apple.com/documentation/foundationmodels/improving-the-safety-of-generative-model-output) and [the guardrails API](https://developer.apple.com/documentation/foundationmodels/systemlanguagemodel/guardrails/permissivecontenttransformations).

The applied patch scopes this mode to description, categories, matching, verification and naming. Rule suggestions, language checks and legacy operations retain the default. It keeps strict output validation, semantic checking and Rust file-operation checks. The temporary full evaluation had no refusal or guardrail errors; that does not guarantee future inputs will never be refused.

## Remaining issues

- The verifier rejected an electricity bill and scanned employment certificate that should have been filed. This is conservative but causes unnecessary manual work.
- The image insurance filename “Home Insurance Warranty” is inaccurate; AGENCY naming was broader than the attachment's type. Correct folder selection does not establish correct naming.
- The unreadable case requested an unavailable read and safely fell back in the recorded runs. A subsequent contract-tested fix converts that request to an explicit uncertain result with a friendly request to choose a folder; it cannot execute the read or accept a destination.
- Same-model verification can repeat a mistake. The zero wrong-folder count here is not a general guarantee.

## Verification and reproduction

Final verification: 80 Rust tests passed, nine live tests ignored; Clippy passed with warnings denied; standalone Swift contract tests passed. Live category setup passed against the rebuilt production helper, including all three synthetic documents and unchanged source hashes. The earlier default-guardrail failure remains recorded above. No assertion was weakened.

```sh
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
swiftc -parse-as-library src-tauri/swift/TuckRoleValidation.swift evals/role-contract-tests.swift -o /tmp/tuck-role-contract-tests
/tmp/tuck-role-contract-tests
TUCK_EVAL_FIXTURES="$PWD/evals/reports/2026-10-01/refusal-investigation/fixtures.json" TUCK_EVAL_REPORT=/tmp/tuck-refusal-eval.json cargo test --manifest-path src-tauri/Cargo.toml --lib live_apple_accuracy_eval -- --ignored --nocapture
cargo test --manifest-path src-tauri/Cargo.toml --lib live_apple_category_suggestions -- --ignored --nocapture
```

The broad evaluation records quality mismatches without failing its completion assertion; inspect the per-case report rather than treating a successful exit as an accuracy gate. The category setup test has stricter assertions and passes with the approved production configuration. Tests do not change the selected guardrail policy.
