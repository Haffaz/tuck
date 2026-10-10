# Ollama comparison — 28 September 2026

## Recommendation

Qwen3.5 9B is the stronger candidate for a future Tuck integration when avoiding wrong automatic filings matters more than speed. On this suite it filed seven more documents correctly than Apple, with no wrong destinations observed. Qwen3.5 4B was faster, but made two confident folder mistakes. This is evidence for further integration testing, not proof of general accuracy. The installed app still uses Apple Intelligence.

## Same documents, same filing pipeline

One frozen 30-case synthetic regression suite, run sequentially on an Apple M4 MacBook Pro with 16 GB memory and macOS 27.0. The first 26 figure below counts only cases with an expected destination; four other cases deliberately require manual attention. The existing production filing-v6 prompts, folder rules, evidence, OCR, language preparation, bounded tools and validators were reused. Only the generation transport was replaced. No documents were moved.

| Model | Correctly filed / 26 | Wrong automatic destinations | Unnecessary manual fallbacks / 26 | Expected manual cases handled / 4 | Median per document |
| --- | ---: | ---: | ---: | ---: | ---: |
| Apple Intelligence (fresh baseline) | 16 | 0 | 10 | 4 | 2.45 s |
| Qwen3.5 4B | 23 | 2 | 1 | 4 | 7.89 s |
| Qwen3.5 9B | 23 | 0 | 3 | 4 | 14.32 s |

Apple produced eight explicit refusal/guardrail fallbacks. Ollama had no recorded transport/generation errors, no output-limit stops, and no explicit refusal/guardrail exceptions in these runs. All four expected manual cases remained manual with every model.

## What differed

- **4B wrong destination:** the rendered birth registration record was identified and named Birth Certificate, then filed in Education & Certifications instead of Identification & Licensing. 9B chose the correct folder.
- **4B wrong destination:** an employment certificate in the Aster application bundle was filed in Work instead of Migration. 9B left this case manual.
- **Source bundles remain a weakness:** 9B left the AGENCY, Cedar and Aster bundle cases manual after the independent source-association check. 4B left AGENCY manual, filed Cedar correctly, and misfiled Aster. These rules were not edited after observing results.
- **Names still need work:** both Qwen sizes named the Swedish home insurance document Insurance Certificate, missing the more useful home-insurance specificity required by the fixture.

| Model | Generated names passing fixture checks | All substantive naming checks passing / 26 |
| --- | ---: | ---: |
| Apple Intelligence (fresh baseline) | 13 / 13 | 14 |
| Qwen3.5 4B | 23 / 24 | 24 |
| Qwen3.5 9B | 21 / 22 | 22 |

Name checks are automated concept checks, not a complete semantic audit. The denominator for generated names excludes cases where the original filename was kept, including manual fallbacks.

## Speed and local execution

- Ollama version 0.34.4, both models Q4_K_M. Downloads: 4B 3.39 GB; 9B 6.59 GB. Full installed model digests are in manifest.json.
- Each Ollama run made 67 generation calls, including four image calls across the two rendered fixtures. Largest reported prompt: 2,930 tokens, inside the 8,192 context setting.
- Neutral Ollama warmups were excluded from per-document timing: 4B 4.26 s; 9B 4.50 s. Model output generation averaged 29.3 tokens/s for 4B and 18.0 tokens/s for 9B across calls.
- Rendered driving licence: 4B 33.79 s; 9B 53.86 s. Rendered birth record PDF: 4B 27.38 s; 9B 44.35 s. These timings include the full document classification path, not just one model call.
- Models ran sequentially and were unloaded between providers. Both Ollama models remain installed; the benchmark runner unloaded them after testing. Inference used loopback only.

## Limits and reproducibility

This is one run per model on a known regression suite: mostly English/Swedish synthetic text and only two rendered documents. It does not establish accuracy across arbitrary real scans, languages or folder rules. Adaptive tool calls can differ between models. The fresh Apple run reproduced the previous baseline’s folder/name outcomes, with a lower observed median (2.45 s versus 3.18 s previously); timings should not be treated as stable performance guarantees.

Apple guided schemas were encoded directly as JSON Schema. Ollama receives that schema both as a constrained output format and as readable schema text; Apple supplies it through its guided-generation API. Consequently the evidence and source instructions are held constant, but provider token streams and decoding mechanisms are not identical. Apple OCR and language preparation remain in this evaluation, so it does not measure standalone Ollama multilingual preprocessing.

All case IDs and extracted text character counts matched across runs; the frozen fixture and inference source hashes are recorded in manifest.json. No fixture-specific prompt tuning or quality retries were performed. See [the runnable evaluation instructions](../../../ollama/README.md).

Artifacts: [Apple results](apple.json), [4B results](qwen3.5-4b.json), [9B results](qwen3.5-9b.json), [manifest and model digests](manifest.json), [4B call metrics](qwen3.5-4b-calls.jsonl), [9B call metrics](qwen3.5-9b-calls.jsonl).

Official model information: [Qwen3.5 4B](https://ollama.com/library/qwen3.5:4b), [Qwen3.5 9B](https://ollama.com/library/qwen3.5:9b).
