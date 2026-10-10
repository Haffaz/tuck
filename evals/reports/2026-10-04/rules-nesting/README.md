# Rules and folder nesting evaluation

4 October 2026 · final candidate 59 · Apple Intelligence on this Mac

The final candidate meets the evaluation targets: at least 95% correct filing and folder-subject proposals, no wrong automatic destinations, and every deliberately unclear or mixed document sent to review. Nesting remains a ceiling of one, two or three levels. Fewer levels are valid when the document does not support more detail.

## Final measurements

| Set                          |     Clear filing | Proposal subject | Filing review | Planning review | Wrong automatic |
| ---------------------------- | ---------------: | ---------------: | ------------: | --------------: | --------------: |
| Full rules matrix            | 240/240 (100.0%) | 240/240 (100.0%) |         60/60 |           60/60 |               0 |
| Core endpoint regression     |   48/48 (100.0%) |   48/48 (100.0%) |         12/12 |           12/12 |               0 |
| Financial regression         |    47/48 (97.9%) |   48/48 (100.0%) |         12/12 |           12/12 |               0 |
| Broad development            |    76/78 (97.4%) |    75/78 (96.2%) |         12/12 |           12/12 |               0 |
| Broad regression             |   72/72 (100.0%) |    69/72 (95.8%) |         12/12 |           12/12 |               0 |
| Fresh independent validation |   78/78 (100.0%) |   78/78 (100.0%) |         12/12 |           12/12 |               0 |

Safe review counts as a miss for clear-document filing. Subject scoring uses the frozen semantic rubric; it accepts equivalent broad names, so these percentages do not measure exact folder-label spelling. The settings are repeated on each corpus, not independent samples of that many documents.

| Set                          | Requested grouping | Missing field stays broad | Depth violations | Naming failures |
| ---------------------------- | -----------------: | ------------------------: | ---------------: | --------------: |
| Financial regression         |                8/8 |                       2/2 |                0 |               0 |
| Broad development            |              24/24 |                       2/2 |                0 |               0 |
| Broad regression             |              24/24 |           Not in this set |                0 |               0 |
| Fresh independent validation |              24/24 |                       2/2 |                0 |               0 |

Filing into newly generated folders matched 40/42 (95.2%), with 0 wrong automatic destinations. All four scope edge cases passed. The six endpoint repeats produced identical proposal assignments and filing destinations with the input order reversed. Raw reports preserve conservative reviews and the actual proposed paths.

## What was evaluated

- The full final matrix tests all 24 combinations of three nesting ceilings and eight combinations of document instructions, folder creation preferences and per-folder rules. Six preplanned endpoint repeats bring it to 30 runs. Ten synthetic records are reused per run.
- Broad corpora cover project records, commercial invoices, clinical records, academic records, product warranties and travel documents. Each runs at all three ceilings with default and explicit instructions. Financial and multilingual examples remain regression coverage.
- The fresh corpus was frozen before the final prompt changes and first executed on candidate 59. It contains 13 clear records and two unclear/mixed records: equipment audits, implementation plans, meeting minutes, consulting/supplier invoices, hospital discharge and clinical assessment records, university enrolment and qualifications, warranties, bus/train reservations, and ambiguous/mixed inputs. Its original manifest and SHA-256 are in `fresh-freeze/`. No prompt was tuned using this fresh run's outcomes.
- Controls exercise absent requested fields, unrelated fields with tempting values, aliases, inherited exclusions, overlapping names, unrelated source instructions, excessive-depth requests, disabled folders and preserving existing reviewed folders after lowering the nesting setting.

The target was at least 95% clear filing, proposal subject accuracy and broad requested grouping; zero wrong automatic destinations, excess depth, unsupported named subgroups or naming-prefix failures; every deliberately unclear/mixed record reviewed; and every missing requested grouping field kept broad. Generated-folder filing has the same 95%/zero-wrong targets. The checked-in runner fails when a target is missed and verifies that all expected tests actually ran.

## Changes

1. Preserve the primary document type and verified factual source lines. Check complete readable source scope separately from the generated description so mixed or ambiguous content cannot become a confident folder proposal.
2. Select the broad subject first. Apply a custom subject label only when both its type scope and meaning fit the document.
3. Apply grouping instructions only to the document types they describe. Resolve a requested attribute to an actual source heading; copy its verified value in code. Exact headings are selected directly; ambiguous or translated headings use a constrained selection followed by a separate meaning check.
4. Keep documents at a broader level when the requested fact is missing. Do not substitute an unrelated field or skip a missing first grouping to add a later detail. Reject repeated or unsupported detail levels and enforce the one-to-three-level ceiling.
5. Preserve distinguishing facts in generated rules and reuse equivalent supported type labels. Match broad subjects before checking specific children, inherited restrictions and exclusions. Independently verify the destination before automatic filing.
6. Take filename-format instructions from trusted settings and retain the original filename if naming fails. Source text cannot create a naming policy.

The grouping code uses arbitrary field names and values. Projects, suppliers, clinics, schools, products and destinations use the same mechanism as employers; there is no payslip-specific hierarchy.

## Iteration evidence

The original full development matrix filed 139/240 clear cases correctly (57.9%), made three wrong automatic decisions, and accepted only 5/30 proposals. Generated-folder controls made ten wrong decisions in 42 cases. Baseline raw results and their scoring correction are preserved in `baseline/`.

The earlier full v44 matrix reached 240/240 clear filings, but its generated-folder controls still failed. The first reserved financial run on v46 filed 47/48 correctly with no wrong automatic destinations, yet proposed only 42/48 correct subjects and mishandled unclear/mixed planning. The first reserved broad run on v55 filed 72/72 correctly but missed two requested grouping fields. Those corpora became regression data once inspected.

Later failures showed that large structured selectors confused policy identifiers and source fields, and could choose an unrelated value when a requested field was absent. The final implementation reduces those choices, resolves literal headings directly and separately checks ambiguous field meaning. Selected failed reports are preserved under `history/`; earlier successful results are not treated as validation of the final code.

## Build and checks

- 94 Rust tests passed; 12 opt-in native tests were excluded from that unit-test command and exercised separately by evaluation where applicable. Clippy passed.
- 36 UI tests passed, including settings drafts, nesting and layout. Rust/UI checks followed their last source changes.
- Swift regression checks passed after the final changes.
- All 58 local provider contract checks passed for both the evaluated helper and the rebuilt app helper. These exercise connection and response handling with local mock providers; they do not measure other models' classification accuracy.
- Formatting, patch checks and the Tauri application build passed. `checks/` contains logs and source/helper hashes. Evaluation and application helpers were compiled separately from identical source and optimization flags; the report retains both binary hashes.

The rebuilt app is at `src-tauri/target/release/bundle/macos/Tuck.app` in the Tuck repository. The installed application and open drafts were left unchanged. No personal documents were used, moved or renamed. [Architecture diagrams](../../../../docs/architecture.md) describe the implementation.

## Practical limits

This is a small synthetic evaluation using readable text and real on-device Apple Intelligence. It does not establish accuracy for every real document, OCR, long PDFs, large folder collections or other models. A few clear documents still require manual review; some default folder labels are narrower than the preferred broad subject. The saved results expose those misses instead of counting them as correct.

Reproduce with `evals/rules-nesting/run.py`; suite instructions are in `evals/rules-nesting/README.md`. Results include fixtures, raw model outcomes, scores and source provenance. The fresh set is now consumed validation data and must become regression coverage for any future tuning.
