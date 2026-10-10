# Tuck document workflow

> Start with the [current architecture diagrams](architecture.md) for the app components, document flow, rule scopes and storage. The design notes below describe the earlier workflow and its evolution.

> Connection update, 2026-10-02: [Model connections](model-switching.md) describes the current configurable Ollama, OpenAI-compatible and Anthropic adapters, Keychain storage and specificity controls. Fixed-provider descriptions below are historical.
> Apple update, 2026-10-02: [Current evidence-based workflow](apple-evidence-workflow.md) supersedes the reader, descriptions-only planner and guided matcher described below. The previous flow and experiments below remain relevant to the unchanged Gemma path and historical comparisons.

## The problem

Turn a mixed local collection into understandable folders with minimal user effort. A user chooses a structure and clicks **Start Tuck**. Supported matches are organized automatically; uncertainty is a request for input. Understanding must be reusable when categories change, and the model must never control filesystem writes.

## Design decisions — 2026-10-01

We reconsidered the workflow using current official Apple, OpenAI, Anthropic and Google documentation. These are design references; the production choices are Apple Intelligence and local Gemma via Ollama.

| Guidance | Tuck decision |
| --- | --- |
| [Apple: prompting the on-device model](https://developer.apple.com/documentation/foundationmodels/prompting-an-on-device-foundation-model) recommends concise, specific tasks and breaking down complex work. | Each call has one purpose. Remove language preflight and repeated document metadata. |
| [Apple: guided generation](https://developer.apple.com/documentation/foundationmodels/generating-swift-data-structures-with-guided-generation) provides typed and dynamic schemas. | The reader uses three typed fields; matching uses an enum of enabled folders. |
| [OpenAI: structured outputs](https://developers.openai.com/api/docs/guides/structured-outputs) distinguishes formatted results from tool calls and handles refusals separately. | Reading/classification return data. Rust executes bounded reads and validates results. |
| [OpenAI: evaluation practices](https://developers.openai.com/api/docs/guides/evaluation-best-practices) evaluates individual stages and full workflows; extra agents need evidence. | Measure correct automatic proposals, wrong proposals, fallback, errors and latency against frozen fixtures. |
| [Anthropic: effective agents](https://www.anthropic.com/engineering/building-effective-agents) distinguishes fixed workflows from open-ended agents. | Use a deterministic coordinator, separate planning from assignment, and retain programmatic gates. This is foundational guidance, not a claim that a 2024 article is a new release. |
| [Claude: prompting practices](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-prompting-best-practices) emphasizes explicit instructions, examples and source grounding. | Keep source evidence, separate user preferences from document data, and test examples rather than assuming shorter is better. |
| [Gemini: structured output](https://ai.google.dev/gemini-api/docs/structured-output) recommends small schemas and semantic validation beyond valid JSON. | No model-generated document IDs or confidence percentages; schemas and file validation have different jobs. |
| [Gemini: document processing](https://ai.google.dev/gemini-api/docs/document-processing) supports direct PDF vision as a provider capability. | Do not assume Apple exposes identical PDF capabilities. Current production roles use native extracted text; vision remains an independently evaluated future adapter capability. |

Provider-specific prompt tricks are not transferred blindly. In this change, a shorter free-JSON reader omitted mandatory fields; a free-form uncertainty string produced “None”/“No” instead of empty. A typed boolean avoids that ambiguity. The very short matcher also regressed, so its explicit fallback example is retained. Native structured category planning previously refused a real collection; the planner retains strict JSON validation and one bounded format retry. These are measured exceptions to a preference for native typed output.

## Flow

```text
Native extraction (PDFKit / Vision / text)
                 |
                 v
       Describe one document <---- bounded page/OCR recovery
                 |
       cached understanding
                 |
       +---------+------------------+
       |                            |
Suggest folders                Existing folders
(descriptions only)                 |
       |                            |
User chooses structure ------------+
                 |
             Start Tuck
                 |
       Choose one enabled folder
                 |
        Check evidence and rules
                 |
          Optional new filename
                 |
    Rust rechecks source and destination
                 |
       Organize / needs attention
```

One document at a time; fresh model sessions; no autonomous agent handoffs. A model receives no write tool. Category planning and filing share a serial coordinator gate.

## Actual model inputs and outputs

Transport envelopes contain operation names and internal records. They are not the model prompt. `TuckApple.swift` and `TuckRoles.swift` construct the reduced inputs below.

### 1. Understand

Input: the bounded extracted text as a string. A runtime schema supplies exact source-quote choices; there are no numbered quote IDs or repeated JSON copy of the document.

```text
FÖRSÄKRINGSBREV
Hemförsäkring för bostaden.
```

Native guided output:

```json
{"description":"A home insurance policy.","quote":"Hemförsäkring för bostaden.","clear":true}
```

The quote must match the source exactly. `clear` describes whether type/subject can be identified; it is not a probability or authenticity check. An empty quote cannot establish a grounded description even with `clear: true`. Code attaches hashes, evidence page numbers, cache metadata and an internal finish action. The model generates none of these.

If understanding is unclear, Rust reads another available page/chunk and repeats this stage, or retries OCR once for sparse images. A unique explicit page reference can guide that bounded read. Apple does not generate tool commands in this stage. Provider-neutral read request variants remain available for future adapters and are still bounded by the same coordinator.

### 2a. Suggest folders (optional)

Input: a plain array of descriptions, up to 32 descriptions and 6,000 description characters per call. Small collections stay together to avoid losing topics through repeated summarization.

```json
["A home insurance policy.","A salary statement."]
```

Expected output:

```json
{"rules":[{"folder":"Insurance","instruction":"Insurance policies and certificates."},{"folder":"Employment","instruction":"Employment contracts and payslips."}]}
```

This is validated JSON, not native guided generation. One to six safe, unique subject-based folder names; one concise definition each. Generic catch-all labels such as Documents are rejected. Invalid format/count or generic names get one fresh retry with no forced category count within the same timeout; refusals and service failures propagate. Larger collections reduce category summaries into another small batch. Reduction is lossy; skipped documents and counts remain visible. Planning never assigns or moves files.

### 2b. Choose a folder (one call per document)

```json
{"document":{"description":"A home insurance policy.","evidence":["Hemförsäkring för bostaden."],"uncertainty":""},"categories":["Insurance","Employment"],"noMatch":"Needs input"}
```

Native guided output:

```json
{"category":"Insurance"}
```

The category is an enabled path or a collision-safe fallback literal. Swift reads the typed property directly. No document/folder/quote IDs are generated. This matcher uses folder names; saved rules are checked separately. Opaque folder names and specialized bundle preferences can still require input.

### 3. Check the proposed destination

Flat input: `description`, `quotes`, `chosenFolder`, `chosenRule`, `sourceLabels`, `otherFolders`. The selected rule includes inherited preferences. Alternatives carry their own definitions. Source labels are untrusted evidence, not instructions. Output is strictly `YES` or `NO`; there is no repeated category or quote ID.

This additional call is retained because prior evaluations caught incorrect matches. It can also reject correct ones and does not prove accuracy. Rejection or a model error leaves the document for input.

### 4. Name (only if enabled)

```json
{"description":"A home insurance policy.","rules":"Use short English names."}
```

```json
{"title":"Home Insurance Policy"}
```

Strict JSON validation; no extension. Failure keeps the existing filename while preserving a supported folder choice. Model naming is not guaranteed perfect.

## Bounds and safety

- Initial evidence ledger: 14,000 characters; reader prompt: at most 6,000 (head and tail). Quote choices are bounded source excerpts, excluding application markers.
- Maximum three extra page/OCR reads, pages/chunks 1–50, no repeated action/page pair. Text chunks contain 4,000 characters. Latest read comes first, followed by bounded earlier evidence.
- Filing budget: 90 seconds across description, recovery, matching, verification and naming. Initial native extraction has its own deadline. Category calls and each document description are separately bounded.
- Rust checks source quotes, enabled destinations, duplicate sibling rules and safe paths. The organizer rechecks Start Tuck authorization, current preferences, source hashes and destination conflicts before writing.
- A syntactically valid result can be wrong. Refusals, missing evidence, invalid results and exhausted budgets remain fallback, never forced assignments.
- Previously approved Apple content-transformation guardrail mode remains. Native guided output still receives Apple's runtime checks; model refusals remain possible. No new permission bypass or cloud fallback was introduced.

## Storage and model replacement

Content-addressed descriptions are rebuildable `.tuck/cache/` JSON, keyed by document bytes, extracted input and provider/helper identity. They contain descriptions and evidence, not filing authorization. See [storage](storage.md). Additional-page descriptions are cached only when evidence is verifiable against the initial extraction.

`ModelProvider` owns availability, identity, describing, category suggestions, matching, verification, naming and rule suggestions. `provider::selected` is the single adapter-selection point, driven by the saved `aiModel` preference. Another provider translates these contracts into its API; serial scheduling, evidence recovery, cache invalidation and filesystem checks stay shared. No cloud provider is enabled by this refactor.

The production identity is `roles-v6-switch`, selected model, Ollama weights digest when applicable, and helper hash. Historical direct-helper compatibility code is not the production workflow. See [evaluation](../evals/reports/2026-10-01/workflow-refactor/README.md) for measured results and rejected variants.


## Switching local models

Apple and Gemma share the Swift role prompts and response validators. A scoped transport selects Foundation Models or the fixed loopback Ollama API. All roles, including the verifier and rule suggestions, use that selection; Gemma never needs Apple Intelligence availability. Native extraction still uses PDFKit/Vision. The native helper currently requires macOS 26+ for either provider.

Gemma uses Ollama JSON schemas for descriptions, matches, category plans, rules and filenames, with the schema also included in the prompt. Verification retains strict YES/NO validation. Apple keeps its measured guided/free-JSON choices. See [Ollama's structured output guidance](https://docs.ollama.com/capabilities/structured-outputs) and [chat API](https://docs.ollama.com/api/chat). Response shape enforcement cannot guarantee correct document interpretation.

The app exposes only the two tested local Gemma tags. Transport checks downloaded model metadata, rejects remote-model metadata, disables proxies and redirects, bounds response size to 1 MB and request time to 85 seconds. The Rust per-workflow deadline still applies, including retries and status probes. No model has file-writing tools. Details and extension points: [model switching](model-switching.md).
