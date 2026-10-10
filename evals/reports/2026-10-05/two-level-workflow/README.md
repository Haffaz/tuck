# Two-level folders and workflow improvements

5 October 2026. Local Apple Intelligence evaluation using synthetic documents. No personal document or installed-app preference was changed.

## Final result

All acceptance checks passed. Across the core pilot, diverse suite, and generated-folder controls, all **124 filing decisions** were correct, including holding review cases. There were **zero wrong automatic filings**. The separate broader proposal score was **46/48 (95.8%)**; its two errors are retained below. The release app is rebuilt and ready to install. Changes are uncommitted.

## Changes

- New folder suggestions and active destinations allow one or two levels. Existing third-level rule files are preserved on disk, excluded from active destinations, and surfaced in Settings. Saving unrelated preferences preserves their bytes.
- Negative and unrelated grouping clauses no longer reintroduce subfolders during deterministic layout cleanup.
- Generated rules retain every observed document type in structured metadata, separate from the short description. A hash ties that metadata to the description. Editing the rule clears its old generated scope; duplicate-rule checks include the saved types.
- Planning carries established root subjects between batches of up to four documents. Layout decisions use document counts across all batches.
- Matching compares complete scopes in batches of at most eight candidates and 5,000 serialized rule bytes. Large type lists use unique choice IDs and groups of up to six types; nothing is silently clipped.
- Verification checks the selected type once when the rule restricts types, then compares bounded competing destinations. Broad subject labels use subject comparison instead of a narrow type gate. A single-folder verification prompt was too permissive in live testing. Selection and verification share one implementation with distinct prompts.
- Multipage filing reuses the planner's separate page checks. A first-page summary cannot by itself clear a file containing unrelated records.
- Naming receives source quotes. Rust rejects names containing words absent from the verified facts, English description, and an explicit filename prefix. Rejected naming keeps the original filename.
- Each helper operation permits at most 32 generations, including retries. The existing 90-second document deadline still applies.

## Acceptance criteria

Frozen fixtures and scorers are retained with each run. Targets: at least 95% recall for clear documents and proposals; zero wrong automatic filings; all unclear or mixed records held for review; no depth violations; no unsupported named subgroup or filename-prefix failures. Guided grouping and missing-field fallbacks must meet each suite's frozen checks.

The core pilot covers one and two levels with all three custom rule scopes off or on. It is four endpoint combinations, not the full 20-run matrix. A separate existing diverse-heldout suite covers project records, supplier invoices, clinical records, education, warranties, and travel. Additional controls file into generated rules.

## Retained failed iterations

| Iteration | Core clear documents correct | Review cases held | Wrong automatic filings | Outcome                                                                                                                               |
| --------- | ---------------------------: | ----------------: | ----------------------: | ------------------------------------------------------------------------------------------------------------------------------------- |
| 1         |                   Incomplete |        Incomplete |              Not scored | Short verification prompt rejected correct folders; an old three-level evaluation edge then stopped the runner. Raw results retained. |
| 2         |                        30/32 |               4/8 |                       4 | Constrained single-folder verification restored recall but accepted mixed documents.                                                  |
| 3         |                        29/32 |               4/8 |                       5 | Full source and extra prompt wording did not fix the mixed-document failure.                                                          |
| 4         |                        31/32 |               8/8 |                       1 | Shared page checks fixed mixed documents; a related-type confusion remained.                                                          |

A targeted probe showed Apple could return `CLEAR` while explaining that a file contained two distinct substantive records. Another probe showed a one-folder verification could approve a payslip against an employment-contract rule. The final implementation uses separate page checks and competing folder choices. A final targeted probe accepted the correct income folder and rejected the employment folder.

The first broader-suite run was stopped after identifying valid documents rejected by a narrow type gate on label-only subject rules. Its partial results remain in `diverse-iteration-6`. The next run exempts broad labels from that gate while keeping comparative verification and all explicit saved type restrictions. A protocol regression verifies both sides of that distinction. One project record still failed comparison in the four-candidate tournament; `diverse-iteration-7` retains that partial run. Increasing the candidate count to eight while retaining the same 5,000-byte limit keeps ordinary six-folder subject trees together. Targeted probes then filed the project report correctly with and without custom naming instructions. The complete run still found a hotel reservation assigned to Health. Its 46/48 clear filings and 46/48 clear proposals met recall thresholds, but one wrong automatic filing failed safety; see `diverse-iteration-8`. The final correction restores readable folder names in the constrained schema instead of opaque ordinal IDs. Repeated type variants receive collision-free suffixes. Targeted project, hotel, and income probes then chose and verified their correct folders; the deliberately wrong income-to-employment choice was rejected.

## Verification

The final core pilot meets every frozen acceptance check: 32/32 clear documents correctly filed, 8/8 review cases held, 32/32 clear proposal assignments correct, and zero wrong automatic filings or depth violations. All 28 generated-folder controls also match their proposed destinations. The final diverse run also meets every acceptance check: 48/48 clear documents correctly filed, 8/8 review cases held, 46/48 clear proposal subjects correct (95.8%), all 12 requested grouping details correct, and zero wrong automatic filings or depth violations. These percentages describe the frozen synthetic cases only. The two remaining proposal errors are the same supplier-invoice fixture suggested under Work with default creation preferences at each depth. Explicit creation preferences put it under Invoices, and filing into saved rules selects Invoices correctly in all four variants. This remains a recorded proposal-quality limitation.

Machine-readable artifacts contain the exact outcomes, source/helper hashes, fixture hashes, and logs.

- Rust: 99 passed, 12 opt-in tests ignored. The updated cross-batch subject-catalog assertion also passed.
- Swift: pure response, grouping, scope, batching, and generation-budget tests passed.
- UI: 37 passed, including two-level preferences, preserving existing two-level destinations when the suggestion ceiling is lowered, and clearing generated scope after editing.
- Provider contracts: 73 passed against temporary localhost mocks for Ollama, OpenAI-compatible, and Anthropic protocols. This is protocol coverage, not live accuracy for external models.
- Structure: 16 frozen cases in both orders, complete merged type scopes, and cross-batch layout boundaries passed.
- Rust lint, production frontend build, and macOS release app build passed.
- The final release additionally charges an optional legacy image-to-text fallback against the generation budget. The evaluated Apple text workflow does not take that branch; its prompts and decisions are unchanged. That optional image fallback is not covered by these live suites.

## Limits

These are small synthetic fixtures, not a universal accuracy estimate. This run does not re-evaluate the user's scanned collection or cover every combination in the full core matrix. Live external-provider accuracy, large real folder trees, poor OCR, and long documents remain separate validation work. Complete-source extraction still has the existing 5,000-character/12-page evidence boundary. Context or work-budget exhaustion leaves documents for review.

The app bundle was built at `src-tauri/target/release/bundle/macos/Tuck.app`. The installed app and its open draft were left as they were.
