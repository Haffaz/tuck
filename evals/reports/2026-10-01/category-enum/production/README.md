# Production guided category matching — 2026-10-01

The tested no-rules guided matcher is now part of the native Apple adapter, with generic starter categories. One document per call; category names only; one structured category literal or Needs input. No model-generated document or evidence IDs. Existing stored categories/rules are preserved. The verifier, naming stage and file-safety checks remain.

## Integrated native evaluation

All 16 synthetic cases ran through current production Rust/Swift code with fresh extraction, language preparation, description, matching, verification and optional naming. Two image fixtures and one PDF exercise native extraction/OCR. Another fixture requires text chunk 5. These are text inputs to the LLM after extraction, not direct image/PDF inference. No personal documents were read or moved; this harness produces decisions without organizing files. Expected labels were frozen before inference.

| Measure | Result |
| --- | --- |
| Expected raw category among 14 clear documents | 13/14 |
| Correct automatic proposals after verification | 11/14 |
| Wrong automatic proposals | 0 |
| Clear documents needing input | 3 |
| Intentionally unclear documents retained | 2/2 |
| Refusal/API/output-format errors | 0 |
| Median full pipeline time | 2.83 s |
| Maximum full pipeline time | 4.22 s |

Seventeen description calls (one additional page read), sixteen matches, fourteen verifier calls and eleven naming calls. The category planner was not exercised: the same seven categories were supplied for all cases. The live automatic-flow smoke passed: Start Tuck organized one synthetic bank statement with its hash unchanged, kept one unreadable file in place, and the watcher ignored its own output.

## Remaining limitations

- The historical immigration-bundle employment contract was matched by application purpose (Identity & Immigration), rather than the predeclared type label (Employment & Pay). Verification rejected it. The text supports both readings; its old proper name is fixture content, not a category or special production rule.
- The Swedish insurance image was described incorrectly as a warranty letter, but correctly matched to Insurance. Verification rejected it.
- The employment-certificate PDF was correctly described and matched to Employment & Pay, but verification rejected it.
- Later-page evidence was correctly read and classified. Its reader output also cited the application marker `[Earlier excerpt]` as one of its quotes. The first carried quote was genuine maintenance evidence; marker filtering needs separate improvement.

This small synthetic benchmark does not establish accuracy on personal documents or all supported languages. The verifier is conservative and fallible. Saved opaque folder names or special bundle rules may yield more requests for input because the matcher sees names only.

## Evidence and reproduction

- `fixtures.json`: source text, rendering flags, explicit generic rules and expected labels.
- `evaluation.json`: decisions, per-stage outputs, timings and summary. The harness's automated name checks are coarse; folder results were inspected separately.
- `traces/`: synthetic helper inputs/outputs for each stage.
- `provenance.json`: production helper and prompt-source hashes.
- `run.py`, `analyze.py`, `tuck-apple`: captured runner and trace wrapper with local scratch paths. Create the scratch `results/` and `traces/` directories and link `tuck-reader` to the built reader before running.

Build the helper/test binary with `cargo test --manifest-path src-tauri/Cargo.toml --lib`. For a portable full run, set `TUCK_EVAL_FIXTURES` to this directory's fixture file and `TUCK_EVAL_REPORT` to a temporary output path, then run `cargo test --manifest-path src-tauri/Cargo.toml --lib evaluation::tests::live_apple_accuracy_eval -- --ignored --exact --nocapture`. This harness reports quality; it does not assert all predictions are correct.

Regression verification: 80 Rust tests and 31 UI tests passed; Swift contract tests, frontend production build and Clippy passed. No release bundle was installed or app relaunched.
