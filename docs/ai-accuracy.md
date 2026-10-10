## Local model switching verification

The app now supports Apple Intelligence and the two installed Gemma choices through the same full workflow. On the frozen 16 synthetic fixtures, correct automatic proposals were Apple 12/14 clear cases, E2B 14/14, E4B 13/14, with zero wrong automatic proposals and both intentionally unclear cases retained. This is a small regression set, not a general model ranking. See [results and limits](../evals/reports/2026-10-01/model-switching/README.md).

# Current workflow refactor — 2026-10-01

Current production: **extract → typed description → suggest folders or match → verify → optional name → safe file operation**. Automatic language detection/translation preflight and download suggestions are disabled. Apple describes the original extracted text using three guided fields: description, exact source quote and clear/unclear. Rust manages bounded extra reads. Category planning takes plain description strings with no document IDs. Matching retains the evaluated one-field category enum; verification receives a flat, focused input.

See [architecture, schemas and official design references](ai-architecture.md) and [evaluation results](../evals/reports/2026-10-01/workflow-refactor/README.md). Earlier results below describe historical pipelines; they are not current performance claims.

---

# Historical description-first flow

The app now runs `read → describe → group or match → verify match → validate → organize`. `description.rs` is shared by Suggest categories and individual filing. Understanding uses free-form factual sentences, source-checked quotes and explicit uncertainty. It receives document content without folder rules or filenames, so the same understanding can be reused after renames and rule changes. Document IDs and hashes come from Rust.

Matching now uses a guided one-field category enum with enabled folder names only. The reader handles extra pages before matching. The separate verifier still checks inherited rules and source-group requirements, and code checks source quotes and allowed destinations. Ambiguous or unsupported results need user input. Duplicate sibling rules are rejected by code. Filename generation is a separate optional call; failure retains the original filename. Automatic moves still require Start Tuck and existing source/destination checks. Roles run serially under a provider-neutral coordinator; no role has a writable LLM tool. See [AI architecture](ai-architecture.md) for role contracts and budgets.

The production flow supplies an understanding record to the Apple bridge. Its legacy fixed-type branch is retained for older direct helper/evaluation callers only; it is not the app's description stage. Earlier accuracy numbers below describe earlier pipelines and do not establish this pipeline's accuracy. See [storage](storage.md) for cache invalidation and [category suggestions](category-suggestions.md) for context limits.

## Historical production: simple guided matching (2026-10-01)

The generic-name, no-rules matcher is now integrated. The full 16-document synthetic rerun used fresh extraction/descriptions, matching, verification and naming: 13/14 clear inputs had the expected raw category, 11/14 received a correct automatic filing proposal, and 0 received a wrong automatic proposal. Both unclear inputs stayed for input. There were zero model API/refusal/format errors; median full-pipeline time was 2.83 seconds. These are small synthetic results, not a real-world accuracy estimate.

The verifier rejected the bundle/contract mismatch and two correct categories (a Swedish insurance scan and an employment-certificate PDF). The scan was inaccurately described as a warranty letter; the certificate was correctly understood but still rejected. Rules remain in verification and naming. The matcher receives names only. See [inputs, traces and full results](../evals/reports/2026-10-01/category-enum/production/README.md).

Build, 80 Rust tests, 31 UI tests, Swift contracts and Clippy passed. Existing saved folders/rules remain intact. The installed app has not been replaced.

## Refusal investigation (2026-10-01)

The [full report and synthetic results](../evals/reports/2026-10-01/refusal-investigation/README.md) record the prompt/validation fixes and the explicitly approved Apple guardrail-mode change.

