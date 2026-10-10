# Category specificity — 2026-10-01

## Reported issue

The installed preview proposed Documents, Travel and Identification. Documents included personal/official records, IDs, permits and certificates, overlapping the named subjects. The planner asked for broad categories and its format retry required exactly three, encouraging excessive merging. Fixed-size batches of 12 also introduced unnecessary repeated summarization for a 25-file collection.

## Change

- Ask for distinct subjects/purposes supported by the collection; preserve unrelated topics.
- Require exactly folder/instruction string fields in each category entry.
- Reject known generic catch-all names in Swift and again at the provider-neutral Rust boundary. Qualified subjects such as Medical Records remain valid. Existing and user-created categories are unaffected.
- Retry once without a forced category count. A repeated invalid result remains a visible error.
- Plan up to 32 descriptions / 6,000 description characters together. Larger collections retain bounded reduction, with a non-progress guard. No input descriptions are silently dropped.

## Evaluation

The first prompt revision identified useful topics but returned extra subject fields or instruction arrays. Explicit field/type instructions fixed the controlled case. On the real collection, the initial 12-item reduction still collapsed the draft into Education only. That candidate was not installed. Keeping this size collection together produced the final useful range of topics.

Controlled: ten synthetic descriptions from five known subjects produced Identification, Travel, Employment, Insurance and Education, with matching concise definitions. See controlled.json. This is a semantic fixture check, not merely schema success.

Read-only personal collection: 22 described, 3 skipped, 6 suggested categories in 73.93 seconds: Education, Travel, Identification, Employment, Legal, Vehicle. Every source hash stayed unchanged. Personal descriptions/raw model inputs are not included here. We inspected topic specificity and retained Travel/Identification; this does not establish perfect per-document coverage or subsequent filing accuracy. Three skipped documents do not contribute to the draft.

81 Rust tests passed, including topic-name rejection, qualified names, preservation of user-created names, all-document batching, count/text limits and order preservation. Swift role contracts passed, including a retry after a generic draft. Clippy with warnings denied and the Tauri release build passed. No UI behavior changed; the prior UI regression suite remains recorded separately.

See [category suggestions](../../../../docs/category-suggestions.md) for behavior and remaining lossy-reduction limits.

The signed bundle was installed at `/Applications/Tuck.app`, with backup `/private/tmp/tuck-before-category-update-s_vtqs7v/Tuck.app`. Installed file hashes matched the built bundle. Category suggestions were regenerated in the native app for inspection; applying them still requires the user’s Use these categories action.
