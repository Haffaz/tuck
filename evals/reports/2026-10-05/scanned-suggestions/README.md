# Scan suggestion fixes — 5 October 2026

## Result

The updated app is built and installed. The selected 25-document folder improved from 4 contributors in the original app draft to 15 in both the private replay and a fresh draft generated in the installed app. The refreshed draft contains ten proposed categories and remains unapproved for the user to review. The replay had no evidence-format errors. Coverage measures participation, not classification accuracy; ten files still required review. No source documents were moved or renamed, and all 25 original hashes were verified.

## Changes

- Keep valid primary evidence when optional detail references are invalid, repeated or excessive. Retry malformed responses once using a constrained primary-evidence schema.
- Identify a record’s purpose separately from choosing its evidence. This corrected a continuation form that had been named after the law in its footer.
- Classify each record independently before reconciling equivalent subject labels.
- Show partial coverage and per-file review reasons before accepting suggestions.
- Enforce clear, unconditional type-to-path rules against conflicting destinations. Requested details still require source support and paths remain bounded by the nesting ceiling.

## Final validation

| Check                                                       | Result                                      |
| ----------------------------------------------------------- | ------------------------------------------- |
| Core: six combinations across 1–3 levels                    | 48/48 clear filings, 12/12 review decisions |
| Core proposal subjects                                      | 48/48                                       |
| Explicit requested grouping                                 | 8/8                                         |
| Filing into generated folders                               | 42/42                                       |
| Held-out: six combinations, varied document types           | 72/72 clear filings, 12/12 review decisions |
| Held-out proposal subjects                                  | 69/72 (95.8%)                               |
| Held-out requested grouping details                         | 24/24                                       |
| Incorrect automatic filing in final suites and safety edges | 0                                           |
| Depth violations                                            | 0                                           |
| Rust / UI / provider contracts                              | 94 / 37 / 58 passed                         |
| Swift assertions, Clippy, formatting, macOS bundle          | Passed                                      |

All final acceptance targets passed. A separate shared-name scope edge remained a safe review rather than the expected Health filing; the other three scope edges passed. These focused suites do not repeat the entire earlier 30-run matrix or establish universal accuracy. Twelve Rust tests were intentionally ignored; the real-model checks above ran separately.

## Failures retained

The first candidate passed the main filing matrix but missed one requested subgroup and incorrectly selected a different branch when the correct branch was disabled. The original scorer omitted that disabled-branch decision from its safety total. The corrected scorer now rejects it. Preserving the routing sentence in the model prompt alone did not fix the failure; code-level type-to-path checks did. Both failures passed in the final replay. The interrupted intermediate run is retained as incomplete, not as a passing result.

## Evidence and privacy

`core/` and `heldout/` contain frozen synthetic fixtures, results and source provenance. `history/` retains earlier results, including failures. `selected-folder-summary.json` contains counts only. Private document text, names, extracted descriptions and raw model responses remain outside this repository. The installed app’s signature and complete file manifest were verified; the previous app was preserved in a temporary backup.
