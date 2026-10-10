# Generic categories: local model comparison — 2026-10-01

## Requested category set

Seven flat categories replace the personal folder hierarchy:

1. Identity & Immigration
2. Banking & Taxes
3. Employment & Pay
4. Housing & Utilities
5. Insurance
6. Medical Records
7. Maintenance & Repairs

Needs input is the explicit abstention result, not a category that creates a folder. The matcher receives only category names, the frozen document description, reader evidence and uncertainty. Its one-field enum and exact prompt are unchanged from the preceding no-rules comparison.

## Labels and unchanged inputs

[Expected labels and category definitions](taxonomy.json) were saved before inference. All 16 descriptions, source quotes and uncertainties are unchanged from the previous comparison. File IDs remain runner metadata, not model-generated IDs. The old AGENCY category and special source-bundle rule are removed. The former attachment is scored as Employment & Pay based on the underlying employment contract, rather than its supporting use in an immigration application. Original source text still mentions that use; this creates a real ambiguity in this one fixture.

The common verifier gets the new generic category definitions, with empty shared rules and no source labels. These definitions are not sent to the matcher. Saved app preferences and folders were not changed. This is an evaluation taxonomy, not a migration of user files or settings.

## Complete 16-case results

| Model | Correct category /14 | Unclear declined /2 | Response errors | Correct accepted /14 | Rubric mismatches accepted | Median match time |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Apple on-device | 13 | 2 | 0 | 11 | 0 | 0.55 s |
| Gemma 4 E2B | 13 | 1 | 0 | 11 | 0 | 0.59 s |
| Gemma 4 E4B | 13 | 2 | 0 | 11 | 0 | 1.13 s |
| Qwen 3.5 4B | 13 | 1 | 0 | 11 | 0 | 1.01 s |
| Qwen 3.5 9B | 13 | 1 | 0 | 11 | 0 | 1.88 s |
| Qwen 3 4B | 13 | 2 | 0 | 11 | 0 | 0.62 s |

Correct category is the raw matcher selection against the frozen labels. Correct accepted is after the unchanged Apple verifier prompt checks the new category definitions. Rubric mismatches accepted counts choices that differ from the predeclared expected destination and passed verification. No files were moved.

The common verifier rejected two correctly selected categories: the scanned insurance document (whose frozen description says warranty letter) and the scanned employment certificate. It also rejected the contract/bundle disagreement. Consequently all providers finished with 11/14 correct accepted choices, no accepted rubric mismatch, and both unclear documents retained for input. The verifier's rejection of the insurance image is consistent with the inherited description's inaccurate wording; the exact internal cause is not exposed.

## Separate view of the ambiguous contract

This breakdown is explanatory and does not replace or inflate the full 14-document score above.

| Model | Correct on 13 documents without the bundle ambiguity |
| --- | ---: |
| Apple on-device | 13/13 |
| Gemma 4 E2B | 13/13 |
| Gemma 4 E4B | 13/13 |
| Qwen 3.5 4B | 13/13 |
| Qwen 3.5 9B | 13/13 |
| Qwen 3 4B | 13/13 |

The source description for the remaining document is:

> AGENCY immigration application bundle containing an employment contract for permanent employment as supporting evidence for a residence permit application.

The frozen expected label is Employment & Pay. Identity & Immigration is a plausible purpose-based interpretation of that description, but it is not counted as correct under this test's document-type criterion. Changing this label after seeing outputs would obscure the ambiguity, so the original score is retained.

| Model | Contract/bundle choice | Common verifier |
| --- | --- | --- |
| Apple on-device | Identity & Immigration | Needs input |
| Gemma 4 E2B | Identity & Immigration | Needs input |
| Gemma 4 E4B | Identity & Immigration | Needs input |
| Qwen 3.5 4B | Identity & Immigration | Needs input |
| Qwen 3.5 9B | Identity & Immigration | Needs input |
| Qwen 3 4B | Identity & Immigration | Needs input |

## Method and limitations

- Six providers rerun live: Apple guided generation and the five already-installed Ollama tags in provenance. All calls local on the same 16 GB M4 Mac; models run one at a time and unload afterward.
- Same instructions, logical input fields and one-field category enum across providers. Apple injects its schema through the framework; Ollama receives `format` plus the schema appended to instructions. Temperature 0, seed 42, thinking off, 4096 context and 160 output tokens for Ollama; Apple retains its greedy settings.
- This is matching only. Descriptions were frozen from Apple; no new OCR, image/PDF analysis, description generation, filename generation or complete organizer execution is compared.
- The common Apple verifier is called once for each distinct document/category choice; identical proposals across providers share its result. Reader grounding remains required. Verification is measured separately and may make its own mistakes.
- Medians cover matching calls only. Ollama warmups/load times are separate; Apple has no explicit warmup and includes helper process startup. Prefix caches and provider runtimes differ. These are not full-app or cold-start times.
- Categories were intentionally designed for this known small document set. Some document content appears twice across original source formats. Results do not establish accuracy for unseen collections or a universal model ranking.
- Any improvement over the prior personalized taxonomy is a taxonomy comparison with corresponding new expected labels and verifier definitions, not evidence that model weights or extraction improved.

## Artifacts and reproduction

[Summary](summary.json), [all 96 results](results.json), [frozen cases](cases.json), [taxonomy and scoring rubric](taxonomy.json), [exact instructions](instructions.txt), [model/runtime provenance](provenance.json), [shared verifier outputs](verification-cache.json), [warmups](warmups.json), [runner](run.py).

Run the copied runner with this `cases.json` beside it in a fresh output directory. It requires the local models and Apple helpers described in [the previous comparison](../ollama/README.md). Never omit this frozen cases file: the runner's legacy fixture bootstrap targets the earlier category set. It performs inference only and never writes user documents or app settings.
