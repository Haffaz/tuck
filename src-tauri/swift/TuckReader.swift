import AppKit
import PDFKit
import Vision
import Foundation
import CoreImage

func plainOCR(_ image: CGImage) throws -> String {
    let request = VNRecognizeTextRequest()
    request.recognitionLevel = .accurate
    request.automaticallyDetectsLanguage = true
    request.usesLanguageCorrection = true
    try VNImageRequestHandler(cgImage: image, options: [:]).perform([request])
    return (request.results ?? []).compactMap { $0.topCandidates(1).first?.string }.joined(separator: "\n")
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
func recognize(_ image: CGImage) async throws -> String {
    if #available(macOS 26.0, *) {
        var request = RecognizeDocumentsRequest()
        request.textRecognitionOptions.automaticallyDetectLanguage = true
        request.textRecognitionOptions.useLanguageCorrection = true
        if let documents = try? await request.perform(on: image) {
            let text = documents.map { structuredText($0.document) }.joined(separator: "\n\n")
            if !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty { return text }
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
@main struct TuckReader {
    static func main() async {
        guard CommandLine.arguments.count == 2 || CommandLine.arguments.count == 4 else { exit(1) }
        let requestedPage = CommandLine.arguments.count == 4 ? Int(CommandLine.arguments[2]) : nil
        let forceOCR = CommandLine.arguments.count == 4 && CommandLine.arguments[3] == "true"
        if CommandLine.arguments.count == 4 && (requestedPage == nil || requestedPage! < 1 || requestedPage! > 50) { exit(1) }
        let url = URL(fileURLWithPath: CommandLine.arguments[1])
        do {
            var text = ""
            if url.pathExtension.lowercased() == "pdf" {
                guard let document = PDFDocument(url: url), !document.isLocked else { exit(2) }
                if let requestedPage, requestedPage > document.pageCount { exit(5) }
                let indices = requestedPage.map { ($0 - 1)..<$0 } ?? (0..<min(document.pageCount, 12))
                text = "[Document has \(document.pageCount) pages.]\n"
                for index in indices {
                    guard let page = document.page(at: index) else { continue }
                    var content = page.string ?? ""
                    if forceOCR || content.trimmingCharacters(in: .whitespacesAndNewlines).count < 40 {
                        let preview = page.thumbnail(of: NSSize(width: 1600, height: 2200), for: .mediaBox)
                        if let image = preview.cgImage(forProposedRect: nil, context: nil, hints: nil) { content = try await recognize(image) }
                    }
                    text += "\n[Page \(index + 1)]\n" + content
                    if text.utf8.count >= 14000 { break }
                }
            } else {
                if let requestedPage, requestedPage != 1 { exit(5) }
                guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
                      let image = CGImageSourceCreateThumbnailAtIndex(source, 0, [kCGImageSourceCreateThumbnailFromImageAlways: true, kCGImageSourceThumbnailMaxPixelSize: 2200, kCGImageSourceCreateThumbnailWithTransform: true] as CFDictionary) else { exit(3) }
                if let corrected = correctedPhoto(image), let recognized = try? await recognize(corrected), !recognized.isEmpty { text = recognized }
                else { text = try await recognize(image) }
            }
            let bounded = String(decoding: Array(text.utf8.prefix(14000)), as: UTF8.self)
                + (text.utf8.count > 14000 ? "\n[Text truncated]" : "")
            let data = try JSONSerialization.data(withJSONObject: ["text": bounded])
            FileHandle.standardOutput.write(data)
        } catch { exit(4) }
    }
}
