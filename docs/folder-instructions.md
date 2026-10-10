# Folder-owned instructions

Status: implemented. The Markdown files are authoritative; Tuck reads and edits the same files. Preferences live in `.tuck/settings.json`; there is no database. Prompts stay beside the documents so they are portable and editable.

## Structure

```text
Documents/                         ← the destination root
├── .tuck/
│   ├── settings.json              ← small app preferences
│   └── instructions.md            ← general organization and naming preferences
├── Migration/
│   ├── .tuck/instructions.md       ← migration documents and shared preferences
│   └── AGENCY/
│       └── .tuck/instructions.md   ← specific application bundle instructions
├── Finances/
│   └── .tuck/instructions.md
└── Home/
    └── .tuck/instructions.md
```

Root instructions are plain Markdown, for example:

```markdown
Use clear English names describing the actual document.
Preserve copies that keep an application bundle complete.
Never invent a date or describe a residence permit as a visa.
```

A destination file has optional, deliberately small front matter followed by a plain-language prompt:

```markdown
---
enabled: true
priority: 0
---

All documents from AGENCY or its application folders belong here,
including employment certificates and other supporting documents.
Name each attachment by its actual document type.
```

`enabled` defaults to `true`; lower `priority` comes first. Files without priority follow numbered entries, ordered by relative path. Only these two front-matter keys are accepted. They are not full YAML: use `true`/`false` and a non-negative whole number. Unknown or duplicate fields are errors. Root instructions have no front matter. An enabled destination needs a nonempty prompt.

## Inheritance and routing

1. Root preferences apply everywhere.
2. A destination inherits each ancestor's instructions; the more specific prompt refines the parent. Siblings do not contribute naming preferences.
3. Disabling a parent disables its entire branch, while retaining each child's own switch and text. Re-enabling the parent restores the enabled children.
4. An ordinary folder without an instruction file is not an automatic destination. It can be a category containing configured children; those children inherit from the nearest configured ancestors.
5. The model checks explicit source-folder associations first, so a AGENCY supporting attachment can remain in its application bundle. The cited name must appear in both the source folders and the selected destination prompt.
6. Otherwise Apple makes one selection from enabled destinations, using their inherited prompts and document contents/filename. This keeps folder selection bounded without separate model calls for every level. After selection, naming uses root → parent → destination instructions.

These are model instructions, not guaranteed semantic or factual enforcement. Rust enforces enabled destinations, depth, tool budgets, and file-operation constraints. Child prompts cannot override those application limits. After Start Tuck, clear validated matches file automatically; uncertainty stays for input. Uncertain routing leaves the destination blank for the user to choose; uncertain naming keeps the original filename.

Keep specific destinations before broader ones in the editor. The maximum is two destination levels and 40 configured destinations, including disabled entries. Each instruction file is limited to 64 KiB; Apple's context budget is smaller, so keep prompts short. Deeper files are not discovered. A depth-one preference rejects a configured two-level destination until it is removed or depth two is restored.

## Editing in Tuck or a text editor

- **Preferences → Instructions for every document** edits the root file.
- **Folder instructions** edits destination prompts, their order, and switches. Saving creates missing destination directories and writes their instruction files.
- Tuck checks disk instructions during refresh and before document operations. Manual changes appear automatically when the editor has no unsaved changes.
- Unsaved edits stay in the editor. A changed disk revision blocks a stale save. **Reload from disk** discards the unsaved draft and adopts the latest loaded files.
- If instructions change during or after analysis, Tuck requires **Analyze again** before approval. The review item keeps its instruction/settings revision in memory.
- An unreadable, missing root, oversized, malformed, or symlinked instruction file blocks analysis and organization and displays an error. Fix/restore the file and reload before retrying. Normal startup does not recreate missing configuration.
- Removing a destination deletes its instruction file only. Existing documents and the directory remain. Removing a parent instruction file does not remove its child files; those children then inherit from the root.
- Editing instructions never automatically reprocesses previously handled documents. Add the files to a new review batch or use **Analyze again**.

