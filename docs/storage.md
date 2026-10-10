# Storage: file-based MVP

Status: implemented. SQLite is not used. Automatic filing and optional watching run only during an explicitly started session.

## What is saved

| Location                                   | Contents                                                                                             |
| ------------------------------------------ | ---------------------------------------------------------------------------------------------------- |
| Destination root `.tuck/settings.json`     | Version, rename preference, maximum folder depth, local AI switch, `aiModel` choice, document-tools switch.  |
| Root `.tuck/cache/<content-hash>.json` | Rebuildable document descriptions, short evidence quotes/page references, uncertainty, content/extraction hashes, provider identity and schema version. |
| Root `.tuck/instructions.md`               | General organizing and naming prompt.                                                                |
| Destination folder `.tuck/instructions.md` | Folder prompt, explicitly saved examples, enabled switch and priority. Parent prompts are inherited. |
| Application Support `preferences.json`     | Only the last destination root path.                                                                 |
| Webview local storage                      | Whether the introductory guide has been seen.                                                        |

Example `settings.json` (all fields are required; unknown fields and unsupported versions are rejected):

```json
{
  "version": 1,
  "rename": true,
  "maxDepth": 2,
  "useAi": true,
  "useAgent": true
}
```

No full OCR transcript, document inventory, analysis queue, filing proposals, or operation history is persisted. Short evidence quotes are stored with successful document descriptions in the cache. Explicitly remembered filing examples are saved as editable text in the chosen folder’s rule file; automatic correction history is not stored. The batch is in memory, with statuses queued, reading, review, organized, skipped, and failed. Dropping multiple files creates one session batch. Folder selection builds an in-memory map of all supported documents, source folders and identical copies. The serial worker handles the mapped collection, dropped documents and optionally stable new/changed files. Watching and Start Tuck authorization are session-only and are never resumed on launch.

## Automatic filing and fallback

Mapping records text excerpts, source hashes, read failures and duplicate links in memory without moving documents. Start Tuck authorizes automatic filing using the selected rules. Clear, validated matches move automatically; uncertain and duplicate decisions need input. The setup map is a snapshot, not a continuously maintained index.

Before organizing, Rust checks the active Start Tuck authorization (or an explicit fallback approval), reloads settings/instructions, validates the source path and hash, and validates the destination. A changed configuration stops automatic filing and watching and requires analysis again. A changed source fails without moving it. A newly occupied destination gets a new preview requiring a fresh approval. File operations use exclusive destinations and never overwrite an existing document.

External files are copied; files already under the destination root are moved. Copies use a synced temporary file followed by exclusive persistence. Moves use an exclusive hard link followed by unlinking the original. No database or history journal spans those steps. A crash can leave a duplicate if it occurs between linking and unlinking; restart does not replay or delete anything. Files may already be organized even if the app closed before showing completion. Inspect the filesystem after an interruption.

**Organize selected** processes approvals in order, stopping on the first error. Earlier successes remain organized and visible in the session. This is not an all-or-nothing batch transaction. There is no persistent undo.

Clearing the batch invalidates any in-flight analysis result. Analysis can finish in the background, but cannot move documents or repopulate the cleared batch. Closing the app stops watching and discards all session work; files not organized remain where they were. Choosing another destination requires clearing the batch first.

## Editing preferences

The hidden files are authoritative. Refreshes read settings and instructions; when the editor has no unsaved draft, changes appear automatically. Unsaved drafts are preserved; stale saves are rejected. **Reload from disk** explicitly replaces the draft with the latest files. Malformed, missing, oversized, or symlinked configuration blocks analysis/organization without replacing the file.

Settings use synced temporary-file replacement. Instruction edits retain the existing short-lived `.tuck/pending-edit.json` rollback journal because one save can modify multiple prompts. On startup it recovers compatible incomplete edits, preserving conflicting manual edits for user inspection. It is not a document-operation journal. Instructions and settings are separate saves, not one filesystem-wide transaction; if a later settings write fails, earlier prompt edits may already be saved. Reload before retrying.

## Existing installations

The uncommitted SQLite implementation was discarded. The MVP does not open, modify, or delete any existing `tuck.db`, its sidecars, backups, legacy `state.json`, or `folders.json`.

If the new root-pointer file does not exist, startup reads the legacy JSON folder selection and preferences only. Existing Markdown instructions take precedence; missing configuration can be initialized using those preferences. Legacy watcher flags, queued paths, activity, and correction memory are ignored. Settings changed exclusively in an old SQLite build are not imported; review Preferences. After the root-pointer file exists, normal startups require the `.tuck` files rather than reverting to stale legacy preferences.

