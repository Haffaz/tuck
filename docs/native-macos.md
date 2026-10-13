# Native macOS integration

Tuck uses Apple frameworks for everything that touches documents and the system. This page lists
which ones, and how they are wired into a Tauri app.

## Reading documents (`tuck-reader`)

- **PDFKit** extracts embedded text page by page (up to 12 pages). Pages with fewer than 40
  characters are rendered at up to 1600 × 2200 and read with OCR.
- **Vision `RecognizeDocumentsRequest`** (macOS 26) reads scans with structure: paragraphs, lists
  and table cells, with tables kept as HTML. A coverage check falls back to Vision's plain transcript
  if the structured output would drop a word. When the structured API gives no text,
  **`VNRecognizeTextRequest`** is used, with automatic language detection and correction.
- **OCR confidence** decides whether a scan is readable. The document API's confidence is coarse,
  so pages below 0.6 are re-checked with `VNRecognizeTextRequest`; only pages that both rate low
  are treated as unreadable.
- **Vision document segmentation + Core Image** straighten a photographed page in memory when a
  large, clearly skewed sheet is detected. Originals are never rewritten.
- **`VNClassifyImageRequest`** labels photos that contain little text ("beach", "dog"), and tells
  pictures of paper from scenes.
- **`VNDetectFaceRectanglesRequest`** and **`VNDetectBarcodesRequest`** run on page 1 to find a
  photo of a person (ID cards, passports) and barcodes or QR codes (tickets, payment slips).
- **Extended attributes**: the reader reads `com.apple.metadata:kMDItemWhereFroms`, which Safari and
  Mail set on downloads, directly with `getxattr`, so it works whether or not Spotlight indexed the
  folder. Only the domain is kept.

The reader does not log or store the text it extracts.

## Models and embeddings (`tuck-apple`)

- **Foundation Models** runs Apple's on-device language model. Document requests use the
  `permissiveContentTransformations` guardrail mode, which Apple documents for transforming
  user-provided text; guided output can still be refused, so card requests fall back to plain text.
  Prompts are counted against the model's context size before sending.
- **NaturalLanguage `NLEmbedding.sentenceEmbedding(for: .english)`** turns each card into a vector
  for clustering and folder ranking. No language model is involved in that step.
- **URLSession** talks to custom connections with an ephemeral configuration: no cookies, cache,
  proxies or redirects (see [model connections](model-connections.md)).
- **Security / Keychain** stores API keys per connection type and endpoint.

## Preview and folder access (`TuckMac`)

`TuckMac.swift` is compiled into a static library and linked into the Tauri binary.

- **Quick Look**: clicking a file name opens a reusable AppKit window with `QLPreviewView`. Rust
  passes only a session document ID; the path is looked up and checked in Rust.
- **Security-scoped bookmarks**: after a folder is chosen, a bookmark is saved in the app support
  folder (outside the document folder). On launch it is resolved and renewed, so a renamed folder is
  found again.

The app is not sandboxed yet. A Mac App Store build would need matching entitlements and checks
that the helpers inherit the folder grant.

## Watching

The Rust `notify` crate uses **FSEvents**. Events only wake a scan; the scan, a six-second settling
period, file signatures and hashes decide what is read. Idle folders are rescanned every minute to
recover missed events, and if FSEvents is unavailable Tuck falls back to polling every three seconds.

## Building

`src-tauri/build.rs` compiles the Swift sources with `swiftc` when Rust builds:
`tuck-reader` and `tuck-apple` into `src-tauri/resources/` (bundled with the app), and `TuckMac`
into a static library. A helper is recompiled only when its sources are newer than the binary,
because Tauri's dev watcher would otherwise rebuild in a loop.
