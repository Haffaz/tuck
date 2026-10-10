# Tuck

A little home for every document. A local macOS document organizer built with Tauri 2, React, TypeScript, Rust, PDFKit, Apple Vision, and Apple Intelligence.

## Organize, Folders and Settings

1. Choose one document folder. Tuck maps all supported documents in it and its subfolders before any file moves. Hidden files and symbolic links are excluded.
2. Keep the configured categories or **Suggest categories**. **Folders** provides one folder tree: select a folder to edit its rule, select the root for rules that apply everywhere, or **Add folder**. **Save rules** applies your edits. **Use existing folders** is offered when the discovered structure differs from the saved rules. Existing folder names become editable topic descriptions; placement is not treated as proof that a document belongs there. Saved rules are preserved when adopting a structure. Suggested categories replace the configured rules only after **Use these categories**; old folders and documents are not deleted.
3. Optionally select **Keep watching this folder**, then **Start Tuck**. Clear, validated matches are filed automatically. You can also drop files or use **Choose files** while Tuck is on.
4. **Needs attention** shows only uncertain decisions and failures. Completed, skipped and pending documents appear in the compact **Activity** view. Uncertain decisions, duplicates and validation failures stay in place for your input. Choose a category and **File document**, or **Leave for now**. **More options** contains renaming, retrying, rule examples and details. Filing validates the edited destination automatically; there is no separate preview step.
5. **Stop Tuck** stops automatic moves and watching. The current read may finish as a suggestion. Clear the session and map again to start fresh. Restarting the app never resumes automatic filing.

Files inside the root are moved; external files are copied with originals preserved. Source hashes, rule revisions, enabled destinations and exclusive output paths are checked before each operation. Existing documents are never overwritten. The model can still make a wrong but plausible decision; automatic filing does not imply calibrated certainty.

The session supports up to 2,000 PDF, PNG, JPEG, HEIC, TIFF, TXT or Markdown documents, each at most 100 MB. A folder exceeding the scan limits is rejected rather than partially organized. Unreadable and oversized documents are shown in the map. Reading uses bounded excerpts and optional page tools, not guaranteed exhaustive page comprehension. Analysis remains serial to avoid competing local model sessions.

The lightweight map connects documents to source folders and identical content. It is a setup snapshot, not a semantic knowledge graph or persistent search index. It stays in memory. Watching checks new/changed files every three seconds while the worker is available, waits at least six seconds of unchanged size/time, then reads them again. Own moves are recorded so they do not cause a filing loop. Rules edited on disk stop automatic filing.

Organize shows the selected folder, a compact destination summary and documents needing attention. Folders contains the folder tree and proposed structures. Settings contains the AI connection, folder nesting and visible document/folder creation instructions. Descriptive naming is the default; there is no naming-quality toggle. A saved folder can start directly; Tuck reads it first if needed. Paths, scan counts and explanations stay under Details. Success messages dismiss automatically. The unified Mac-style toolbar stays fixed while content scrolls. The interface follows system light/dark appearance and Reduce Motion. Use Command-comma for Settings, Command-1 for Organize and Command-2 for Folders. There is one root, no database, built-in cloud sync, saved operation history or durable undo. Reusable document descriptions and short evidence quotes live in `.tuck/cache/`; category matching uses those descriptions with your current rules. See [architecture diagrams](docs/architecture.md), [product experience](docs/product-experience.md), [category suggestions](docs/category-suggestions.md) and [automatic organization](docs/automatic-organization.md).

## Simple local storage

```text
Documents/
├── .tuck/
│   ├── settings.json           # small preferences
│   └── instructions.md         # root prompt
└── Migration/
    ├── .tuck/instructions.md   # migration prompt
    └── AGENCY/
        └── .tuck/instructions.md
```

The app only remembers the destination root in `~/Library/Application Support/app.tuck.desktop/preferences.json`. The document map, session and analysis results are held in memory and disappear when Tuck closes. Organized files remain in place. Clearing a session does not undo completed operations.

SQLite is not used. Existing database files, JSON state, and backups from previous builds are left untouched. On first use, the MVP can read the previous selected root and preferences from legacy JSON; it does not import queues, history, or saved corrections. Existing folder instruction files take precedence. It does not read settings changed exclusively inside an old SQLite database; review Preferences when switching from that build.

See [storage and recovery](docs/storage.md) and [folder rules](docs/folder-instructions.md).

## AI connections

**Settings → AI connection** defaults to Apple Intelligence on this Mac. Choose **Connect another model** to enter an Ollama, OpenAI-compatible or Anthropic endpoint and any supported text model ID. **Find models** can populate IDs from the service. API keys are optional where the service permits and are saved in macOS Keychain, separately from folder settings.

**Test connection** checks the draft with a short greeting. **Save settings** applies it explicitly. Custom services receive document text and filing instructions; the UI identifies the selected endpoint. There is no silent fallback. Legacy Gemma settings remain readable. See [model connections](docs/model-switching.md) for protocols, privacy and limits.

**Settings → Folder nesting** sets a maximum of one or two folder levels. Fewer levels are always allowed. Optional **Folder creation preferences** beside it describe how new folders should be grouped; saved folder rules describe what belongs in each folder. Technical limits are managed automatically. Review proposals before changing the structure.

Apple uses verified source lines and staged subject/subfolder suggestions for any identifiable document type. Short, complete documents receive proposals; long, incomplete or unclear documents require review. Suggested folders remain editable drafts. See [the current Apple workflow](docs/apple-evidence-workflow.md) for limits and validation. The [rules and nesting evaluation](evals/rules-nesting/README.md) exercises all three rule scopes across projects, invoices, health, education, warranties and travel. See the [measured results and limitations](evals/reports/2026-10-04/rules-nesting/README.md).

PDFKit and Vision extract document text for every model. The same serial workflow describes each document, suggests categories or chooses a folder, independently verifies the match, and optionally suggests an English filename. Production roles currently receive extracted text, not PDF/image attachments. Rust validates evidence, paths, source contents and preference revisions before file operations. Unclear results need input; naming failure preserves the original name.

Automatic language detection, translation preflight and download suggestions are disabled. See [architecture diagrams](docs/architecture.md), [model switching](docs/model-switching.md), and [accuracy evidence](docs/ai-accuracy.md). The header distinguishes on-device AI from a configured model connection.

## Development

Requires Node.js 22.12+ and Xcode Command Line Tools. Rust 1.90 is selected by `rust-toolchain.toml`.

```sh
npm install
npm run tauri dev
npm run check
npm run lint:rust
npm run tauri build -- --bundles app
```

The app bundle is `src-tauri/target/release/bundle/macos/Tuck.app`. This is a local development build, not a notarized distribution. The normal tests use synthetic documents and mock model outputs; Apple-dependent evaluations are opt-in ignored tests. UI tests mock the Tauri bridge.

## Limits and later work

No Google Drive integration or file synchronization yet. Back up the documents and hidden `.tuck` files together. Preferences and instruction text are plaintext; local storage is not additional encryption.

There is no persistent file-operation journal or automatic crash reconciliation. Copies are written through a synced temporary file and exclusive destination creation. Within-root moves use an exclusive hard link followed by removal of the source. Interruption between those steps can leave two copies; Tuck keeps them and does not silently clean them up or replay work on restart. While watching, Finder edits and moves are detected as changed/new paths and read again. There is no durable identity tracking across app restarts.

Reconsider SQLite if resumable jobs, durable undo or a persistent document index become necessary. Session-only watching does not require a database.
