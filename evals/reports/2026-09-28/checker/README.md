# Checked-v2 comparison — 2026-09-28

Synthetic documents only, macOS 27.0, Apple's on-device Foundation Models. These are small known regression sets, not a general accuracy estimate. Reports retain outputs and timings, including failures. Running an evaluation test successfully means it completed; it does not mean every expected answer was produced.

## Before and after

| Suite                | Before                                    | Shipped flow                                                            | Median before → after              |
| -------------------- | ----------------------------------------- | ----------------------------------------------------------------------- | ---------------------------------- |
| Existing 12-case set | 8/12 expected outcomes                    | 9/12 expected outcomes                                                  | 2.57 s → 2.64 s                    |
| Five focused cases   | 1/5 expected outcomes                     | 2/5 expected outcomes                                                   | 2.41 s → 2.91 s                    |
| Six-language set     | Earlier run: 4/4 supported inputs correct | 4/4 supported inputs correct; 2 unsupported-language cases require help | 2.76 s final median across all six |

The 12-case set contains 10 identifiable documents and two expected abstentions. Before: 6/10 identifiable documents were filed correctly and 4 required manual choice. After: 7/10 were filed correctly and 3 required manual choice. Both ambiguous cases abstained in both runs. No wrong destination was emitted on this set; no forbidden title claims were observed. The extra correct case was the injected insurance sample. The organizer schema and evidence handling also changed, so this cannot be attributed to the checker alone.

In the five focused cases, identical sibling descriptions now correctly require clarification instead of arbitrarily choosing a folder. This is a code check. The four identifiable cases still have one correct destination and three abstentions. The rental/child-rule case gets an AI check and finishes correctly, taking 4.92 seconds versus 2.41 seconds with the earlier helper. The source-bundle and later-chunk cases still encounter model refusals; the maintenance log remains unidentified. The later-chunk case now successfully reads chunk 3 before the refusal. Additional reading does not guarantee a classification.

The checker has not demonstrated a standalone accuracy gain on these few cases. It provides a bounded second comparison and can still repeat or introduce mistakes. The shipped conservative behavior leaves unresolved cases for the user; human approval remains mandatory for every file operation.

## Rejected attempts

The first and second checker versions rejected the valid personal rental agreement. A further flat-text rule prompt also confused the child's commercial-only restriction with the parent's residential rule. Those outputs are retained. The final checker receives the candidate and each alternative with separately scoped, fully inherited rules in structured JSON. It explicitly keeps child exclusions local to that child. The rental regression then passed in the focused repair and full final run.

The focused set was authored before these tests and subsequently used to repair the prompt, so its final result is a regression result, not held-out validation. The baseline focused comparison uses the preserved old Apple helper with the new Rust evaluation/quote collection code; it isolates helper behavior imperfectly. The 12-case baseline was run before implementation with the original complete pipeline. Each reported run is a single pass, so timing differences include startup and ordinary variability.

## Language and UI checks

German bank, French rental and Spanish medical examples used rendered images and Vision OCR; Japanese used text. All four retained correct folders and English template names. Polish offers a translation-language download; Tamil remains unsupported on this Mac. No resources were downloaded, and installed-translation execution remains unverified.

A live generic Migration-folder description request returned a short editable description. Backend and UI tests verify that generation creates no directories, saves no rules, preserves edits, and releases processing state after failure. Deterministic tests cover checker outcomes, quote grounding, duplicate-rule scope, context limits and bounded read hints. The existing approval/path/hash checks remain in effect.

## Reproduce

```sh
TUCK_EVAL_REPORT=/private/tmp/tuck-regression.json cargo test --manifest-path src-tauri/Cargo.toml live_apple_accuracy_eval -- --ignored --nocapture
TUCK_EVAL_FIXTURES="$PWD/evals/checker.json" TUCK_EVAL_REPORT=/private/tmp/tuck-checker.json cargo test --manifest-path src-tauri/Cargo.toml live_apple_accuracy_eval -- --ignored --nocapture
TUCK_EVAL_FIXTURES="$PWD/evals/multilingual.json" TUCK_EVAL_REPORT=/private/tmp/tuck-languages.json cargo test --manifest-path src-tauri/Cargo.toml live_apple_accuracy_eval -- --ignored --nocapture
```

`TUCK_EVAL_READER` optionally selects a saved reader beside its matching `tuck-apple` helper. `TUCK_EVAL_LABEL` identifies such comparison runs. Do not put personal document contents in these fixtures or reports. Earlier opt-in smoke failures recorded in the parent report were not all rerun here and are not claimed fixed.