- Same 16 cases: correct automatic filing improved from 2/14 clear documents to 10/14 with default guardrails; no wrong automatic folder choices in either run. Both intentionally unclear inputs fell back.
- After explicit approval, the production helper uses Apple's content-transformation mode for the five document roles. The final 16-case rerun reached 12/14 clear documents correctly filed, 0 wrong automatic folders and no refusal/guardrail errors. Both intentionally unclear cases fell back. This mode skips runtime string-content checks; model refusals remain possible.
- Fresh semantic verification prevented wrong proposals observed in early variants. It also rejected an electricity bill and scanned employment certificate that should have been filed. Names still need improvement.
- Rust: 80 tests passed, nine live tests ignored. Clippy and standalone Swift output-contract checks passed. The focused later-page test read actual chunk 5 successfully.
- The earlier default-guardrail category setup missed a synthetic bank document; the final production configuration passed all three files and source-hash checks. No personal documents were moved; the installed app was not rebuilt/relaunched.

## One-field guided matcher experiment (2026-10-01)

The requested literal category output was implemented in a temporary Apple helper with an enum containing the enabled folder paths plus Needs input. A schema and explicit example were supplied; the typed property was read directly, without parsing generated JSON. Two example variants each completed the same 16-case evaluation, but each filed only 2/14 clear documents and hit 12 refusal/guardrail errors, versus 12/14 and zero such errors for the production path at that checkpoint. Both unclear cases stayed for input; no wrong automatic folder was selected. That rule-heavy variant was not adopted. See [patches, inputs and results](../evals/reports/2026-10-01/category-enum/README.md).

### Simplified matcher without rules

A follow-up kept the same guided category schema but removed folder/parent/shared rules and source labels from the matcher input. All 16 cases completed without refusal, guardrail or decoding errors. It correctly filed 10/14 clear documents; the verifier caught three wrong proposals and rejected one correct proposal. Both unclear inputs selected the fallback. At that checkpoint, the app retained the earlier matcher (12/14 in its recorded run). The generic-name variant below was subsequently promoted. [Exact prompt, patch and results](../evals/reports/2026-10-01/category-enum/no-rules.md).

### Local models on the same simplified matcher

Five installed Ollama models and a fresh Apple control were tested on the same 16 frozen synthetic description/evidence inputs. This isolates category matching with an enum of folder names; the common Apple verifier was evaluated separately. It does not compare PDF/image reading or complete provider pipelines. [Results, timing, input hashes and runner](../evals/reports/2026-10-01/category-enum/ollama/README.md).

### Generic category names

The same six matching providers were rerun with seven flat generic category names and labels frozen before inference. Personal folder hierarchy and source-bundle preferences were removed; all document descriptions/evidence were preserved. The old bundle/contract fixture is explicitly reported as ambiguous under type-based filing. [Full results, category definitions and ambiguity breakdown](../evals/reports/2026-10-01/category-enum/generic-categories/README.md).

## Description-first verification (2026-09-30)

- Rust regression suite: 69 passing tests, including cache reuse across category suggestions, changed rules, renamed documents and filing; changed content triggers regeneration. Cache corruption, symlinks, quote grounding and bounded extra reads are covered.
- UI: 31 Playwright tests passed. Frontend build and Clippy passed.
- Live Apple category smoke: three synthetic documents produced Financial Records, Housing Agreements and Employment Contracts.
- Live open-vocabulary smoke: an equipment maintenance report was described and matched to Equipment, reusing its cached description. This type is outside the legacy document-type list. Original bytes stayed unchanged.
- The older live filing smoke did **not** pass completely: the Swedish bank statement reached Finances, but an insurance example hit an Apple guardrail during folder selection and stayed in Needs Review. A naming refusal left the original filename intact. The remaining cases in that fail-fast smoke were not reached; no full accuracy percentage is claimed.
- Initial description attempts treated synthetic examples as ambiguous. The schema now separates whether the document purpose is clear from authenticity/completeness; uncertainty concerns filing-relevant understanding only.

These small synthetic checks establish wiring and selected behavior, not broad document accuracy or measured improvement over the previous pipeline. Guardrails remain enabled.

---

