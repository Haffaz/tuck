# Gemma category-planning comparison — 2026-10-01

## Question and scope

Can the installed Gemma models propose useful categories with the same approach as current Tuck?

This comparison isolates **category planning**, using exactly the same description arrays, instruction text and production Swift output validator. It does not compare extraction, vision, document understanding, individual folder matching, verification, naming or file moves. Apple remains the installed app's provider. No cloud model calls, new downloads or document moves occurred.

## Inputs and protocol

- Controlled collection: ten synthetic descriptions spanning five known topics: Identification, Travel, Employment, Insurance, Education. Category meanings and coverage were inspected, not judged by exact wording alone.
- Personal collection: the same 22 successful cached descriptions from the 25-document Tuck Test - 2 folder. The three skipped documents are not represented in any provider's input. Source bytes and provider identity matched the cached records. Final preparation also reproduced all 22 extraction hashes and supporting quotes. An initial preparation attempt encountered an OCR mismatch and stopped before inference; the completed benchmark uses one frozen array shared by every provider.
- Prompt/retry text extracted directly from current `TuckApple.swift`. Inputs are description strings, with no document IDs.
- Both Apple and Gemma use ordinary JSON generation followed by the **same** compiled `validateCategoryPlan` implementation. Gemma does not receive an extra JSON-schema decoder constraint in this experiment. Invalid format/count/generic names permit one fresh retry within a 90-second budget. Apple uses its production retry implementation internally; its helper does not report attempt counts. Gemma needed no retries.
- Three measured repetitions per provider/dataset; sequential calls. A neutral warmup is excluded from measured times. Gemma models are loaded individually and unloaded afterward; no previously active Ollama model was displaced.
- Ollama: temperature 0, seed 42, context 8192, output budget 1000, thinking disabled. Apple: production greedy sampling and 1000 output tokens. Equivalent task contracts do not imply identical tokenization or runtime internals.
- Models are the **locally installed builds** tagged `gemma4:e2b-it-q4_K_M` and `gemma4:e4b-it-q4_K_M`. Their metadata identifies validation-derived parent builds; the exact digests are recorded in provenance.json. These results are not a claim about every build distributed under a similar model name.

## Results

Median warm category-planning latency, seconds:

| Provider | Controlled, 10 descriptions | Personal, 22 descriptions | Personal topic sets across 3 runs |
| --- | ---: | ---: | --- |
| Apple | 1.61 | 2.24 | Same each run |
| Gemma E2B | 2.34 | 3.25 | Two different sets |
| Gemma E4B | 4.62 | 7.78 | Same each run |

All 18 outputs passed the production validator. Neither Gemma produced Documents/Other catch-all categories. All three providers covered the five controlled subjects with appropriate definitions in all three runs.

Personal suggestions:

- Apple: Education, Travel, Identification, Employment, Legal, Vehicle.
- Gemma E2B, first run: Education, Identification, Travel, Immigration, Employment.
- Gemma E2B, second/third: Education, Identification, Travel, Employment, Vehicles.
- Gemma E4B, all runs: Education, Travel, Identification, Immigration, Employment, Account Access.

E2B changes topic coverage despite temperature 0 and a fixed seed; deterministic settings did not guarantee identical plans on this runtime. E4B gives a consistent alternative structure, but this experiment does not establish that Immigration/Account Access is more accurate than Apple's Legal/Vehicle. The personal collection has no independently hand-labeled category taxonomy, and multiple useful organizations are possible. Folder naming, schema success and coverage are distinct properties.

Gemma load durations were recorded separately in warmups.json (approximately six seconds for each model's neutral warmup). Apple model residency is OS-managed and was not cold-start controlled. Warmups and normal prefix caching can affect timings; three runs on two collections are observations, not statistically strong performance claims.

## Decision

The installed Apple planner remains fastest in this test and gives stable specific topics. Both Gemma variants are viable category-planning candidates, with no demonstrated quality advantage here. E2B is closer in latency but varied; E4B was consistent and slower. This does not justify changing the app provider yet. A provider change would need a full pipeline evaluation, including reading, grounded evidence, folder matching and file-safety behavior.

All 25 source-file hashes were unchanged at completion. No Gemma model remained loaded. The app's current category draft, preferences and provider were untouched.

## Evidence and rerun

`results.json` contains timings, topic names, input hashes and Ollama token/load metrics. Controlled outputs include definitions for inspection. Personal document descriptions and generated private rule text are intentionally excluded. `provenance.json` records local model digests, hardware, OS, settings, helper/validator hashes and the synthetic inputs. `instructions.txt` records the prompt and retry suffix. `summary.json` records aggregate results and source-preservation checks.

Run from the Tuck repository with the same current helper/cache identity and installed local models:

```sh
swiftc -parse-as-library src-tauri/swift/TuckRoleValidation.swift evals/reports/2026-10-01/gemma-category-planning/Validate.swift -o /private/tmp/tuck-category-validator
python3 evals/reports/2026-10-01/gemma-category-planning/run.py --output /private/tmp/tuck-gemma-categories-new-run --personal-root '/Users/mohomedhaffaz/Downloads/Tuck Test - 2' --validator /private/tmp/tuck-category-validator
```

The output directory must be new. Omit `--personal-root` for the synthetic-only comparison. With the personal root, the runner deliberately expects the frozen 22-description cache and verifies source/provider identities; changes require a new benchmark definition rather than silently changing the comparison set.

## Handoff context

Production is still local Apple `serial-roles-v5-simple`. Current category generation rejects generic catch-all names, uses one to six topic-based categories without forcing three, and plans up to 32 descriptions / 6,000 characters together. Larger batches have bounded reduction. Language preflight/download suggestions are removed. There are existing uncommitted implementation changes on main and no remote: preserve them. This task adds evaluation artifacts only. See docs/ai-architecture.md and docs/category-suggestions.md for the active workflow; this report is the latest Gemma category-planning result.
