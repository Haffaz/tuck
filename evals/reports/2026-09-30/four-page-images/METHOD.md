# Four PDF pages per image: local input comparison

## Question

Does rendering four PDF pages into one 2x2 image improve document filing compared with extracted text? Does retaining text help? Compare Apple's current on-device Foundation Model with locally installed Qwen3.5 4B and 9B via Ollama.

## Frozen inputs

- Eight four-page batches, representing ten documents: six synthetic four-page documents and four real documents from the user-approved Tuck Test - 1 folder.
- Synthetic subjects: Swedish residence decision, employment certificate, Swedish home insurance, bank statement explicitly in a AGENCY migration bundle, Swedish tenancy agreement, and an intentionally unidentifiable administrative document.
- Real batches: first four pages of a five-page immigration decision; a mixed four-page batch containing a two-page hotel booking, a one-page membership order confirmation and another one-page hotel booking.
- Ground truth was assigned from the rendered pages before scored inference. Several real filenames are misleading, so they were not treated as labels. Real-document outputs in the report contain document types and target categories only.
- Every condition receives the same document-ID/filename/page manifest and same filing rules. The six synthetic cases have a small four-character marker on page four to measure fine-text reading independently from classification.

## Input conditions

1. **Text:** Tuck's PDFKit text extraction or Apple Vision OCR, with page IDs. Original language, no translation.
2. **Separate pages:** four labeled PNG page images; no extracted text.
3. **Four-up:** one 2x2 PNG contact sheet; no extracted text.
4. **Four-up + text:** the same contact sheet plus the same extracted text as condition 1.

Each page is rendered at 1200 pixels wide, placed in a 1240 x 1760 labeled tile. The contact sheet is 2480 x 3520. Both visual conditions therefore start with identical page pixels. Internal model image resizing/tokenization is provider-dependent and not observable here. The real longer PDF's fifth page is excluded identically from all conditions; this is a four-page-window experiment, not a complete-PDF comprehension test.

## Model calls

This is a direct, single-pass classification experiment. It does not run Tuck's multi-stage production coordinator, conditional checker, language translation, filename templates or file operations. No files are moved, renamed or deleted. No production app or provider settings change.

The requested structured output contains one record per document: ID, document type, folder and the synthetic verification marker (empty for real documents). There are no model-controlled tools or follow-up reads. Failed calls are recorded, not retried for a better answer. Image conditions cannot silently fall back to text.

Apple uses SystemLanguageModel.default, its native guided schema, greedy decoding and an 8192-token context. The API exposes this current on-device model; this test does not access Apple's cloud models or historical model versions.

Qwen uses the same instructions, prompt and schema descriptions through Ollama /api/chat on 127.0.0.1, thinking off, temperature 0, seed 42, presence penalty 0, repeat penalty 1, a 700-token output limit, and a 16384-token context. Apple's dynamic schema is exported as JSON Schema; Ollama also receives its text, with x-order respected for field generation order. This transport difference is explicit.

An initial 8192-token Qwen pilot rejected separate-page requests of about 9400 input tokens. It is archived separately and excluded from the final comparison. Qwen was restarted at 16384 for every final condition. The model input window is therefore not equal across providers, but all modes within each provider use one configuration and the same evidence.

Models run sequentially with a neutral text warmup excluded from scoring. Condition order rotates across batches. Timings are observed per four-page request, including transport and image processing inside the model server, excluding PDF rendering, OCR, compilation and warmup. Prompt/image caching and condition order can affect timings. No cold-cache latency claim is made. A timeout is 120 seconds for Apple and 240 seconds for Ollama.

## Scoring and limits

Folder accuracy counts exact matches to predefined topic rules, including the intentional Needs Review case. Wrong destinations, fallbacks and request failures are also reported separately. Document-type scoring uses predefined labels; the synthetic residence decision accepts Residence Permit or Immigration Decision, and the real membership order accepts Membership Confirmation, Order Confirmation or Receipt. Broad but less specific labels may fail the document-type score while still filing correctly.

Marker accuracy measures exact reading of the six known synthetic markers. There is no identity, date, amount or other personal-field extraction test.

This is a small exploratory sample with one run per condition. Synthetic documents share simple layout templates, the personal sample contains only four documents, and category rules are explicit. These results should not be presented as general model accuracy, a production safety guarantee, or a calibrated confidence estimate.
