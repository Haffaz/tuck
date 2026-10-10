# Controlled classifier-prompt comparison

## Question

Does the proposed role, purpose and examples prompt improve recognition compared with the previous short prompt, while leaving document evidence, schema and model settings fixed?

## Fixed design

- Models: the already installed Gemma 4 E2B and E4B Q4_K_M tags through local Ollama 0.34.4. Models run sequentially and unload afterward.
- Thirteen documents: ten previously used development documents (six synthetic, four real) plus three unused PDFs from the user-approved Tuck Test - 1 folder.
- Fresh PDFs were selected from files not previously sent in these benchmarks, excluding exact source-hash duplicates. Contents were inspected and expected labels frozen before either prompt was run. They are all single-page PDFs and give only a small, English-dominant fresh-data check.
- Two input modes: extracted text and page image. Seven original documents use their existing four-page contact sheets. One original two-page PDF uses a padded grid; original and fresh single-page PDFs use individual page images.
- Two prompts: exact prior short prompt and the proposed classifier prompt with role, purpose, specificity and examples. Both are run again; historical scores are not substituted.
- 104 scored calls: 2 models x 13 documents x 2 input modes x 2 prompts.
- One document per call. Document IDs and expected answers are runner metadata only. The output JSON has only documentType. Existing page IDs can still appear in reused image pixels.
- Each paired request has byte-identical user content and images, schema and settings. Only system-prompt text changes. An evidence hash verifies this.
- Temperature 0, seed 42, thinking off, 16384 context, 700-token limit, presence penalty 0 and repeat penalty 1. No retries for improved answers.
- Neutral warmups excluded. Text/image order alternates across documents. Old/new order alternates across documents, modes and models to reduce systematic cache/order bias.

## Scoring

Expected document identities and accepted synonyms are in rubric.json. Scoring accepts equivalent English names. If a novel synonymous wording appears, it is adjudicated against the frozen definition and recorded separately, consistently across both prompts. It does not change the underlying expected identity.

Specificity matters: generic Document, Decision or Insurance Policy labels do not meet the expected immigration-decision or home-insurance identity. Unknown is correct for the deliberately unidentifiable packet. Existing accepted hotel-booking and membership-order aliases are preserved. Fresh ETA receipt/acknowledgement must not be labeled as an approval. Model outputs are manually inspected before publishing.

## Interpretation limits

- This isolates the effect of the complete system-prompt replacement, not the separate effects of role, examples, length or wording.
- The proposed prompt was developed after seeing the original ten documents. Its Unknown example closely resembles the ambiguous synthetic case. Development improvements must not be described as held-out accuracy.
- The three fresh PDFs were not used to write the proposed prompt. They are too few to establish general accuracy, and some categories are adjacent to examples in the prompt.
- Six controlled four-page packets have generic covers, transmission notes and prominent synthetic watermarks. Their operative content is on page four. Report synthetic and real behavior separately; this is not a representative random document sample.
- Image inputs contain all selected page pixels, but internal image preprocessing can lose small text. No claim is made about exact internal resizing or its causal contribution.
- Timings are per request and exclude loading, downloads, rendering and OCR. Models can reuse prompt/image caches. Counterbalanced order reduces bias but does not make this a cold-cache benchmark.
- Prompt improvements cannot recover unreadable input or prove safe automatic filing. No folder selection, naming, file operations or production coordinator is tested.
- No source PDFs or production app code change. Personal text/images/paths remain in the private scratch directory. Published results include only model type labels, metrics, document keys and hashes.