# Apple Intelligence and document accuracy

## Historical pipeline — filing-v6, 2026-09-28

Tuck uses Apple's on-device model only. The coordinator is deterministic; model calls have bounded read-only tools. No database, cloud model or persistent agent is involved. Only Rust performs file operations after Start Tuck or manual approval.

```text
PDF / image / text
        |
        v
PDFKit text or Vision OCR (automatic language detection)
        |
        v
NaturalLanguage detection + this Mac's language support
        | supported                  | unsupported
        |                            +-- installed translation -> local English text
        |                            +-- downloadable -> show instructions, manual review
        |                            +-- unavailable -> manual review
        v
Guided document type + exact evidence passage
(+ original image / selected PDF page on supported macOS 27 Macs)
        | missing evidence -> bounded page/OCR read -> try again
        v
Explicit source-bundle association, or semantic folder choice
(+ image / selected PDF page for semantic folder choice)
        |
        v
English document-type template, or original filename
        |
        v
Code checks every suggestion; one AI check for difficult cases
        | disagreement -> choose a folder / keep original name
        v
Start Tuck -> clear match -> Rust rechecks -> copy/move
           -> uncertain/duplicate -> user input -> Rust rechecks -> copy/move
```

### Grounding and names

The reading schema selects a quote from actual extracted lines and a document type from an English vocabulary. Each excerpt contributes at most 12 quote candidates of 220 characters, spread across its lines (including long OCR lines), with at most 48 candidates across the bounded evidence. This avoids taking candidates only from the beginning. The quote is independently checked against the extracted text after whitespace/case normalization. Initial evidence (up to 8,000 characters) and each retrieved page (up to 4,000) remain in a labelled ledger; a new page no longer deletes earlier evidence. Text is still excerpted, not a complete representation of a long document.

A quote's presence proves only that the text exists. It does **not** prove that the model interpreted it correctly or resisted an instruction embedded in it. The selected quote, document type and source evidence inform semantic routing using enabled folder prompts. Explicit bundle associations must cite a distinctive name present in the real source folder and selected instruction. Parent context cannot widen a child's required instruction, but this remains a model interpretation rather than a logical guarantee.

Names now come from conservative English templates such as `Bank Statement`, `Residence Permit Card` and `Employment Certificate`. They contain no invented person, organization, date, validity or side. `Other`, `Unknown`, or missing evidence retains the original filename even if a bundle has a usable destination. There is no separate naming generation that can discard a successful folder decision. Free-form filename patterns in instructions are not applied in this MVP; edit the name in review for additional details. The model can still choose the wrong document type, so an automatic name can still be incorrect.

### Evidence gathering and conditional checking

When the model has no grounded document type, the Rust bridge can offer a bounded next read: a text chunk clipped from the initial 8,000-character excerpt, a clipped/unread PDF page, or one OCR retry for sparse image/PDF text. Completed requests are excluded. The model can still explicitly request a referenced page. The existing three-tool and time budgets cover both paths. There is no open-ended search or model-controlled path. Reading succeeds independently of classification: Apple can still refuse to classify the additional evidence.

Code checks run on every usable proposal. In addition to existing path, enabled-destination and quote validation, identical normalized descriptions for sibling destinations require clarification. Different parent branches are not treated as identical merely because their leaf descriptions match.

A fresh Apple session checks difficult proposals: source-bundle routing, an ungrounded or `Other` type, successful or failed tool follow-ups, or a folder fit reported as uncertain. The organizer's fit label is not a calibrated probability and can miss ambiguity. Clear direct proposals skip the extra model call.

The checker receives shared rules once and separately scoped topic rules for the candidate and alternatives. It does not receive the organizer's rationale. Ordinary checks select a numeric destination index independently and separately decide whether the proposed name is supported. The selected destination must agree with the organizer, and the selected quote must exist in the evidence. A disagreement, missing evidence or no matching folder produces a code-generated explanation of that specific failure, rather than displaying a model rationale that claims support while rejecting the folder. If only the name is unsupported, the original name is kept. Source-bundle checks retain their bounded numeric verdict. There is at most one checking call per completed proposal.

