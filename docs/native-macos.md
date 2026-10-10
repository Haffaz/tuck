# Native macOS integration

## Reading documents

Readable PDF pages retain the PDFKit embedded-text path. Pages with fewer than 40 trimmed characters, or explicitly requested OCR, are rendered at up to 1600 x 2200 and read with Vision `RecognizeDocumentsRequest` on macOS 26+. Images are oriented and bounded to 2200 pixels. Paragraphs, lists and table cells become text with HTML tables; merged-cell spans are retained. A serializer coverage check falls back to Vision's transcript when a word would be omitted. Geometric block ordering is a simple approximation, not a guarantee for complex multi-column layouts.

On older systems, failed structured recognition or empty results use accurate `VNRecognizeTextRequest`. Language detection and correction are enabled. The reader does not log or persist full extracted text. The later understanding stage stores short verified evidence quotes alongside descriptions; see [storage](storage.md). Existing 12-page / 14 KB default bounds and the parent's timeout remain.

Photographs can be rectified in memory with Vision document detection and Core Image perspective correction. Only a large, high-confidence, visibly skewed detected sheet qualifies. Original files are never rewritten. If corrected recognition fails or is empty, the original image is read. Cropping still requires broader real-world evaluation; the confidence threshold is a detector heuristic, not proof that all relevant context was preserved.

## Preview

Click a document name in Needs attention or Activity to open a reusable AppKit window containing `QLPreviewView`. Keyboard activation uses the native button behavior. Escape closes the preview, and closing releases its document. Rust accepts a session document ID, looks up the original or actual organized destination, and rejects unavailable/non-regular files. The Swift bridge runs on the main thread inside Tauri. Previewing never authorizes organization.

## Watching

`notify` selects FSEvents on macOS. Notifications are advisory wakeups; the existing enumeration, six-second settling period, signatures, source hashes, settings revisions and move validation still decide what to process. Idle folders reconcile every 60 seconds. Pending changes poll every three seconds until settled. An unavailable or failed native subscription uses the previous polling behavior. Stopping/changing folders drops subscriptions and isolates callback flags by folder.

## Folder access

The in-process Swift bridge creates a security-scoped bookmark after folder selection, stored as `folder-access.json` beside the app's root pointer, outside the document folder. It contains a path and opaque bookmark data, not document contents or session history. On restart, it resolves the bookmark, renews stale data and retains the URL grant before Rust reads settings. A renamed folder can be resolved and the pointer updated. Older installations without a bookmark keep their saved path and create a bookmark when accessible. Failed grants ask the user to choose the folder again.

The current distributed app is not App-Sandbox enabled. Bookmark round trips in this build do not certify an App Store sandbox configuration. A sandboxed distribution still needs matching entitlements, picker/import scope handling, and validation of the OCR helpers' inherited permissions.

## Build and scope

`build.rs` compiles the Swift preview/access bridge as a static library linked into Tauri, using system Swift runtime libraries. The reader remains a bounded subprocess. No extra UI helper app or local service is introduced. Separate language detection and Translation preflight are disabled; Vision OCR remains. Content tagging and Spotlight indexing are not added by this change.

## Verification

Rust tests cover idle scan suppression, event/settling/fallback scheduling, subscription shutdown, existing watcher/file-operation behavior, bookmark persistence and preview path selection. Playwright checks that preview dispatch uses a document ID without organizing. Live native checks use synthetic files only; accuracy comparisons are under `evals/reports/2026-09-30/apple-document-extraction/` and do not imply every OCR input improves.

### Verification on 2026-09-30

- Rust: 65 passed, 8 live-model tests ignored; Clippy passed with warnings denied.
- UI: 31 Playwright tests passed; production frontend and macOS app bundle built.
- Native smoke: a synthetic folder bookmark resolved after a rename; Quick Look opened and released its document when closed.
- Reader smoke: embedded PDF text retained the expected bank-statement heading (0.19 s); forced OCR retained a synthetic table (0.29 s). These are individual warm smoke checks, not performance benchmarks.
- Photo smoke: a synthetic perspective-distorted page was rectified and its bank-statement heading remained readable. Broader photograph accuracy is still unverified.

The build output is `src-tauri/target/release/bundle/macos/Tuck.app`. These checks do not run a live personal-folder organization or the full Apple model accuracy suite.
