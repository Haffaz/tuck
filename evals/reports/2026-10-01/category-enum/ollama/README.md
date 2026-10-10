# Simple category matcher: local model comparison — 2026-10-01

## Scope

Tested the five installed Ollama models and a fresh Apple control on identical frozen inputs from the 16-case synthetic test. This compares **category matching only**. Every model receives the same Apple-generated descriptions, evidence and uncertainty, category names and Needs input fallback. No folder rules, parent rules, source labels or naming preferences are supplied to any matcher. Images/PDFs were already read in the earlier run; this test does not send images or PDFs to Ollama and does not compare extraction, description quality or naming. No personal documents were used or moved. Production provider selection is unchanged.

The complete task instructions are [saved here](instructions.txt). The answer schema has only `category`, constrained to the actual folder paths plus Needs input. Ollama receives the JSON schema both through its `format` parameter and appended to the instructions; Apple receives its dynamic schema with `includeSchemaInPrompt: true`. This follows the [Ollama structured-output documentation](https://docs.ollama.com/capabilities/structured-outputs). Ollama's HTTP response still requires JSON decoding and validation in the client; its generation is constrained by the schema.

## Results

| Model | Correct clear matches | Unclear inputs correctly declined | Errors | Correct after Apple check | Wrong accepted | Median match time |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Apple on-device | 11/14 | 2/2 | 0 | 10/14 | 0 | 0.54 s |
| Gemma 4 E2B Q4_K_M | 9/14 | 1/2 | 0 | 8/14 | 0 | 0.54 s |
| Gemma 4 E4B Q4_K_M | 10/14 | 2/2 | 0 | 9/14 | 0 | 1.05 s |
| Qwen 3.5 4B | 10/14 | 1/2 | 0 | 9/14 | 0 | 0.96 s |
| Qwen 3.5 9B | 10/14 | 1/2 | 0 | 9/14 | 0 | 1.75 s |
| Qwen 3 4B | 10/14 | 2/2 | 0 | 8/14 | 0 | 0.59 s |

The denominator 14 excludes the two intentionally unclear inputs. Matcher correctness is measured before verification. Correct after Apple check means the selected category was accepted by the shared existing verifier, not that a file was moved. Its prompt still includes configured folder rules. All original expected categories were preserved; for example the electricity bill belongs in Home under the original preferences, even though Finances is plausible when only names are shown. This test therefore also measures the loss of user intent when rules are omitted.

No API errors or malformed/out-of-enum responses occurred in this run. Ambiguous-input guesses are scored as wrong even when the verifier later rejects them. These are small known synthetic fixtures, including repeated content across original formats, not a held-out accuracy estimate or a general model ranking.

## Timing and resource method

All inference ran locally on the same 16 GB M4 Mac through Ollama 0.34.4. Models ran sequentially and were unloaded between models and after completion. No downloads were needed. Ollama used temperature 0, seed 42, thinking disabled, 4096-token context and a 160-token output cap. Apple retained the existing greedy matcher settings. Provider-specific tokenizers, schema injection, model templates and runtimes differ; identical logical prompts cannot make these engines identical.

Medians cover the matching request only. They exclude OCR, reading, description generation, verification, naming and file operations. Ollama models had a neutral warmup before scored cases. Apple was rerun live but had no explicit warmup; its first call includes initialization. Apple helper timings include process startup; Ollama timings include localhost HTTP round-trip. Both may benefit from prompt/prefix caches. Small timing differences should not be treated as a whole-app speed result. No peak RAM, energy or battery measurement was made.

| Model | Neutral warmup including loading |
| --- | ---: |
| Gemma 4 E2B Q4_K_M | 5.81 s |
| Gemma 4 E4B Q4_K_M | 6.71 s |
| Qwen 3.5 4B | 3.54 s |
| Qwen 3.5 9B | 5.61 s |
| Qwen 3 4B | 3.08 s |

## Common verification

After all model matches completed and Ollama unloaded, each distinct document/category proposal was checked once with the unchanged production Apple verifier. Identical proposals across models reuse that verdict so verifier variability cannot favor one matcher. Reader grounding is required before verification. Unknown/uncertain cases stay for input. The verdicts and timing are saved in [verification-cache.json](verification-cache.json); this timing is excluded from the matching medians. This is an isolated stage comparison, not a replacement test for the full Rust file-operation coordinator.

## Artifacts and reproduction

- [Summary](summary.json)
- [All 96 matching responses and verdicts](results.json)
- [Frozen input cases, original expected labels and verifier inputs](cases.json)
- [Runtime/model hashes and settings](provenance.json)
- [Neutral warmup calls](warmups.json)
- [Runner](run.py)

Run the script in a fresh output directory with this `cases.json` beside it. It needs the production Apple helper, the temporary guided helper built from [the no-rules patch](../no-rules.patch), and the five model tags listed in the runner already installed. Adjust its explicit local paths if needed. The runner checks that no unrelated Ollama model is loaded before starting. The script uses only localhost model calls and synthetic data; it does not install models or organize files. Each row's input hash was checked identical across models. Expected labels never enter matcher prompts.