The checker is another call to the same on-device model, not an independent authority. Agreement can repeat an error; disagreement can add unnecessary manual work. It never grants permission to move a file. Compare measured outcomes and latency before extending its scope. This feature keeps the custom bounded Rust tool protocol; it does not switch to native Foundation Models `Tool` conformances.

### Source-bundle prompts — filing-v3

Source matching receives source labels, shared root preferences, folder rules and an indexed destination list. Its only output is a numeric selection. Code strictly decodes the integer, maps it to an allowed folder, and verifies a distinctive shared name using whole words. Generic labels such as Incoming or Application Files do not establish bundle membership. Document-type reading and semantic topic routing remain separate tasks.

The fresh checker for a source-bundle proposal uses a concise numeric verdict: both folder and name supported, only name unsupported, folder unsupported, or conflicting destinations. It sees the full candidate/alternative rules and document evidence. Its instructions explicitly give a named-group filing rule priority over an attachment's topic. The same failure handling applies: retain the original name or require a manual folder choice. No failed checker is bypassed.

Measured results and rejected prompt variants are retained in [the filing-v3 comparison](../evals/reports/2026-09-28/prompts/README.md). The AGENCY source-folder example now gets the correct folder but keeps its original name; this does not eliminate refusals or establish general accuracy.

### Explicit examples and optional rule checks — filing-v4

The UI can append up to three short, user-approved positive/negative example blocks to a folder's rule Markdown. Rules with `## Filing example` activate a concise extra routing instruction: a negative example excludes only that named destination; consider the other destinations. Examples do not enter document-type extraction or provide evidence for a filename. Their effect is prompt conditioning, not training or a separate memory database. They remain subject to the same inheritance, enable switches and configuration revision checks.

A separate **Check rules** setup action compares enabled sibling descriptions after normalizing whitespace and capitalization. It flags duplicates for editable user clarification. It does not read documents or invoke a model. Broader semantic auditing was removed after live tests showed both false positives and missed overlaps; it remains deferred.

Live results and rejected variants are in the [learning comparison](../evals/reports/2026-09-28/learning/README.md). Examples do not guarantee increased accuracy. The production reading tools remain bounded as before; expanding their reasoning or tool set is not part of this iteration.

### Limits and errors

On macOS 26.4+, the helper counts the actual instruction, prompt and schema tokens before each generation and reserves 600 response tokens plus 256 for framework overhead. It queries `SystemLanguageModel.contextSize`; it does not assume every model has the same window. Oversized requests stop with a stage-specific `context_limit` message rather than dropping folder instructions. Earlier macOS 26 uses a conservative UTF-8 size estimate. This can abstain more often than the tokenizer path. Huge prompts and accumulated pages can still require shorter preferences or manual review.

The reading loop allows at most three read-only tools. Language preparation and model requests share a bounded classification deadline; initial PDF/OCR extraction has its own limit. Errors identify the failing stage and a safe category (refused, guardrail, unsupported language, context limit, busy, timeout, etc.). Apple's raw diagnostic descriptions are never emitted because they can include document content. Guardrails remain enabled.

`confident` means a usable proposal, not calibrated confidence. After Start Tuck, usable proposals can be automatically organized; uncertain decisions and known duplicates remain for input. Mapping and watching add workflow safeguards, not proof of improved model accuracy. Rust validates allowed destinations, original contents, current configuration revisions and exclusive output paths. Retry clears stale generated filenames and language notes.

### Language support and privacy

See [language support](languages.md) for detection thresholds, downloadable resources, and current limitations. Review metadata is in memory only. Downloading Apple resources needs a network connection, but Tuck's OCR, inference and Translation framework processing are on-device. A configured destination can independently be a cloud-synced folder; the privacy claim concerns Tuck's processing, not third-party sync.

