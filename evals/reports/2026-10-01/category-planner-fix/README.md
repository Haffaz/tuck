# Suggest categories invalid response — 2026-10-01

## Reproduction

The installed app and the read-only native diagnostic both reproduced `invalid_response` in a selected 25-document local folder. The planner returned a JSON object whose array contained another `rules` key between entries. JSON parsing failed at character 152 (expected comma delimiter). The failure was malformed model output, not a reported Apple refusal. No source documents were moved.

A separate reduction input returned eleven categories, exceeding the required one-to-six range. A guided schema trial on the exact malformed-output request returned a model refusal; it was not adopted.

## Fix

- Send only description strings to the Apple planner, leaving document IDs and prior group labels in the coordinator.
- Show an explicit two-entry example of a single `rules` array.
- Validate the whole response, fields, count, names and instructions in Swift; retain Rust path/rule validation.
- Retry a malformed/invalid draft once with the same descriptions and a request for three broader categories. Keep the original helper timeout.
- Never truncate extra categories, extract JSON from refusal prose, retry service errors/refusals, or apply a partial failed proposal.
- Keep the previously approved model policy; no new guardrail changes.

## Verification

Swift contracts cover the malformed array regression, over-limit drafts, retry exhaustion, invalid paths, duplicate names and no retry for refusal/guardrail/timeout/context errors. All checks passed. Eighty Rust regression tests, Clippy and the release/frontend build passed. The affected-folder diagnostic returned four categories from nine confidently described documents, skipped sixteen and verified all source hashes unchanged. This is a successful partial preview, not a claim of complete document understanding.

Personal document descriptions and raw model responses are not stored in this repository. The reusable diagnostic is `evaluation::tests::live_selected_category_preview`, opt-in via explicit `TUCK_SELECTED_ROOT` and `TUCK_EVAL_READER`; it keeps diagnostic description caches in a temporary directory and checks source hashes even when planning fails.

## Installed app confirmation

The release bundle was signed, installed and reopened. Suggest categories completed in the affected folder and displayed four editable suggestions. The UI explicitly reported nine contributing documents and sixteen files needing a closer look. The draft was left open without accepting categories or starting organization. All 25 source paths and hashes matched the pre-run snapshot. Temporary diagnostic copies of descriptions/raw responses were removed.
