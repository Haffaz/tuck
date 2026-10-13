import AppKit
import PDFKit
import Vision
import Foundation
import CoreImage

/// Recognized text plus each line's confidence; low confidence means an unsupported
/// script or a poor scan (measured: good scans 0.74-1.0, Sinhala/Tamil 0.10-0.42).
struct Recognized { var text: String; var confidences: [Float] }

func plainOCR(_ image: CGImage) throws -> Recognized {
    let request = VNRecognizeTextRequest()
    request.recognitionLevel = .accurate
    request.automaticallyDetectsLanguage = true
    request.usesLanguageCorrection = true
    try VNImageRequestHandler(cgImage: image, options: [:]).perform([request])
    let best = (request.results ?? []).compactMap { $0.topCandidates(1).first }
    return Recognized(text: best.map(\.string).joined(separator: "\n"), confidences: best.map(\.confidence))
}
func photoLabels(_ image: CGImage) -> [String] {
    let request = VNClassifyImageRequest()
    guard (try? VNImageRequestHandler(cgImage: image, options: [:]).perform([request])) != nil else { return [] }
    return (request.results ?? []).filter { $0.confidence > 0.3 }.prefix(5)
        .map { $0.identifier.replacingOccurrences(of: "_", with: " ") }
}
func escaped(_ text: String) -> String {
    text.replacingOccurrences(of: "&", with: "&amp;").replacingOccurrences(of: "<", with: "&lt;")
        .replacingOccurrences(of: ">", with: "&gt;").replacingOccurrences(of: "\n", with: "<br>")
}
@available(macOS 26.0, *)
func structuredText(_ doc: DocumentObservation.Container) -> String {
    var blocks: [(CGRect, String)] = []
    var structuredRegions: [CGRect] = []
    for table in doc.tables {
        var seen = Set<String>()
        var rows = ["<table>"]
        for row in table.rows {
            var cells = ""
            for cell in row {
                let key = "\(cell.rowRange.lowerBound):\(cell.columnRange.lowerBound)"
                guard seen.insert(key).inserted else { continue }
                let attributes = (cell.rowRange.count > 1 ? " rowspan=\"\(cell.rowRange.count)\"" : "")
                    + (cell.columnRange.count > 1 ? " colspan=\"\(cell.columnRange.count)\"" : "")
                cells += "<td\(attributes)>\(escaped(cell.content.text.transcript))</td>"
            }
            rows.append("<tr>\(cells)</tr>")
        }
        rows.append("</table>")
        let rect = table.boundingRegion.normalizedPath.boundingBox
        blocks.append((rect, rows.joined(separator: "\n")))
        structuredRegions.append(rect)
    }
    for list in doc.lists {
        let rect = list.boundingRegion.normalizedPath.boundingBox
        blocks.append((rect, list.items.map { "\($0.markerString.isEmpty ? "-" : $0.markerString) \($0.itemString)" }.joined(separator: "\n")))
        structuredRegions.append(rect)
    }
    for paragraph in doc.paragraphs {
        let rect = paragraph.boundingRegion.normalizedPath.boundingBox
        let isInside = structuredRegions.contains { region in
            let intersection = region.intersection(rect)
            return !intersection.isNull && intersection.width * intersection.height > 0.8 * rect.width * rect.height
        }
        if !isInside { blocks.append((rect, paragraph.transcript)) }
    }
    blocks.sort { a, b in a.0.maxY == b.0.maxY ? a.0.minX < b.0.minX : a.0.maxY > b.0.maxY }
    let text = blocks.map { $0.1 }.joined(separator: "\n\n")
    // Geometry can overlap on complex layouts. Never drop recognized words when serializing.
    let words: (String) -> Set<String> = { Set($0.lowercased().split { !$0.isLetter && !$0.isNumber }.map(String.init)) }
    if text.isEmpty || !words(doc.text.transcript).isSubset(of: words(text)) { return doc.text.transcript }
    return text
}
func recognize(_ image: CGImage) async throws -> Recognized {
    if #available(macOS 26.0, *) {
        var request = RecognizeDocumentsRequest()
        request.textRecognitionOptions.automaticallyDetectLanguage = true
        request.textRecognitionOptions.useLanguageCorrection = true
        if let documents = try? await request.perform(on: image) {
            let text = documents.map { structuredText($0.document) }.joined(separator: "\n\n")
            let confidences = documents.flatMap { $0.document.text.lines.map(\.confidence) }
            if !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                // This API's confidence is coarse (clean pages can score ~0.5). When it looks
                // low, confirm with the classic recogniser, which separates unsupported scripts
                // (0.30-0.42) from readable pages (0.75-1.0). Clean pages skip the second pass.
                guard average(confidences) < unreadableConfidence,
                      let check = try? plainOCR(image), !check.confidences.isEmpty else {
                    return Recognized(text: text, confidences: confidences)
                }
                return Recognized(text: text, confidences: check.confidences)
            }
        }
    }
    return try plainOCR(image)
}
// Only rectify a large, confidently detected, visibly skewed sheet. Clean scans stay untouched.
func correctedPhoto(_ image: CGImage) -> CGImage? {
    guard #available(macOS 12.0, *) else { return nil }
    let request = VNDetectDocumentSegmentationRequest()
    guard (try? VNImageRequestHandler(cgImage: image, options: [:]).perform([request])) != nil,
          let page = request.results?.first, page.confidence >= 0.9,
          page.boundingBox.width * page.boundingBox.height >= 0.55,
          abs(page.topLeft.y - page.topRight.y) > 0.025 || abs(page.topLeft.x - page.bottomLeft.x) > 0.025 else { return nil }
    let input = CIImage(cgImage: image)
    let point: (CGPoint) -> CIVector = { CIVector(x: $0.x * input.extent.width, y: $0.y * input.extent.height) }
    let output = input.applyingFilter("CIPerspectiveCorrection", parameters: [
        "inputTopLeft": point(page.topLeft), "inputTopRight": point(page.topRight),
        "inputBottomLeft": point(page.bottomLeft), "inputBottomRight": point(page.bottomRight)
    ])
    return CIContext().createCGImage(output, from: output.extent)
}
let unreadableConfidence: Float = 0.6

