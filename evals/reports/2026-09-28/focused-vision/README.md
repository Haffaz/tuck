# Focused image-aware folder selection — filing-v6

The focused pipeline is retained. Image evidence now reaches semantic folder selection as well as document reading; source-bundle matching, naming templates and conditional checking remain separate. The production routing prompt is unchanged.

## Comparison

The frozen 30-case suite is the same set used in [the rejected single-decision experiment](../simple-pipeline/README.md). The installed v5 helper and its source were verified byte-for-byte against that experiment's baseline before this run. These are synthetic regression cases, not a held-out accuracy estimate.

| Measure | Filing-v5 baseline | Filing-v6 |
| --- | ---: | ---: |
| Correct substantive folder proposals | 16/26 | 16/26 |
| Wrong destinations across all cases | 0/30 | 0/30 |
| Unnecessary manual fallbacks | 10/26 | 10/26 |
| Correct expected-manual cases | 4/4 | 4/4 |
| Refusals/guardrails | 8/30 | 8/30 |
| Generated names passing automated checks | 13/13 | 13/13 |
| Median case latency | 3.72 s | 3.18 s |

Every folder and filename result matched the baseline. This shows no observed regression on this set, **not a measured general accuracy improvement**. Timings are single-run observations, not a controlled speed comparison. The rendered birth-record PDF used image evidence in both reading and folder selection. The synthetic driving-licence PNG was refused during reading, before routing. Text cases do not exercise the new image input.

## Two selected real documents

A read-only diagnostic of the two previously reported images selected Identification & Licensing for both, with Birth Certificate and Driving Licence as their respective names in this run. Both reading and folder selection confirmed image use. Source SHA-256 checks confirmed neither original changed. The previous v5 run kept the partial birth record's original name; because its reading stage is unchanged, that naming difference cannot be attributed to routing images and illustrates model variability. No personal text, identifiers, or images are included here.

## Checks

- TypeScript/Vite build passed.
- Rust: 61 tests passed; eight live tests remain opt-in.
- Playwright: 18 tests passed (mock Tauri bridge).
- Swift model support: 53 checks passed.
- Rust Clippy with warnings denied passed.
- Diff whitespace check passed.

All live classification tests were read-only. These checks do not establish that the remaining manual cases can be automatically organized or that arbitrary future documents will be classified correctly.
