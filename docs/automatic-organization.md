# Automatic organization and the document map

## Flow

```text
Choose folder
    |
Read every supported document (bounded text/OCR)
    |
Session map: documents -> current folders; identical hash -> duplicate link
    |
Choose existing / suggested / custom folder rules
    |
Start Tuck [+ optionally watch]
    |
Apple evidence extraction -> classification -> conditional checker
    |
    +-- clear valid match -> verify source/rules/path -> organize
    +-- uncertainty / duplicate / error -> leave in place for input
```

No model executes file operations. Rust owns filesystem actions. Stop Tuck and configuration changes revoke automatic filing; the same mutex protects authorization and transfers. An already completed filesystem operation is not undone. Clearing a session invalidates in-flight results. Application restart starts with automation off.

## Map scope

All supported files are enumerated before reading begins. Mapping reads each file, checks its hash before/after, records its current folder and exact duplicates, and reports unreadable files. Empty existing folders are offered too, up to supported destination depth. The map is deliberately a small relationship structure, with no embeddings, graph database or invented semantic links. Source-folder relationships may be context but never prove a correct destination.

The map caches bounded text and is reused for classification only when its hash matches. Starting validates that the collection has not changed since setup. Classification rechecks file contents, and committing rechecks them again. The displayed map is the setup snapshot; the session document list reports later organization. Large files above 100 MB appear as unreadable in the map and remain untouched. Hidden files, unsupported formats and symbolic links are excluded.

## Watching

macOS FSEvents (through the Rust `notify` backend) wakes the existing scan when the root or its subfolders change. Events are coalesced to at most one scan every three seconds while the serial worker is free. Pending writes keep the three-second settling checks active. An idle folder is reconciled once a minute to recover missed events. If native watching cannot start or reports an error, Tuck falls back to three-second polling. New or changed size/mtime signatures must remain stable for at least six seconds. Files are then read, classified and validated afresh. This settling heuristic is not proof that a producer has closed a file; hash checks at read/commit catch observed changes. A new write occurring after a completed move cannot be predicted.

The watcher records successful output signatures while holding the operation lock so Tuck's own moves are not re-enqueued. Unchanged uncertain files are not retried repeatedly. Manual moves appear as removed/new paths; no durable file identity is maintained. Errors stop automation. Changes made with deliberately preserved size and modification time may not be detected until remapping.

Limits apply to a session: 2,000 documents, 10,000 scanned entries and 32 traversal levels. A full session must be stopped and cleared before continuing. Closing Tuck stops watching. Watch selection and running authorization are not persisted.

## Accuracy

A clear model result is not a statistical confidence score. Existing grounded extraction, quote checks, destination restrictions and conditional second-pass checks remain. This change removes routine per-file approval, so plausible model mistakes can now result in a move. The map itself does not improve semantic accuracy. Broader Apple model quality results and known limitations remain in `ai-accuracy.md` and `evals/`.

## Verification on 2026-09-28

The first live Apple run used the existing seven-folder default rules and a synthetic bank statement. Apple refused folder selection; Tuck correctly left the statement for input. A focused two-folder run (Finances/Work, preserve filenames) then filed the synthetic statement automatically, verified its unchanged hash, kept an unreadable file in place, and confirmed that watching ignored the output. This is an integration smoke test, not a comparison proving improved accuracy. No personal documents were moved during verification.