### Evaluation

Use synthetic fixtures only. Quality is reported separately from whether the test process completes. Expected review cases must not inflate substantive classification accuracy. The known regression suite is `evals/documents.json`; `evals/multilingual.json` was authored separately before its first live run and includes three rendered image/OCR cases. After its first use, it becomes a regression set rather than an indefinitely held-out benchmark.

```sh
TUCK_EVAL_REPORT=/private/tmp/tuck-apple-eval.json cargo test --manifest-path src-tauri/Cargo.toml live_apple_accuracy_eval -- --ignored --nocapture
TUCK_EVAL_FIXTURES="$PWD/evals/multilingual.json" TUCK_EVAL_REPORT=/private/tmp/tuck-multilingual-eval.json cargo test --manifest-path src-tauri/Cargo.toml live_apple_accuracy_eval -- --ignored --nocapture
swiftc -parse-as-library src-tauri/swift/TuckModelSupport.swift src-tauri/swift/TuckModelTests.swift -o /private/tmp/tuck-model-tests
/private/tmp/tuck-model-tests
```

The validation machine runs macOS 27.0. Older macOS 26 compatibility branches are guarded and compiled but have not been runtime-tested here. Model behavior can change with an OS/model update. Each report records OS version, pipeline version, outputs, language metadata and latency. No personal documents belong in these fixtures or reports.

The checked-v2 comparison is recorded in [the checker report](../evals/reports/2026-09-28/checker/README.md). Earlier measured outcomes and rejected variants are recorded in [the 2026-09-28 report](../evals/reports/2026-09-28/README.md). The installed-translation branch has not been exercised end to end on this Mac because its fallback pair is not installed.

### Image and text reading — filing-v5

Measured outcomes and rejected prompt variants are recorded in [the image-reading report](../evals/reports/2026-09-28/vision/README.md).

macOS 27 Macs whose installed model advertises the vision capability receive the original image during document-type reading. PDFs supply a bounded rendering of the first page, or the latest page requested by the coordinator. OCR/extracted text remains alongside the image for exact quote verification, language handling, categories and context. Images are not a replacement for these checks. Unsupported language gating still applies.

The Rust caller provides the file path; model output cannot choose an attachment or arbitrary path. Images and PDF previews are processed locally. No cloud model is used and no rendered personal-document images are written to disk. macOS 26 and models without vision use the existing text path. Attachment/context compatibility errors can fall back to text; refusals and guardrails propagate without retrying through another modality.

On this macOS 27 installation, the model accepted image generation but its image-prompt token counter rejected the same prompt. When exact image token counting is unavailable, Tuck reserves 3,072 tokens in addition to the counted text/schema/instructions. Oversized visual requests use text; the framework enforces the actual context limit as well. Visual reading is indicated in the document explanation when it succeeds.

The vocabulary now includes Driving Licence and Birth Certificate. The conditional second check selects a destination by index and evaluates the filename separately. Shared root preferences are separated from each folder’s topic rules. An uncertain name can preserve the original filename while retaining a supported folder. Rejection explanations come from the actual validation outcome, rather than contradictory model prose. A verified quote establishes that text exists in the source; it does not prove the model interpreted the type correctly.

