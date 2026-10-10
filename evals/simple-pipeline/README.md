# Single-decision filing experiment

This experiment supplies the image, text and folder rules to one model decision. It separates naming uncertainty from folder choice and replaces the second model checker with deterministic validation. **None of the three tested variants is enabled in Tuck:** all regressed compared with filing-v5. [Results](../reports/2026-09-28/simple-pipeline/README.md).

- `comparison.json`: frozen 30-case synthetic regression set.
- `TuckSimple.swift`: v3, the final smaller response format.
- `TuckSimple-v1.swift.txt` / `TuckSimple-v2.swift.txt`: rejected candidate snapshots.
- `Tests.swift`: deterministic validation checks (v2/v3).
- `build.py`: compile an isolated helper; production Swift and the installed app are untouched.
- `compare.py`: distinguish correct filing, wrong destinations, unnecessary manual fallbacks, naming checks and refusals.

## Reproduce

Run from the repository root on a Mac with Apple Intelligence and the matching Swift SDK. First build Tuck's helper resources with `cargo test --manifest-path src-tauri/Cargo.toml --lib`. The candidate builder reuses the production input/operation dispatch before the `var stage = "input"` marker and the shared model support file; it targets the filing-v5/v6 input and dispatch layout. Model or source updates may change results.

```sh
python3 evals/simple-pipeline/build.py /private/tmp/tuck-simple --variant v3
TUCK_EVAL_READER=/private/tmp/tuck-simple/tuck-reader \
TUCK_EVAL_FIXTURES="$PWD/evals/simple-pipeline/comparison.json" \
TUCK_EVAL_REPORT=/private/tmp/tuck-simple-results.json \
TUCK_EVAL_LABEL=single-decision-v3 \
cargo test --manifest-path src-tauri/Cargo.toml live_apple_accuracy_eval -- --ignored --nocapture

swiftc -parse-as-library /private/tmp/tuck-simple/Input.swift \
src-tauri/swift/TuckModelSupport.swift evals/simple-pipeline/TuckSimple.swift \
evals/simple-pipeline/Tests.swift -o /private/tmp/tuck-simple-checks
/private/tmp/tuck-simple-checks

python3 evals/simple-pipeline/compare.py \
evals/reports/2026-09-28/simple-pipeline/baseline.json \
/private/tmp/tuck-simple-results.json
```

For the baseline use the production reader/helper pair with the same fixture set and an explicit report label. Use `--variant v1` or `v2` to reproduce earlier candidates; keep each variant in a separate output directory. Run live model evaluations sequentially. The harness does not move files and a passing harness process does not mean its classification decisions are correct.