All `.tuck` contents are excluded from direct document imports. Symlink instruction files/directories are rejected. The app does not follow symlink folders to discover instructions.

## Migration and recovery

The file-based MVP imports only the previous root and preferences from legacy JSON when no new root pointer exists. It leaves the legacy JSON, database, and backups unchanged. Existing root instructions take precedence; when creating new configuration, individual pre-existing destination prompts are also preserved. Old queue, history, watcher flags, and correction memory are not resumed. See [storage](storage.md) for details.

Editor saves use atomic file replacement and a temporary root `.tuck/pending-edit.json` journal. On restart an incomplete save is rolled back only when each file still matches its recorded before/after bytes. A conflicting manual edit is preserved and organization remains blocked. Keep the journal and files for recovery; do not delete it blindly. Empty directories created during a failed save may remain. This is crash recovery, not a backup or a filesystem-wide transaction.

Back up `.tuck` alongside the documents. Tuck does not provide Google Drive synchronization yet; any external sync tool must include hidden files. The current review batch remains in memory for this session.

See [file-based storage](storage.md) for persistence and recovery, and [AI accuracy](ai-accuracy.md) for model limitations and evaluation.

## Short descriptions and AI drafts

A short sentence describing what belongs in the folder is enough. For a blank rule, enter a folder name and choose **Suggest description**. Apple uses that name and the current root/ancestor rules to write an editable draft. It does not read documents for this action. The draft stays in the UI until **Save rules**; generation does not create directories or change `.tuck` files. Existing descriptions are preserved. If the user edits the draft while generation is running, the stale result is discarded.

A parent describes the broad category; a child describes its additional requirement. For example, Migration can contain immigration documents, while Migration/AGENCY requires an explicit AGENCY association. A child's exclusions apply only to that child. Avoid identical descriptions for sibling folders: Tuck asks you to distinguish them before making a destination suggestion.

## Remembering a filing example

After correcting a document and choosing **Update preview**, choose **Remember this example**. The same action remains available after organizing a corrected item in this session. Edit the short description and optionally add a contrasting example of what does not belong here. **Add to rule draft** opens Folder rules; **Save rules** persists it. Canceling or closing before saving leaves the on-disk rules unchanged. This is explicit opt-in, not automatic learning from every move.

Only the text shown in the example editor is saved. The proposed filename pre-fills the description, so remove names or details you do not want in the rule. Tuck does not save a document copy, OCR text, path, transcript or inventory for this feature. Each field is limited to 240 characters; the UI allows at most three generated example blocks per folder and rejects exact duplicate blocks. Hand-edited rules still use the normal file and context limits.

Examples are ordinary editable Markdown in the existing folder file:

```markdown
## Filing example

Belongs here: A bank statement included in my AGENCY application.
Does not belong here: An unrelated monthly bank statement.
```

A negative example applies only to its own destination. It does not prohibit other folders from accepting that document. Examples describe filing preferences, never facts about the current document. Remove or edit the block in Folder rules or your text editor. Existing revision checks require fresh analysis after any saved rule change. The temporary review batch and correction markers are still discarded when Tuck closes.

## Checking duplicate rules

**Check rules** compares enabled destination descriptions within the same parent branch, ignoring differences in capitalization and whitespace. Disabled folders and descendants of disabled parents are excluded. Identical descriptions in different branches can inherit different meanings and are not flagged. This check works with Apple Intelligence disabled and can check an unsaved draft without saving it.

Each finding lets you write a distinction and choose **Add clarification to draft**. That appends editable text to one folder; **Save rules** persists it. Results become stale after draft or on-disk settings changes. A changed root, stale revision, or active processing blocks the check. No model or document reading is involved.

This check finds duplicate descriptions only. Different wording can still overlap. Broader AI auditing was implemented and tested, then removed from production because Apple's model repeatedly missed real overlaps and invented false ones. Its results are retained in the [learning comparison](../evals/reports/2026-09-28/learning/README.md).
