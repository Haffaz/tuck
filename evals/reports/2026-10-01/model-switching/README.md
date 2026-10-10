# Model switching verification — 2026-10-01

## Scope

Same frozen 16 synthetic fixtures from [the workflow evaluation](../workflow-refactor/fixtures.json): 14 clear/expected destinations and two intentionally unclear documents, including Swedish/English, rendered scans, a PDF, an embedded instruction and a later text chunk. No personal documents were used or moved. Proposals only; PDFKit/Vision extraction is shared. Each model performs its own description, matching, verification and naming. Gemma never calls Apple Intelligence as a fallback.

| Model | Correct automatic / 14 clear | Wrong automatic | Needs input / 16 | Automated name checks / 16 | Median pipeline |
| --- | ---: | ---: | ---: | ---: | ---: |
| Apple | 12 | 0 | 4 | 12 | 3.02 s |
| Gemma E2B | 14 | 0 | 2 | 15 | 3.52 s |
| Gemma E4B | 13 | 0 | 3 | 14 | 6.60 s |

Both intentionally unclear cases stayed for input under every model. Apple also withheld the Swedish bank statement and the historical contract/application-bundle fixture; E4B withheld the latter. The contract fixture has competing purposes, and its expected type-based folder is explicitly predeclared. Keyword-based filename checks are imperfect and are not a human naming-quality score. All models missed the payslip name check; no forbidden title claims were detected.

These are small regression runs, one per model, not a general accuracy or controlled performance comparison. Build activity and cache/model residency were not controlled. Gemma tags are the local validation builds identified by weights digests in `smoke.json`; the same tag elsewhere need not have identical weights. Input is extracted text, not direct document images.

## Other gates

84 Rust tests, 33 UI tests, Swift role-contract checks, clippy with warnings denied, and a release app build passed. Settings checks cover legacy Apple defaults, unknown models, model persistence, revision invalidation, and stopping automatic filing on model change. UI checks cover draft selection, explicit Save/reload, the selected model reaching the backend, and an actionable connection error without fallback.

The three-model smoke run covered actual readiness generation, rule suggestions and category planning. Every response passed its structural contract; unknown model keys were rejected. Category quality still varies: E2B suggested overlapping Insurance and Travel categories for this small collection, while E4B separated home/travel insurance. Schema validity does not prove an ideal taxonomy.

## Issues caught

The first Gemma run could classify but returned invalid free-JSON filenames. Schema-constrained naming fixed that; the same schema mechanism now protects Gemma category planning. Apple retained its measured guided/free-JSON role choices.

Apple temporarily reported available while all generation failed with `generation_failed`. The previously installed helper failed the same synthetic request too. After releasing test models and allowing time, Apple recovered; this does not prove the cause. Check connection now performs a tiny synthetic generation so availability alone is not presented as working inference. Cache identity reads remain metadata-only. No system services or Apple settings were changed.

## Reproduce

Run the ignored `evaluation::tests::live_apple_accuracy_eval` Rust test with `TUCK_EVAL_MODEL=apple`, `gemmaE2b`, or `gemmaE4b`, `TUCK_EVAL_FIXTURES` set to the absolute fixture path above, and `TUCK_EVAL_REPORT` set to an output path. The historical test name is retained, but provider selection is explicit. It reports quality rather than asserting every prediction is correct. Run models serially.

`python3 evals/model-switching-smoke.py /absolute/path/to/tuck-apple /private/tmp/smoke.json` exercises the other operations with synthetic input. Tuck does not download models or use cloud providers.

## Delivery

The final release was signed, installed at `/Applications/Tuck.app`, and reopened. Signature and every installed bundle file matched the built artifact. All 25 personal source files were unchanged across installation. The previous app is backed up at `/private/tmp/tuck-before-category-update-btgefsn_/Tuck.app`; the unapproved category draft was preserved at `/private/tmp/tuck-unsaved-categories-before-model-update.json`. The user started a category-suggestion run after launch; that run was left alone. No filing was started by the update.
