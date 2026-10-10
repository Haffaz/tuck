# Rules and folder nesting evaluation

This opt-in evaluation runs real Apple Intelligence through Tuck's production reader, folder planner, matcher, independent verifier and naming role. It uses synthetic text files in a disposable directory. It checks hashes and never saves preferences or moves documents.

Build the native helpers first, then run from the repository root:

```sh
npm run tauri build -- --bundles app
python3 evals/rules-nesting/run.py --suite diverse --out /private/tmp/tuck-diverse-new
python3 evals/rules-nesting/run.py --suite core --out /private/tmp/tuck-core-new
python3 evals/rules-nesting/run.py --suite heldout --pilot --out /private/tmp/tuck-heldout-new
python3 evals/rules-nesting/run.py --suite diverse-heldout --out /private/tmp/tuck-domain-holdout-new
```

Apple Intelligence must be enabled and ready on this Mac. Run suites serially. Cached Cargo dependencies are required; the runner uses offline Cargo. Scores are semantic outcomes, separate from Rust test process success. A failing quality target returns a nonzero exit code and preserves the report.

## Coverage

- **Core:** three nesting ceilings × eight combinations of document instructions, folder creation preferences and per-folder rules; six repeated endpoint combinations. Ten clear/unclear/mixed/injected records. Extra tests cover aliases, exclusions, matching names, mixed subjects and filing into generated rules.
- **Diverse:** three ceilings × default/explicit instructions. Fifteen project, invoice, clinical, academic, warranty and travel records, with missing details and unrelated pages.
- **Diverse held-out:** new projects, suppliers, health providers, schools, products and travel destinations, including previously unseen record types.
- **Held-out:** Swedish and English records originally reserved for validation. Its first run exposed planning failures, so it is now a regression corpus. The broad diverse held-out set was first executed on candidate 55; it is now a regression corpus. Reserve a new frozen corpus before using further outcomes to tune prompts.

A separately frozen corpus can use the same rubric with `--fixtures /absolute/path/fixtures.json`. The runner copies its inputs and records their hash and original location. A corpus becomes development data once its outcomes are inspected; keep the first independent result even when it fails.

## Acceptance

At least 95% of clear documents must reach their expected saved folder and 95% of usable proposal subject assignments must be correct. No incorrect automatic destination, accepted excess nesting or unsupported subgroup is allowed. Every deliberately unclear/mixed file must require review in both planning and filing. Explicit field grouping must use a supported value in at least 95% of the diverse guided cases. Requested filename prefixes must pass all naming checks. Repeating a grouping value as an extra detail level is also a failing check. Records missing an explicitly requested grouping field must stay in the broad subject; another observed field cannot substitute for it. Generated folders must also meet 95% recall with zero incorrect destinations; rule edge cases can conservatively require review. Scores include safe abstentions in the recall denominator.

The maximum of one, two or three levels applies to new suggestions. Existing reviewed rules survive lowering this setting. The fixtures are deliberately small and readable: these results do not establish accuracy for every document type, OCR, long PDFs or large folder collections. Adapter protocol coverage uses separate local mocks in `evals/connection-contract.py`.

## Latest results

The [4 October evaluation report](../reports/2026-10-04/rules-nesting/README.md) includes the final full matrix, broad document regressions, a fresh validation corpus, generated-folder checks and the retained failure history.