Back up the destination root, including hidden `.tuck` folders. The root pointer can be recreated by choosing the folder again. There is no database backup or restore UI. Plaintext settings have ordinary filesystem protection, not added encryption.

## Future triggers for a database

Reconsider a database if the product adds persistent queues, watching across app restarts, durable undo, search indexing, or cross-session history. Filesystem identity and manual-move reconciliation belong with that future work. None is needed for a temporary document map and app-session watcher.

## Verification

Rust tests cover mapping without moves, explicit Start Tuck, automatic clear matches, stop/rule-change behavior, watcher settling and own-move suppression, fallback approvals, manual filing when uncertain, external-copy preservation, collision reapproval, stale prompt/settings revisions, changed sources, disabled destinations, skipped/cleared batches, symlink rejection, restart behavior, and untouched legacy JSON/database bytes. UI tests cover the corresponding review interactions, configuration editing, onboarding, and fixed header/sidebar scrolling.

## macOS folder access

The app-data directory also contains `folder-access.json`: a security-scoped bookmark for the selected root, used to restore access across launches. It stores an opaque access reference and path, not document contents. See [Native macOS integration](native-macos.md).

## Document understanding cache

Each supported document is described before grouping or matching it to a folder. The shared stage writes one small JSON file under the selected root's `.tuck/cache/`, keyed by SHA-256 of the document bytes. Document identity is assigned by code. No filenames, source paths, folder choices or user rules are needed in this record. Identical copies and renamed documents can share a description. Matching uses the current enabled folder names; separate verification still uses the current source location and rules.

Cache reads require matching content, prepared-text and helper-binary hashes plus a supported schema version. Source quotes must still occur in the current extracted/prepared text. Changed files, changed extraction/translation output, helper updates, malformed records and missing entries cause regeneration. Cache entries are not trusted as file-operation authorization. Descriptions remain fallible; routing checks use source evidence. Model-asset changes without a helper/extraction change do not automatically invalidate the cache yet; delete the cache to force regeneration after such a change.

Only grounded, unambiguous descriptions are persisted. Descriptions requiring evidence outside the initial extraction remain session-only and may need extra reads again next time. The model may request at most three bounded page/chunk reads or OCR retries. Errors and uncertain understanding stay available for manual filing. Cache failure does not block in-memory analysis. Unsafe cache paths/links are ignored, not followed.

You can delete `.tuck/cache/` while Tuck is stopped; it rebuilds on demand. Settings, rules and documents remain intact. Removed/changed documents may leave old cache files until you clear it. There is no automatic eviction or cache-management UI yet.

The cache is plaintext and may include personal facts and short document quotes. New cache directories are owner-only and files use private temporary-file permissions. Tuck adds no encryption. Because it lives inside the selected folder, any external backup or cloud-sync software covering that folder may copy it too; local inference does not prevent that external synchronization.

The cache field `helperHash` now holds the provider identity (selected provider/role version, local model digest for Ollama, and helper binary hash). Changing providers or helper prompts invalidates earlier descriptions. The legacy field name is retained for format compatibility; entries from older identities regenerate automatically.


The optional version-1 settings field `aiModel` accepts `apple`, `gemmaE2b`, or `gemmaE4b`. Existing files without it load as Apple. Unknown new selections are rejected; obsolete legacy `provider`/`model` fields in old app state remain ignored. The setting participates in the preferences revision, so model switches reject earlier filing/category proposals and stop automatic processing. Rebuildable descriptions are regenerated for the selected model.

## Model connection credentials

Connection type, base URL, model ID, folder nesting and optional category instructions are stored with the folder’s settings and participate in revision checks. API keys are held separately in macOS Keychain, scoped to the connection type and API endpoint. They are never written into the document folder or its configuration. Blank key fields preserve the saved credential. Changing the endpoint requires a key for the new endpoint.

## Complete generated scopes and two-level migration

Folder rules may include `document-types: ["Academic transcript", "Degree certificate"]` and `scope-hash: <sha256>` in Markdown front matter. The type array is complete, bounded to 128 entries of at most 300 characters each, and is separate from the concise folder description. The hash covers the trimmed description. A manual description edit makes the old generated scope inactive; saving an edited description replaces it. The editor displays every included type.

Version-1 settings with a former depth of three load with a maximum of two. Existing third-level instruction files remain part of revision checks and are preserved byte-for-byte during saves, but are excluded from active destinations. Recovery can still read legacy three-level journal entries. Documents are never moved as part of this migration.
