# Gemma local document comparison

## Scope

64 new calls: two Gemma 4 models, four input formats, eight batches representing ten documents. Same six synthetic and four user-approved real documents as the [previous benchmark](../four-page-images/README.md). Every request hash matches the previous Qwen 4B input for that case and mode. Inference stayed on 127.0.0.1; no documents were moved or renamed and no production app code changed.

## Correct folder decisions

| Model | Text | Separate pages | Four pages in one image | Packed image + text |
|---|---:|---:|---:|---:|
| Apple on-device | 7/10 | 4/10 | 2/10 | 8/10 |
| Qwen3.5 4B | 9/10 | 10/10 | 7/10 | 9/10 |
| Qwen3.5 9B | 9/10 | 9/10 | 10/10 | 9/10 |
| Gemma 4 E2B Q4_K_M | 9/10 | 8/10 | 6/10 | 7/10 |
| Gemma 4 E4B Q4_K_M | 10/10 | 9/10 | 8/10 | 10/10 |

## Gemma details

| Model | Input | Folder | Type | Single-document cases | Mixed-document cases | Errors | Wrong destinations | Tiny markers | Median successful request |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Gemma 4 E2B Q4_K_M | Text | 9/10 | 9/10 | 7/7 | 2/3 | 0 | 1 | 6/6 | 2.63 s |
| Gemma 4 E2B Q4_K_M | Four separate images | 8/10 | 4/10 | 7/7 | 1/3 | 0 | 2 | 6/6 | 20.57 s |
| Gemma 4 E2B Q4_K_M | One four-page image | 6/10 | 3/10 | 5/7 | 1/3 | 0 | 2 | 6/6 | 7.53 s |
| Gemma 4 E2B Q4_K_M | Four-page image + text | 7/10 | 8/10 | 5/7 | 2/3 | 0 | 3 | 6/6 | 5.4 s |
| Gemma 4 E4B Q4_K_M | Text | 10/10 | 8/10 | 7/7 | 3/3 | 0 | 0 | 6/6 | 5.74 s |
| Gemma 4 E4B Q4_K_M | Four separate images | 9/10 | 7/10 | 7/7 | 2/3 | 0 | 0 | 6/6 | 31.05 s |
| Gemma 4 E4B Q4_K_M | One four-page image | 8/10 | 7/10 | 6/7 | 2/3 | 0 | 2 | 2/6 | 11.62 s |
| Gemma 4 E4B Q4_K_M | Four-page image + text | 10/10 | 8/10 | 7/7 | 3/3 | 0 | 0 | 6/6 | 9.23 s |

The correct Needs Review answer for the intentionally ambiguous document counts toward folder accuracy. Errors count as failures; results are never retried to improve a score. Type accuracy is stricter than choosing the right broad folder.

## Local resource costs

| Model | Model artifact size | Largest sampled server + inference RSS | First neutral request including load |
|---|---:|---:|---:|
| Gemma 4 E2B Q4_K_M | 4.59 GB | 3.44 GB | 9.63 s |
| Gemma 4 E4B Q4_K_M | 6.58 GB | 7.33 GB | 10.09 s |

Memory is the largest observed sum of resident-set sizes for the benchmark server and its direct inference subprocess, sampled every two seconds. Sampling began during E2B case s3, so its startup and first cases are excluded. This is not total system memory or a precise peak: shared mappings and transient allocations affect the result. Ollama /api/ps reports an implausibly small Gemma allocation, so it is retained in raw records but not used for the resource comparison. Artifact size is the registry model size, not unique disk space: vision components may be shared. First-request timing is a neutral text call, not a document call. The Mac already used about 6.5 GB of swap at the E2B warmup. Swap snapshots are recorded per request but other apps can affect them; low resident memory must not be read as a minimum RAM requirement or proof of no memory pressure. This test does not measure battery consumption or temperature.

## Method and limitations

- Ollama 0.34.4 on the same 16 GB M4 Mac. Explicit Q4_K_M GGUF tags; this is not an MLX measurement. Each model runs alone and unloads afterward. Ollama automatically enabled a speculative draft-MTP backend for both Gemma models (observed in process arguments); runtime implementation differs from the earlier Qwen backend.
- The [original methodology](../four-page-images/METHOD.md) describes the frozen dataset, four-page rendering and scoring. Personal inputs remain in the private scratch directory and are not published here.
- Same schema, folder rules, temperature 0, seed 42, thinking off, 16384 context and 700 output tokens as the Qwen run. Google recommends different default sampling; this is a controlled Tuck-task comparison, not a best-possible Gemma benchmark.
- Models run sequentially and condition order rotates. Prompt and image caches affect timing; medians exclude warmup, download, PDF rendering, OCR and failed calls. No full end-to-end or cold-cache latency claim is made.
- One pass per condition on ten documents is exploratory. This does not test Tuck's production coordinator, checks, translation, watching, file operations or calibrated confidence.
- Prior Apple and Qwen scores are copied from the completed baseline, not rerun. All scores use identical labels.
- Raw response text and personal metadata are excluded. The runner imports the original scratch benchmark and needs the private manifest and compiled helper to replay.
