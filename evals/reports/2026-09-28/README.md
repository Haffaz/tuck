# Apple pipeline validation — 2026-09-28

These are synthetic documents on this Mac's macOS 27.0 on-device model. They are small regression sets, **not population accuracy estimates**. No personal document was classified or moved in these evaluations. Case outputs, errors and timings are preserved in the JSON reports.

## Known set: 12 cases

| Measure | Before | Final |
| --- | ---: | ---: |
| Correct folders for 10 identifiable documents | 5/10 | 6/10 |
| Correct proposed folders (excluding abstentions) | 5/6 | 6/6 |
| Automated title checks for identifiable documents | 1/10 | 6/10 |
| Identifiable documents left for manual review | 4/10 | 4/10 |
| Expected ambiguous/unreadable review cases handled | 2/2 | 2/2 |
| Forbidden title claims | 1 | 0 |
| Median analysis time, text inputs | 3.52 s | 2.63 s |

The six final usable proposals were residence permit card, bank statement, payslip, tenancy, home insurance, and employment certificate. The medical certificate, AGENCY attachment, electricity invoice and injected-insurance document were left in review. A refusal/abstention is not a correct classification. The injection fixture no longer produced the embedded `Annual Bonus` name; this single case is not proof of injection resistance.

Development included rejected variants: free-form naming often refused or returned untranslated titles; a stricter prose folder prompt produced 11 review outcomes and only 3 expected folders across 12 cases. That variant was not installed. The final prompt represents destination and parent instructions separately in JSON. No guardrails were disabled. Scores describe a single final run and remain subject to model variation.

## Multilingual set: 6 cases

The fixture set was written before its first run. Its first run exposed the same folder-prompt regression already found on the known set; the first-use report is retained as `multilingual-first-run-rejected.json`. After fixing that prompt using the known set, the multilingual set was rerun. The final numbers are therefore regression results, not a fresh held-out claim.

- German bank statement, French rental agreement and Spanish medical certificate were rendered as PNGs, read by Vision OCR, and assigned the expected folder and English template name.
- Japanese employment certificate text received the expected Work folder and English name.
- Polish bank text was detected as Polish; this Mac's model does not support it directly and Translation reported a downloadable English pair. It stayed in review with download guidance.
- Tamil bank text was detected as Tamil; neither the model nor the Translation pair supported it on this Mac. It stayed in manual review without an install suggestion.

That is **4/4 correct folders and names among the supported inputs**, with 2/6 left in review for language support. Overall document classification coverage is 4/6, not 6/6. The final median is 2.73 seconds, including extraction for rendered cases and the quick language-only fallbacks. First-use OCR took 28.6 seconds in the rejected run, so warm timings do not predict first-launch latency.

The actual download-needed and unsupported branches were exercised. The installed-translation execution branch compiled but was not exercised end to end because the necessary fallback language pair was not installed. No language packs were downloaded or macOS language settings changed during these tests. macOS 26 compatibility branches were not runtime-tested.

## Reproduce

See [pipeline documentation](../../../docs/ai-accuracy.md) for the commands. Test-process success only confirms evaluation completion; the JSON quality fields can report failures. Routine tests also cover quote/name validation, context overflow, protocol parsing, stale retry state, UI language guidance, and approval/path safeguards. Larger fresh datasets and real scanned-page variety remain future work.

## Additional live smoke failures

The existing opt-in `live_apple_agent_smoke` still fails its short English insurance-description assertion because Apple refuses folder selection. It stops before its later-page assertion. The existing `live_apple_review_flow` also fails its richer English bank-statement expectation because Apple refuses folder selection; the source remains in review and the approval/move step is not reached. The assertions were retained. A focused-quote-only routing variant was tried against these two failures and did not resolve them; it was discarded. These additional inputs show that the favorable six-case multilingual run does not establish reliable real-document accuracy.

The ordinary Rust approval and file-operation tests pass with controlled decisions, but this turn's live-model approval/move smoke did **not** pass. Do not describe all live checks as passing.

## Installed app verification

The final build was installed at `/Applications/Tuck.app`; its signature and app/helper binary hashes were checked against the bundle. Native UI verification confirmed the new privacy header and Polish download guidance, with the original filename kept and Organize disabled until a destination is chosen. The disposable text fixture was cleared from the batch without organizing it. No personal documents were used. The prior app is recoverable at `/private/tmp/tuck-before-languages-gbsk1754/Tuck.app`.
