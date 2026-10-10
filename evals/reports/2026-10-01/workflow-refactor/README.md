# Workflow refactor evaluation — 2026-10-01

## Scope

Same frozen 16 synthetic fixtures and seven generic category labels as the earlier category-enum production evaluation. Fourteen clear documents, two intentionally unclear. Includes two images, a PDF, Swedish and English, an embedded instruction attack and a later text chunk. Expected labels were fixed before inference. All calls used local Apple Intelligence; GPT/Claude/Gemini were documentation research only. This harness makes filing proposals without moving documents.

## Final results

| Measure | Previous recorded v4 | Current v5 |
| --- | ---: | ---: |
| Correct automatic proposals / clear cases | 11/14 | 12/14 |
| Wrong automatic proposals | 0 | 0 |
| Clear cases needing input | 3 | 2 |
| Intentionally unclear retained | 2/2 | 2/2 |
| Model/refusal/format errors | 0 | 0 |
| Median full pipeline | 2.83 s | 2.98 s |
| Maximum | 4.22 s | 4.54 s |

This is a small synthetic regression comparison, not a broad accuracy estimate or a statistically significant speed comparison. The previous run was recorded earlier, not executed simultaneously. Description, verification and language preflight changed together, so this does not isolate their individual contribution.

Seventeen descriptions (one recovery read), sixteen provider match operations (two abstain locally before an actual model match), thirteen verification calls and twelve naming calls. No language preflight calls. Category planning is tested separately below.

## What improved and what still fails

- The Swedish insurance scan is now described as home insurance and receives the correct automatic folder. The employment-certificate PDF also passes.
- The later-page fixture obtains an empty quote initially. Rust reads the referenced chunk 5, retries description, then files it correctly. No model-generated read command or metadata quote is needed.
- The bank statement is correctly described but the reader selects no source quote, so it stays for input. We did not weaken grounding to force an automatic proposal.
- The historical employment contract in an immigration application bundle still produces an Identity & Immigration candidate. The stricter verifier rejects it. The predeclared expected label is Employment & Pay; both purposes are present in the text, so the case is explicitly ambiguous.
- The residence-card image is described broadly as a right-to-live document. Its name remains less precise than desired. Folder accuracy does not establish name accuracy.
- Both intentionally unclear inputs stay for input. The embedded instruction does not redirect the insurance document.

## Rejected experiments

`experiments.json` records four development variants on these same fixtures. A short free-JSON reader omitted mandatory fields. Explicitly requiring all keys improved some results but left decoding errors and matcher refusals. Native guided free-text uncertainty produced literal “No”/“None”, which strict application checks correctly did not treat as empty. A typed boolean removed that ambiguity. The briefer verifier accepted the ambiguous contract/application candidate; the final version restores explicit requirements that a better type match and unsupported intended-use association must fail.

Native guided output is used for the three-field reader and one-field matcher. Planning retains bounded validated JSON because the prior guided-planner experiment refused an actual collection. No refusal text is converted into a usable answer.

## Evidence and reproduction

`fixtures.json` contains synthetic inputs and expected labels. `evaluation.json` includes per-stage outputs and timings. `traces/` contains synthetic subprocess envelopes/results; these envelopes include application metadata that adapters omit from model prompts. `provenance.json` records OS and source/helper hashes. No personal document text is stored in this report.

Build with `cargo test --manifest-path src-tauri/Cargo.toml --lib`. Set `TUCK_EVAL_FIXTURES` to this fixture file and `TUCK_EVAL_REPORT` to a temporary path; run the ignored `evaluation::tests::live_apple_accuracy_eval` test with `--exact --nocapture`. The harness reports quality without asserting every prediction is correct. The captured trace runner uses its recorded `/private/tmp/tuck-workflow-final` scratch directory and requires results/traces directories plus a link to the built native reader.

See [architecture and official design references](../../../../docs/ai-architecture.md) for actual model contracts and bounds.

## Real-folder category check and delivery

A separate read-only preview used the previously authorized 25-document Tuck Test - 2 collection. Temporary cache lived outside the selected folder. The final workflow described 22, skipped 3 and returned 3 categories in 79.40 seconds. Every source hash was unchanged. This checks completion/coverage, not the correctness of every personal-document description or category. No personal source text, descriptions or category payloads are saved in this report. The earlier v4 preview described 9 and skipped 16; folder locations changed through the user's app session between runs, so this is not a controlled extraction-only comparison.

Validation: 79 Rust tests passed (10 live tests ignored in the standard suite); 31 Playwright tests passed; standalone Swift role contracts and model-support tests passed; Clippy with warnings denied passed; frontend and Tauri release builds passed; git diff whitespace check passed. The live synthetic automatic-flow smoke organized one bank statement with its hash unchanged, retained an unreadable document, and confirmed the watcher ignores its own output.

The built bundle was signed, verified and installed at `/Applications/Tuck.app`, with a recoverable backup at `/private/tmp/tuck-before-category-update-_x1ubich/Tuck.app`. Build/installation file hashes matched. Installed helper reports `serial-roles-v5-simple`. The app was relaunched and its native accessibility tree shows the saved four-category structure, Suggest categories, watching preference and Start Tuck. The diagnostic three-category draft was not applied. Restart stopped the user's prior session; no automatic filing was restarted.
