# Four-page PDF image comparison

Run on 30 September 2026. All inference stayed on this Mac. No source documents or production code were changed.

## Main result

| Model | Text | Four separate images | One four-page image | Four-page image + text |
|---|---:|---:|---:|---:|
| Apple on-device | 7/10 | 4/10 | 2/10 | 8/10 |
| Qwen3.5 4B | 9/10 | 10/10 | 7/10 | 9/10 |
| Qwen3.5 9B | 9/10 | 9/10 | 10/10 | 9/10 |

### Single-document sheets versus the mixed-document sheet

| Model | Input | Seven single-document decisions | Three decisions in mixed batch |
|---|---|---:|---:|
| Apple on-device | Text | 5/7 | 2/3 |
| Apple on-device | Four separate page images | 3/7 | 1/3 |
| Apple on-device | One four-page image | 2/7 | 0/3 |
| Apple on-device | Four-page image + text | 6/7 | 2/3 |
| Qwen3.5 4B | Text | 7/7 | 2/3 |
| Qwen3.5 4B | Four separate page images | 7/7 | 3/3 |
| Qwen3.5 4B | One four-page image | 7/7 | 0/3 |
| Qwen3.5 4B | Four-page image + text | 7/7 | 2/3 |
| Qwen3.5 9B | Text | 6/7 | 3/3 |
| Qwen3.5 9B | Four separate page images | 6/7 | 3/3 |
| Qwen3.5 9B | One four-page image | 7/7 | 3/3 |
| Qwen3.5 9B | Four-page image + text | 6/7 | 3/3 |

Entries are correct folder decisions across ten documents (six synthetic and four real), including one intentional Needs Review result. Each condition ran eight four-page requests. This is an exploratory input-format experiment, not production-pipeline accuracy.

## Detailed results

| Model | Input | Folder correct | Type correct | Wrong folder suggestions | Fallbacks incl. errors | Call errors | Tiny markers read | Median successful request (s) | Mean input tokens |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Apple on-device | Text | 7/10 | 6/10 | 1 | 3 | 2 | 4/6 | 1.47 | Not exposed |
| Apple on-device | Four separate page images | 4/10 | 5/10 | 6 | 0 | 0 | 0/6 | 2.17 | Not exposed |
| Apple on-device | One four-page image | 2/10 | 1/10 | 8 | 0 | 0 | 0/6 | 1.34 | Not exposed |
| Apple on-device | Four-page image + text | 8/10 | 8/10 | 2 | 1 | 0 | 6/6 | 1.76 | Not exposed |
| Qwen3.5 4B | Text | 9/10 | 9/10 | 1 | 1 | 0 | 6/6 | 4.42 | 1735.1 |
| Qwen3.5 4B | Four separate page images | 10/10 | 9/10 | 0 | 1 | 0 | 6/6 | 54.64 | 9403.1 |
| Qwen3.5 4B | One four-page image | 7/10 | 6/10 | 3 | 1 | 0 | 6/6 | 32.73 | 4864.1 |
| Qwen3.5 4B | Four-page image + text | 9/10 | 8/10 | 1 | 1 | 0 | 6/6 | 35.15 | 5802.1 |
| Qwen3.5 9B | Text | 9/10 | 10/10 | 1 | 1 | 0 | 6/6 | 11.8 | 1735.1 |
| Qwen3.5 9B | Four separate page images | 9/10 | 10/10 | 1 | 1 | 0 | 6/6 | 92.76 | 9403.1 |
| Qwen3.5 9B | One four-page image | 10/10 | 9/10 | 0 | 1 | 0 | 5/6 | 56.82 | 4864.1 |
| Qwen3.5 9B | Four-page image + text | 9/10 | 10/10 | 1 | 1 | 0 | 6/6 | 35.56 | 5802.1 |

Fallbacks include the deliberately ambiguous document and failures. A wrong suggestion is a model output; the experiment never executes it. Median timings exclude failed calls, warmup, PDF rendering and OCR. Timings include transport and model-side image processing. Caching makes some repeated-image requests much faster.

## First visual request in each batch

These samples avoid reusing the same batch image from an earlier condition. They are not a controlled cold-cache benchmark; shared page layouts and server behavior can still affect timing.

| Model | Packed image, 4 batches | Separate pages, 2 batches | Packed image + text, 2 batches |
|---|---:|---:|---:|
| Apple on-device | 1.36 s | 2.42 s | 3.32 s |
| Qwen3.5 4B | 32.73 s | 58.96 s | 40.20 s |
| Qwen3.5 9B | 56.82 s | 92.53 s | 67.79 s |

## Interpretation

- Apple’s image-only reading was weak on the packed pages. Keeping extracted text materially improved this run.
- Keep single-PDF and mixed-document results separate. Qwen 4B filed all seven single-document cases correctly in every input mode, but its packed image alone misfiled all three documents in the mixed sheet. Qwen 9B correctly filed those same three documents from the packed image. This small sample therefore shows model-dependent behavior, not a universal failure of packing.
- Category choice and document-type precision are distinct. A broad policy label can still choose Insurance correctly; correct document reading can still apply a folder rule incorrectly.
- Four pages in one image use fewer Qwen input tokens than four separate images. Reduced text detail after image preprocessing is a plausible risk, but this test does not measure internal resizing. Read the marker scores alongside filing accuracy.
- Text already supplies useful evidence at much lower inference cost. This test does not justify removing OCR or changing Tuck’s default provider.

## Reproducibility and limits

- Read [METHOD.md](METHOD.md) for dataset selection, rendering dimensions, scoring and limitations.
- Apple used its fixed 8192-token window; final Qwen runs used 16384. The initial 8192-token Qwen pilot exceeded context on separate-page inputs and is excluded.
- Qwen decoding used temperature 0, seed 42, thinking off, presence penalty 0 and repeat penalty 1. Apple used greedy guided generation. These APIs do not expose identical decoders.
- [results.json](results.json) contains all 96 scored call records. [cases.json](cases.json) contains frozen expected labels and content hashes, without source paths or document text. [provenance.json](provenance.json) records model digests, runtime versions and rules.
- Personal PDFs, rendered personal pages, OCR text and the private replay manifest remain outside the repository in `/private/tmp/tuck-four-page-eval`. The replay script expects that private manifest and the local compiled helper beside it.
- `AppleGrid.swift`, `run.py` and `summarize.py` are saved for audit; they are evaluation-only and are not bundled into Tuck.
- One pass per condition on a small sample does not establish general model accuracy. Tuck’s production multi-stage coordinator, checks, translation and file-operation validators were not exercised.
