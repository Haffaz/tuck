# Collection-aware folder structure — 5 October 2026

## Problem and change

A valid document type was sufficient to create a subfolder. A small collection could therefore produce Education/Degree, Education/Certification and Education/Transcript with one document in each. The previous accuracy evaluation did not measure this fragmentation.

The new pass runs after all model batches, before generating folder rules. An automatic child normally needs three documents and a meaningful alternative: another substantial child or at least three documents remaining at the parent. Redundant single-child levels collapse. Each deeper level is assessed separately. Related documents stay under their original subject; unrelated root subjects and review cases are not merged.

Applicable user instructions preserve only the requested depth. For example, grouping invoices by supplier can retain a small supplier folder without retaining an unnecessary third-level type folder. Rules for another subject, naming instructions and prohibitions do not preserve automatic subfolders. The pass affects new suggestions; existing saved folder structures require user review to replace.

## Evidence

- 16 frozen structure cases pass in both input orders, including sparse education records, useful dense groups, three-level trees, explicit exceptions and parent fallbacks. Applying the pass again is stable.
- A production integration check combines 24 documents across multiple model batches before simplifying. A second check verifies that merging education folders retains degree, certification and transcript scopes in the generated rule.
- The live Apple core pilot passed all acceptance gates: 48/48 clear filing decisions, 12/12 review decisions, 48/48 clear proposal subjects, 8/8 requested grouping checks, and 42/42 filings into generated folders. No incorrect automatic filing or depth violations occurred. One separate shared-name edge remained a safe review; the other scope edges passed.
- 95 Rust tests passed; 12 opt-in tests were ignored. Swift checks, 61 custom-provider contract checks, Clippy, formatting and the macOS build passed. The installed app signature and complete file manifest were verified.

Both the selected-folder replay and the fresh installed-app preview reduce ten categories to six: Birth certificate, Driving Licence, Education, Immigration, Travel and Wellness. The three education documents share Education; the live rule explicitly includes the degree, placement certification and transcript. The preview remains unapproved. Coverage remains 15/25; ten documents still need review. Private document text, filenames and raw responses are excluded from this report.

These are structure expectations and focused regression results, not a universal accuracy percentage. Three documents is a conservative default; explicit grouping rules allow smaller intentional groups. The entire earlier matrix and held-out model suite were not repeated for this deterministic layout change.

## Reproduce

Run `python3 evals/structure/run.py --out /private/tmp/tuck-structure-new-run` for structure and integration checks. The optional `--assignments` argument replays a previously captured assignment array without model calls or file moves; use a private output directory for personal data. The live regression used `evals/rules-nesting/run.py --suite core --pilot` with the rebuilt helper.