func average(_ values: [Float]) -> Float {
    values.isEmpty ? 1 : values.reduce(0, +) / Float(values.count)
}

@main struct TuckReader {
    static func main() async {
        let arguments = CommandLine.arguments
        // `tuck-reader <file> --clues`: only file clues, for text files Rust reads itself.
        if arguments.count == 3 && arguments[2] == "--clues" {
            let url = URL(fileURLWithPath: arguments[1])
            emitReading(text: "", clues: whereFromDomains(url).map { "Downloaded from or sent by: \($0)" }, confidences: [], unreadable: false)
            return
        }
        guard arguments.count == 2 || arguments.count == 4 else { exit(1) }
        let requestedPage = arguments.count == 4 ? Int(arguments[2]) : nil
        let forceOCR = arguments.count == 4 && arguments[3] == "true"
        if arguments.count == 4 && (requestedPage == nil || requestedPage! < 1 || requestedPage! > 50) { exit(1) }
        let url = URL(fileURLWithPath: arguments[1])
        var clues = whereFromDomains(url).map { "Downloaded from or sent by: \($0)" }
        var confidences: [Float] = []
        var paper = true
        do {
            var text = ""
            if url.pathExtension.lowercased() == "pdf" {
                guard let document = PDFDocument(url: url), !document.isLocked else { exit(2) }
                if let requestedPage, requestedPage > document.pageCount { exit(5) }
                clues += pdfClues(document)
                let indices = requestedPage.map { ($0 - 1)..<$0 } ?? (0..<min(document.pageCount, 12))
                text = "[Document has \(document.pageCount) pages.]\n"
                for index in indices {
                    guard let page = document.page(at: index) else { continue }
                    var content = page.string ?? ""
                    let scanned = forceOCR || content.trimmingCharacters(in: .whitespacesAndNewlines).count < 40
                    if scanned || index == indices.lowerBound {
                        let preview = page.thumbnail(of: NSSize(width: 1600, height: 2200), for: .mediaBox)
                        if let image = preview.cgImage(forProposedRect: nil, context: nil, hints: nil) {
                            if index == indices.lowerBound { clues += imageClues(image) }
                            if scanned {
                                let recognized = try await recognize(image)
                                content = recognized.text
                                confidences += recognized.confidences
                            }
                        }
                    }
                    text += "\n[Page \(index + 1)]\n" + content
                    if text.utf8.count >= 14000 { break }
                }
            } else {
                if let requestedPage, requestedPage != 1 { exit(5) }
                guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
                      let image = CGImageSourceCreateThumbnailAtIndex(source, 0, [kCGImageSourceCreateThumbnailFromImageAlways: true, kCGImageSourceThumbnailMaxPixelSize: 2200, kCGImageSourceCreateThumbnailWithTransform: true] as CFDictionary) else { exit(3) }
                clues += imageClues(image)
                var recognized: Recognized
                if let corrected = correctedPhoto(image), let fixed = try? await recognize(corrected), !fixed.text.isEmpty { recognized = fixed }
                else { recognized = try await recognize(image) }
                text = recognized.text
                confidences = recognized.confidences
                // A picture rather than a document: describe what it shows so it can still be grouped.
                if text.filter(\.isLetter).count < 25 {
                    let labels = photoLabels(image)
                    paper = looksLikeDocument(labels)
                    if !labels.isEmpty { text += "\n[Photo labels: \(labels.joined(separator: ", "))]" }
                }
            }
            if let zone = machineReadableClue(text) { clues.append(zone) }
            let bounded = String(decoding: Array(text.utf8.prefix(14000)), as: UTF8.self)
                + (text.utf8.count > 14000 ? "\n[Text truncated]" : "")
            emitReading(text: bounded, clues: clues, confidences: confidences,
                        unreadable: paper && !confidences.isEmpty && average(confidences) < unreadableConfidence)
        } catch { exit(4) }
    }
}

func emitReading(text: String, clues: [String], confidences: [Float], unreadable: Bool) {
    var output: [String: Any] = ["text": text, "clues": Array(clues.prefix(12)), "unreadable": unreadable]
    if !confidences.isEmpty { output["ocrConfidence"] = Double(average(confidences)) }
    guard let data = try? JSONSerialization.data(withJSONObject: output) else { exit(4) }
    FileHandle.standardOutput.write(data)
}
