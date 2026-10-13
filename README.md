# Tuck

A little home for every document. A local macOS document organizer built with Tauri 2, React, TypeScript, Rust, PDFKit, Apple Vision, and Apple Intelligence.

## How to use it

1. **Choose folder.** Tuck reads every supported document in it (and its subfolders) on this Mac and suggests folders. Nothing moves yet.
2. **Check the suggestion.** Each folder shows how many files it gets. Rename (✎) or remove (✕) a folder, open it to move a file elsewhere or edit what belongs there. Files Tuck could not read wait for you.
3. **Organize.** The folders are saved and the files move in one step, renamed like `Bank Statement - Nordea - 2026-03` unless renaming is off. Tick **Keep watching for new files** to have new documents filed as they arrive; you can also drop files on the window.
4. **Files that need you** are listed with a folder picker: **File** or **Skip**. **Done** ends the session.

A folder that already has Tuck folders shows them with **Organize new files** and **Suggest again**. Settings (gear, or Command-comma) holds the AI model, folder levels and renaming; folder preferences and filing instructions are under **Advanced**.

Files inside the root are moved; external files are copied with originals preserved. Source hashes, rule revisions, enabled destinations and exclusive output paths are checked before each operation. Existing documents are never overwritten. The model can still make a wrong but plausible decision; automatic filing does not imply calibrated certainty.

The session supports up to 2,000 PDF, PNG, JPEG, HEIC, TIFF, TXT or Markdown documents, each at most 100 MB. A folder exceeding the scan limits is rejected rather than partially organized. Unreadable and oversized documents are shown in the map. Reading uses bounded excerpts and optional page tools, not guaranteed exhaustive page comprehension. Reading and describing run a few documents at a time; the on-device model itself serves one request at a time.

The lightweight map connects documents to source folders and identical content. It is a setup snapshot, not a semantic knowledge graph or persistent search index. It stays in memory. Watching checks new/changed files every three seconds while the worker is available, waits at least six seconds of unchanged size/time, then reads them again. Own moves are recorded so they do not cause a filing loop. Rules edited on disk stop automatic filing.

The interface follows system light/dark appearance and Reduce Motion. There is one root, no database, built-in cloud sync, saved operation history or durable undo. Reusable document cards live in `.tuck/cache/cards/`; folder choice uses those cards with your current rules. See [architecture diagrams](docs/architecture.md), [product experience](docs/product-experience.md), [category suggestions](docs/category-suggestions.md) and [automatic organization](docs/automatic-organization.md).

## Simple local storage

```text
Documents/
├── .tuck/
│   ├── settings.json           # small preferences
│   └── instructions.md         # root prompt
└── Migration/
    ├── .tuck/instructions.md   # migration prompt
    └── Cedar/
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