Apple documents the attachment API in [Analyzing images with multimodal prompting](https://developer.apple.com/documentation/foundationmodels/analyzing-images-with-multimodal-prompting). A persistent/prewarmed helper, native Tool conformances and custom adapters remain deferred.

## Focused visual folder selection — filing-v6

[Comparison results and limitations](../evals/reports/2026-09-28/focused-vision/README.md): 16/26 substantive documents correctly routed and zero wrong destinations on the same 30-case set, matching filing-v5.

The semantic folder-selection call receives the same original image or selected PDF page as the reading stage. It still has its own folder-choice schema and all existing folder rules. Text evidence, quote validation, source-group handling, filename templates and the conditional checker are retained. This avoids routing solely from OCR and a previous type label while preserving the focused model stages. Source-group matching and the conditional checker remain text-based.

Visual input follows the same capability/context fallback policy as reading. It does not retry a refused request through another modality. Successful visual folder decisions include an explanation marker, allowing evaluations to verify that the routing call actually used the image. This change provides more evidence; it does not establish higher accuracy by itself.

## Single-decision experiment — not adopted

We tested three simpler pipelines with image/text evidence and folder/type selection in one call, followed by code validation. All three regressed on a frozen 30-case comparison and were not adopted. Filing-v6 retains the focused stages and adds image evidence to folder selection. [Measured outcomes, rejected variants and reproduction instructions](../evals/reports/2026-09-28/simple-pipeline/README.md).

## Official references

- [Apple: guided generation](https://developer.apple.com/documentation/foundationmodels/generating-swift-data-structures-with-guided-generation)
- [Apple: context management](https://developer.apple.com/documentation/foundationmodels/managing-the-context-window)
- [Apple: evaluate prompts](https://developer.apple.com/documentation/foundationmodels/evaluating-prompts-to-measure-performance-and-improve-model-responses)
- [Apple: supported languages](https://developer.apple.com/documentation/foundationmodels/supporting-languages-and-locales-with-foundation-models)

## Historical measurements (older pipelines)

The following results predate the current implementation and do not describe this build.

## Initial full-pipeline baseline (before semantic folder instructions)

The first run of the Apple-only evaluation selected 11/12 expected folders. Automated title checks passed for 3/10 substantive documents; both expected review cases also satisfied the empty-name check (5/12 total). Manual review accepts one additional `Rent Agreement` title. The embedded-instruction case still produced an inappropriate name. See [the report and limitations](../evals/README.md). That baseline included a deterministic keyword matcher. Subsequent semantic-instruction results are recorded separately and do not establish general accuracy.

## Semantic-instruction follow-up

The previous flat semantic pipeline scored 9/12 expected folders and 4/12 automated title checks (including two expected review cases). The AGENCY/context feature assertions pass, but the broader set regresses from the keyword-assisted baseline. See [the recorded results](../evals/README.md#semantic-folder-instructions--2026-09-27). Those scores predate the inherited folder prompts and separate naming stage; they are not measurements of the current build. Evidence-based naming and resistance to embedded instructions remain future work.

Instruction files are authoritative and rechecked before file operations. A changed revision invalidates an in-flight decision. See [folder instructions](folder-instructions.md) for inheritance and recovery. This strengthens configuration consistency, not model accuracy.

## Folder-file follow-up (2026-09-27)

The final smoke run passes Swedish banking, English insurance, unreadable-document fallback, and reading evidence from text chunk 5. The semantic/context suite passes five of six cases: source-folder AGENCY association, disabled destination, semantic tenancy, migrated AGENCY source association, and unrelated Incoming folder. The remaining filename-only AGENCY case incorrectly selects Work. It passed an earlier prompt iteration but regressed when root preferences were deduplicated to restore the Swedish-bank case. The live suite intentionally retains that failing assertion; model accuracy is not declared improved.

A focused inheritance run selected `Home/Rental` for a synthetic tenancy agreement and supplied an English name. Disabling Home excluded both Home and Rental, but Apple incorrectly chose the remaining Finances destination instead of abstaining. The disabled-branch assertion covers exclusion only; this is a recorded semantic failure. One name included the original extension; file operations now remove a matching generated extension before appending the real one.

A separate category-then-child model search was tested and removed because it regressed the filename-bundle case. The shipped flow uses one enabled-destination selection pass with inherited prompts, then optional naming. These targeted checks do not establish an accuracy improvement; the full evaluation has not been rerun for this version.
