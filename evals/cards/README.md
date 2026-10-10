# Card pipeline evaluation

Compares folder suggestions and filing on synthetic corpora through the public entry points
(`categories::suggest_map`, `classifier::classify_with_tools`), so the same harness runs on
any commit. Never moves files.

```sh
python3 evals/cards/make_corpus.py /private/tmp/tuck-corpus        # 35 mixed docs: long, scans, sv/de
python3 evals/cards/from_fixtures.py /private/tmp/tuck-heldout     # 29 docs from rules-nesting "diverse"
TUCK_CORPUS=/private/tmp/tuck-corpus TUCK_CORPUS_REPORT=/private/tmp/report.json TUCK_CORPUS_NESTING=2 \
  cargo test --manifest-path src-tauri/Cargo.toml live_corpus_suggest_and_file -- --ignored --nocapture
```

Requires Apple Intelligence. `pairPrecision`/`pairRecall` compare top-level suggested folders with
gold subjects over document pairs; documents with gold "Needs Review" are excluded from pairs.

## Results, 10 October 2026 ([reports](../reports/2026-10-10/card-pipeline/))

Old = commit 49c1291 (describe → group → match → check). New = card clustering.

| Corpus | Measure | Old | New |
| --- | --- | --- | --- |
| Mixed (35) | Files with a suggested folder | 29 | 35 |
| | Filing correct / wrong / review | 29 / 0 / 6 | 35 / 0 / 0 |
| | Median filing time | 3.1 s | 0.8 s |
| Held-out (29) | Clear docs filed correctly | 25/25 | 25/25 |
| | Unclear or mixed docs held for review | 4/4 | 1/4 |
| | Pair precision / recall | 0.78 / 0.90 | 0.97 / 0.83 |
| | Median filing time | 3.2 s | 0.6 s |

The old pipeline sent every document over 5,000 characters to review. Two changes were made after
inspecting the mixed corpus (no leftover attachment; "Identity & Immigration" as a subject example),
so its numbers are not held out; the held-out corpus was run once after those changes. The new
pipeline files most deliberately unclear or two-record documents instead of holding them; this is the
chosen "place more, review less" trade-off. Small synthetic sets; not a real-world accuracy estimate.
