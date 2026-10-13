# Rules and folder nesting fixtures

The runner and Rust harness for this evaluation were removed with the old describe/match/verify/name pipeline they exercised. The synthetic fixtures remain as a corpus: `evals/cards/from_fixtures.py` converts the `diverse` and `diverse-heldout` suites into the card evaluation layout. `Cargo.toml` is still copied by `evals/structure/run.py`.

Earlier results are kept in the [4 October evaluation report](../reports/2026-10-04/rules-nesting/README.md).
