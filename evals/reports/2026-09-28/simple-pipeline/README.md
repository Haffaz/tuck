# Single-decision pipeline experiment

## Decision

**Do not adopt these candidates.** The experiment concluded by retaining filing-v5. See the current architecture document for subsequent focused updates. The simpler single-decision approach performed worse in each tested response format. This is evidence about these implementations with the current on-device model, not proof that every simpler architecture must fail.

## Method

A fixed set of 30 existing synthetic regression cases was assembled before the first candidate run: 26 have expected folders and four deliberately require manual handling. The set covers Swedish and English, a rendered PNG and PDF, named application bundles, identical rules, parent/child exclusions, learned examples, later-page retrieval and embedded commands aimed at the AI. All variants use the same case content, rules, OCR reader, language preparation, local model and tool/file-operation boundaries. These are familiar regression cases, not a held-out estimate of real-world accuracy. One full run per variant was measured; no retries were selected to improve scores.

No files were moved. “Correct” and “wrong” below describe accepted model proposals that would have been eligible for automatic organization. Expected manual cases are excluded from the filing denominator. Wrong destinations include mistakes on any of the 30 cases.

| Variant | Correct destination /26 | Wrong destinations /30 | Unnecessary manual /26 | Correct manual /4 | Refusal or guardrail /30 | Median seconds |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Current filing-v5 | 16 | 0 | 10 | 4 | 8 | 3.72 |
| v1: numeric destination, separate folder/type quotes | 10 | 9 | 9 | 2 | 4 | 3.66 |
| v2: exact destination names, separate quotes | 4 | 1 | 21 | 4 | 14 | 2.60 |
| v3: explanation first, one quote, smaller schema | 9 | 3 | 14 | 4 | 14 | 2.63 |

Faster aggregate medians partly reflect early refusals; they are not a successful-filing speedup.

| Variant | Generated names | Generated names passing automated checks | All substantive name checks /26 |
| --- | ---: | ---: | ---: |
| Current | 13 | 13 | 14 |
| v1 | 14 | 12 | 13 |
| v2 | 3 | 3 | 4 |
| v3 | 10 | 10 | 11 |

Name checks match fixture concepts and forbidden claims. Some cases expect the original name to be kept, so the final column can exceed the count of generated names. These checks do not establish semantic correctness for arbitrary files.

## Findings

- The numeric candidate sometimes explained the right folder but emitted a different valid index. A bank statement was sent to Insurance while the explanation named Finances; a certificate was sent to Home while its explanation named Work.
- Selecting exact folder names removed that extra mapping step but substantially increased fallback/refusal rates in this experiment.
- The smaller, explanation-first response recovered some cases but still confused a birth record with an immigration destination and mishandled source-group/learned preferences.
- All candidates allow the folder to succeed independently of naming and give image evidence directly to the filing decision. Those changes alone did not deliver better outcomes.
- Range checks, allowlists and quote-presence checks cannot prove that a valid folder is the correct semantic destination. The failed candidates demonstrate this limitation directly.
- Refusals propagate. There is no retry through another modality or safety-setting change after a refusal.

The current multi-stage design remains imperfect, with ten unnecessary fallbacks in this set. These results do not isolate a pure model-capability ceiling: prompt wording, structured response format, task decomposition and model restrictions all interact. A reasonable next experiment is to retain a focused folder-decision stage, supply visual evidence there, and handle naming separately. Adding more agents is not justified by these measurements.

## Selected real-document check

The final v3 candidate was also run read-only on the two previously reported images. It correctly selected the identification/licensing folder and driving-licence name for one. It incorrectly selected immigration/residency and the name Immigration Application for the partial birth record. Image input was confirmed for both and SHA-256 checks showed both originals were unchanged. Personal content and identifiers are not included here. This further supports rejecting the candidate.

## Verification

Sixteen deterministic validation checks passed for each candidate revision's validation behavior, including original-name preservation, invalid destination/evidence rejection, explicit source-name checking, identical rules, and bounded read requests. Production Swift sources and the installed helper match the saved baseline hashes. The candidate is isolated under `evals/simple-pipeline` and is not included in the app build.

See [experiment and reproduction instructions](../../../simple-pipeline/README.md), the four JSON reports, and `metadata.json` in this folder. Personal document contents are excluded.
