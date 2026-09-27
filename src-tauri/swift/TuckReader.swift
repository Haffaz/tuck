import AppKit
import PDFKit
import Vision
import Foundation

func recognize(_ image: CGImage) throws -> String {
    let request = VNRecognizeTextRequest()
    request.recognitionLevel = .accurate
    request.recognitionLanguages = ["en-US", "sv-SE"]
    request.usesLanguageCorrection = true
    try VNImageRequestHandler(cgImage: image, options: [:]).perform([request])
    return (request.results ?? []).compactMap { $0.topCandidates(1).first?.string }.joined(separator: "\n")
}

guard CommandLine.arguments.count == 2 else { exit(1) }
let url = URL(fileURLWithPath: CommandLine.arguments[1])
do {
    var text = ""
    if url.pathExtension.lowercased() == "pdf" {
        guard let document = PDFDocument(url: url), !document.isLocked else { exit(2) }
        for index in 0..<min(document.pageCount, 12) {
            guard let page = document.page(at: index) else { continue }
            var content = page.string ?? ""
            if content.trimmingCharacters(in: .whitespacesAndNewlines).count < 40 {
                let preview = page.thumbnail(of: NSSize(width: 1600, height: 2200), for: .mediaBox)
                if let image = preview.cgImage(forProposedRect: nil, context: nil, hints: nil) { content = try recognize(image) }
            }
            text += "\n" + content
            if text.utf8.count >= 14000 { break }
        }
    } else {
        guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
              let image = CGImageSourceCreateThumbnailAtIndex(source, 0, [kCGImageSourceCreateThumbnailFromImageAlways: true, kCGImageSourceThumbnailMaxPixelSize: 2200, kCGImageSourceCreateThumbnailWithTransform: true] as CFDictionary) else { exit(3) }
        text = try recognize(image)
    }
    // Keep the pipe payload bounded; the Rust parent enforces a time limit.
    let bounded = String(decoding: Array(text.utf8.prefix(14000)), as: UTF8.self)
    let data = try JSONSerialization.data(withJSONObject: ["text": bounded])
    FileHandle.standardOutput.write(data)
} catch { exit(4) }
