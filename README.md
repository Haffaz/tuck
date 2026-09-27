# Tuck

A little home for every document. A local document organizer for macOS, built with Tauri 2, React, TypeScript, Rust, and Apple Vision.

## Run

Requires Node.js 22.12+ and Xcode Command Line Tools. Rust 1.90 is selected by `rust-toolchain.toml`.

```sh
npm install
npm run tauri dev
```

Create the Mac application:

```sh
npm run tauri build -- --bundles app
```

The output is `src-tauri/target/release/bundle/macos/Tuck.app`. It is a local development build, not a signed/notarized distribution.

## First use

1. Choose a document folder. Tuck starts paused and records the current files as a baseline.
2. Edit **Folder rules**. Comma-separated keywords match document text or filenames; the first matching rule wins. Use at most two folder levels.
3. Optionally connect a local Ollama model. Keyword rules work without it.
4. Click **Start watching**. New or changed documents are checked every two seconds; Tuck waits for two matching scans before sorting.
5. Use **Sort existing** for an initial cleanup. Drag files into the window or use **Add documents** to import them.

Tuck watches while the app is running. Closing or quitting the app stops the watcher. Saved watching state resumes when reopened. This version has no login startup or menu bar service.

## Local AI

Install Ollama separately, then:

```sh
ollama pull qwen3:4b
```

Start Ollama, choose the installed model in **Preferences**, and click **Check connection**. Tuck calls only `http://127.0.0.1:11434`, bypasses HTTP proxies, and never sends document contents to a cloud provider. Tuck does not download models itself.

Rules determine the destination when they match. Local AI can suggest an English filename and classify documents unmatched by keywords. AI output is restricted to your saved folders. Uncertain documents go to **Needs Review** with the original name. If Ollama is unavailable, matching rules still work and filenames stay unchanged.

## Supported documents

- PDF, PNG, JPG/JPEG, HEIC, TIFF, TXT, Markdown.
- Apple PDFKit extracts PDF text; Apple Vision reads scans locally in English and Swedish.
- To bound runtime, the reader examines at most 12 PDF pages and 14 KB of extracted text. Files above 100 MB are left in place with an activity error.
- Hidden paths and symbolic links are excluded from folder scans. Unsupported formats are left alone.
- DOCX, cloud providers, Google Drive sync, and learning rules from manual moves are future work.

## File behavior

- Files inside the selected root are moved; files imported from elsewhere are copied and their originals are kept.
- Destination files are created exclusively. A name collision gets a numeric suffix; existing files are never overwritten.
- Exact matching contents are flagged against previously organized documents. Matching copies are always kept, including application bundles.
- Tuck records its own output signatures to prevent sorting loops. Undo records the restored file as already handled.
- Undo checks the content hash and refuses to remove or replace edited documents. Undoing a copy also requires the unchanged original to still exist.
- Moves across volumes fail safely with the original kept. Choose a root on one volume.
- Preferences and the operation journal live in `~/Library/Application Support/app.tuck.desktop/state.json`. The journal stores paths, hashes, and explanations; extracted document text is not persisted.
- Pending moves are recorded before touching files and reconciled on startup. Interrupted or ambiguous operations preserve files and pause the watcher.
- macOS may ask for access to Documents or other protected folders. Tuck needs that access to watch the selected location.

## Verify

```sh
npm run build
npm run test:rust
npx playwright install chromium
npm run test:ui
```

Rust tests use temporary synthetic documents to check stability, initial cleanup, collisions, duplicate preservation, pause, persistence, and undo. Browser tests exercise the interface through a mocked Tauri bridge; they do not claim native file-picker or AI inference coverage.

## Structure

- `src/`: interface, preferences, activity, and native bridge calls.
- `src-tauri/src/engine.rs`: watched-folder polling, queue, journal, and undo.
- `src-tauri/src/files.rs`: path validation, signatures, hashes, and file operations.
- `src-tauri/src/classifier.rs`: rules, bounded local extraction, and Ollama JSON output.
- `src-tauri/swift/TuckReader.swift`: bundled PDF and OCR reader compiled during the Rust build.

The UI can be previewed with `npm run dev`. Browser preview disables native filesystem actions.
