# How Tuck got here

Tuck went through two quite different designs. This page keeps what was learned; the detailed
reports from the first design were removed because they referred to private test folders.

## 1. A pipeline of model roles (late September – early October 2026)

The first design asked Apple's on-device model to do everything, step by step, for each document:

1. **Describe** the document and quote the exact lines that support the description.
2. **Group** documents into suggested folders, four at a time, carrying subject names between batches.
3. **Match** a new document against the folder rules in a tournament of up to eight candidates.
4. **Verify** the match with a separate call, and reject it if the check disagreed.
5. **Name** the file, accepting only words found in the verified facts.

Rust checked every step (quotes really exist in the text, folders are enabled, paths stay inside the
root), and anything uncertain went to review. Synthetic test suites reached 95–100% with zero wrong
automatic filings.

On a real folder of 25 scanned documents, though, only 15 got a folder suggestion. The causes were
structural rather than a weak model:

- Documents over 5,000 characters or 12 pages were rejected outright, so most long scans never
  reached the model.
- Each document needed up to about ten chained calls; any refusal, guardrail hit or malformed reply
  sent it to review.
- The prompts had been tuned against the synthetic suites they were measured on.

## 2. Cards and clustering (from 10 October 2026)

A small prototype took a different approach: describe each file once, then group the descriptions
mathematically. Tuck was rebuilt around it:

- **One card per document**: kind, issuer, date and three tags, from one guided call, plus an
  on-device sentence embedding. Long documents are read from the start instead of rejected.
- **Grouping is deterministic**: average-linkage clustering by cosine similarity groups documents of
  the same kind; the model only names the groups and places them under broad subjects.
- **Filing is one choice** among the closest folders, with the folder rules shown to the model.

Same harness, same documents, old pipeline versus new (synthetic corpora, see
[evals/cards](../evals/cards/README.md)):

| Measure | Role pipeline | Card pipeline |
| --- | --- | --- |
| Mixed corpus (35 docs incl. long, scanned, Swedish, German): files given a folder | 29 | 35 |
| Mixed corpus: filed correctly / wrongly / sent to review | 29 / 0 / 6 | 35 / 0 / 0 |
| Held-out corpus (29 docs): clear documents filed correctly | 25 / 25 | 25 / 25 |
| Held-out: deliberately unclear documents held for review | 4 / 4 | 1 / 4 |
| Median time to file a document | ~3.2 s | ~0.6 s |

The card pipeline trades some caution on deliberately ambiguous files for far fewer unnecessary
reviews.

## Lessons worth keeping

- **Guardrails differ by output mode.** With Apple's permissive guardrail mode, guided (schema)
  generation still refused identity, tax and immigration papers; plain-text generation did not.
  Tuck asks for structured output first and falls back to a fixed plain-text format.
- **Embeddings group kinds, not subjects.** Apple's sentence embeddings scored the same kind from
  different issuers around 0.8 and different kinds 0.55–0.73, but could not tell that a payslip and
  an employment contract both belong under work. A single model call places kinds under subjects.
- **Output is expensive, input is cheap.** Dropping one summary sentence from each card halved its
  generation time; giving the model the document's opening during folder choice cost almost
  nothing.
- **The on-device model serves one request at a time.** Three concurrent requests saved about a
  quarter of the wall time; more saved nothing.
- **Text recognition confidence beats language detection** for spotting unreadable scans.
  Apple's OCR does not support Sinhala or Tamil and returned garbage with confidence 0.30–0.42
  (readable pages: 0.75–1.0), while language detection reported the garbled Tamil as Hindi with
  83% confidence.
- **Small prompt changes move answers.** Adding one sentence about file clues turned a hotel
  booking into a "rental agreement", so files without clues keep the original prompt exactly.
- **Measure on documents the prompts were not written for.** Every change here was checked on a
  second corpus it was not tuned on.
