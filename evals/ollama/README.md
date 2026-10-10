# Local Ollama comparison

Evaluation only. This does not add a provider setting or change the installed app.

## Run

On the Apple Intelligence Mac used for Tuck, with Ollama running locally:

```sh
ollama pull qwen3.5:4b
ollama pull qwen3.5:9b
python3 evals/ollama/run.py /private/tmp/tuck-ollama-new-run --apple
python3 evals/simple-pipeline/compare.py /private/tmp/tuck-ollama-new-run/filing-v6-apple.json /private/tmp/tuck-ollama-new-run/filing-v6-ollama-qwen3.5-4b.json /private/tmp/tuck-ollama-new-run/filing-v6-ollama-qwen3.5-9b.json
```

The output directory must be new. No cloud service or account is used. Model downloads use Ollama's registry; document inference uses only `127.0.0.1:11434`. Downloaded models stay installed. The runner unloads its benchmark models after each run.

## What stays the same

- The frozen 30 synthetic cases in `evals/simple-pipeline/comparison.json`: 26 documents with an expected category and four cases expected to need a person.
- Production `TuckApple.swift`: reading, bounded tool choices, source group matching, folder selection, filename templates, conditional independent checking and deterministic validators.
- The Rust coordinator, evidence excerpts, user rules, local OCR, language detection/translation and time limits. This is a controlled model comparison, not a test of replacing Apple's language services.
- Original PNG inputs and the same PDF page thumbnail dimensions (1600 by 2200 bounding size). The same Apple context gate decides attachment eligibility.
- The model can only propose decisions. The harness never moves documents.

## Necessary transport differences

`build.py` removes only the production `generate` function from a temporary copy of model support, replacing it with `OllamaTransport.swift`. It compiles the existing production main alongside that copy. The application and its installed helpers stay unchanged.

Apple's `GenerationSchema` is encoded directly as JSON Schema, preserving guides, enums, descriptions and `x-order` property order. Ollama receives the schema in its `format` parameter and rendered in the user message so the model can read field descriptions. Apple exposes schema descriptions through its guided generation API. Thus source prompts and evidence are reused, while the providers' actual token streams and decoders differ.

Ollama settings: temperature 0, seed 42, thinking disabled, context 8192, per-call output limit from production (normally 600), presence penalty 0, repeat penalty 1. Its HTTP timeout is 85 seconds inside the existing 90-second coordinator deadline. HTTP or generation failures become manual fallbacks; there is no retry to find a better answer. Ollama models run sequentially after a neutral warmup. Warmup/load timing is separate from per-document timing.

Apple framework error strings still say Apple because the production decision code is deliberately reused. The report's pipeline label identifies the actual model. In this experiment Apple availability remains a prerequisite for identical preprocessing; this is not a standalone Ollama product integration.

## Interpreting results

Report correct automatic destinations, wrong automatic destinations, unnecessary manual fallbacks and correct manual fallbacks separately. A process exit of zero confirms harness completion, not classification accuracy. Filename checks are fixture-specific automated assertions, not a complete semantic naming audit.

This is a known regression suite, not held-out evidence of general accuracy. Only two cases are rendered image/PDF inputs; the others use synthetic text. Adaptive tool calls can differ when models make different decisions. A single run per model is observational, and model choice should not be generalized beyond this Mac and these cases.

Call metrics contain model identifiers, schema names, timing, token counts and errors, without document text or prompts. Reports contain only synthetic fixture decisions. The manifest records source hashes, installed model digests and warmup information.

Official references: [Qwen3.5 4B](https://ollama.com/library/qwen3.5:4b), [Qwen3.5 9B](https://ollama.com/library/qwen3.5:9b), [structured outputs](https://docs.ollama.com/capabilities/structured-outputs), [image inputs](https://docs.ollama.com/capabilities/vision).
