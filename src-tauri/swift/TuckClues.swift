import Foundation
import PDFKit
import Vision

// Cheap, local clues about a file that are not in its text: where it came from, what made
// it, PDF properties and form fields, and what page 1 shows (faces, barcodes, passport
// lines). Each clue is one short line for the describing model. Nothing leaves the Mac.

/// The domain part of a "Where from" entry: a URL host or an email address domain.
func clueDomain(_ entry: String) -> String? {
    if let host = URL(string: entry.trimmingCharacters(in: .whitespaces))?.host, host.contains(".") {
        return host.lowercased().replacingOccurrences(of: "www.", with: "")
    }
    guard let at = entry.lastIndex(of: "@") else { return nil }
    let domain = entry[entry.index(after: at)...].prefix { $0.isLetter || $0.isNumber || $0 == "." || $0 == "-" }
    return domain.contains(".") ? domain.lowercased() : nil
}

/// Download page or email sender, from the extended attribute macOS writes on download.
/// Read directly so it works whether or not Spotlight indexed the folder.
func whereFromDomains(_ url: URL) -> [String] {
    let name = "com.apple.metadata:kMDItemWhereFroms"
    let size = getxattr(url.path, name, nil, 0, 0, 0)
    guard size > 0, size < 64 * 1024 else { return [] }
    var data = Data(count: size)
    let read = data.withUnsafeMutableBytes { getxattr(url.path, name, $0.baseAddress, size, 0, 0) }
    guard read == size,
          let entries = try? PropertyListSerialization.propertyList(from: data, format: nil) as? [String] else { return [] }
    var seen: [String] = []
    for domain in entries.compactMap(clueDomain) where !seen.contains(domain) { seen.append(domain) }
    return Array(seen.prefix(3))
}

/// Short single-line text, or nil when empty.
func clueText(_ value: Any?, limit: Int = 80) -> String? {
    guard let text = (value as? String)?.replacingOccurrences(of: "\n", with: " ")
        .trimmingCharacters(in: .whitespacesAndNewlines), !text.isEmpty else { return nil }
    return String(text.prefix(limit))
}

func pdfClues(_ document: PDFDocument) -> [String] {
    var clues: [String] = []
    let attributes = document.documentAttributes ?? [:]
    for (key, label) in [(PDFDocumentAttribute.titleAttribute, "PDF title"),
                         (.subjectAttribute, "PDF subject"),
                         (.authorAttribute, "PDF author"),
                         (.creatorAttribute, "Made with"),
                         (.producerAttribute, "PDF producer")] {
        if let value = clueText(attributes[key]) { clues.append("\(label): \(value)") }
    }
    var fields: [String] = []
    for index in 0..<min(document.pageCount, 3) {
        for annotation in document.page(at: index)?.annotations ?? [] {
            if let field = clueText(annotation.fieldName, limit: 40), !fields.contains(field) { fields.append(field) }
        }
    }
    if !fields.isEmpty { clues.append("Form fields: \(fields.prefix(15).joined(separator: ", "))") }
    return clues
}

/// Meaning of common barcode payloads, without copying account numbers or amounts.
func barcodeClue(_ payload: String, symbology: String) -> String {
    let start = payload.prefix(4).uppercased()
    if start.hasPrefix("BCD") { return "Payment QR code (SEPA transfer)" }
    if start.hasPrefix("SPC") { return "Payment QR code (Swiss QR-bill)" }
    if start.hasPrefix("M1") || start.hasPrefix("M2") { return "Boarding pass barcode" }
    if let domain = clueDomain(payload) { return "\(symbology) code linking to \(domain)" }
    return "\(symbology) code"
}

/// Faces and barcodes on a page image. Small faces (crowds, logos) are ignored.
func imageClues(_ image: CGImage) -> [String] {
    let faces = VNDetectFaceRectanglesRequest()
    let barcodes = VNDetectBarcodesRequest()
    guard (try? VNImageRequestHandler(cgImage: image, options: [:]).perform([faces, barcodes])) != nil else { return [] }
    var clues: [String] = []
    let portraits = (faces.results ?? []).filter { $0.boundingBox.width * $0.boundingBox.height >= 0.004 }
    if portraits.count == 1 { clues.append("Has a photo of one person") }
    else if portraits.count > 1 { clues.append("Has photos of \(portraits.count) people") }
    var seen: [String] = []
    for code in barcodes.results ?? [] {
        let symbology = code.symbology == .qr ? "QR" : "Barcode"
        let clue = barcodeClue(code.payloadStringValue ?? "", symbology: symbology)
        if !seen.contains(clue) { seen.append(clue) }
    }
    return clues + seen.prefix(3)
}

/// Passport, ID card and visa machine-readable lines (ICAO 9303) found in the text.
func machineReadableClue(_ text: String) -> String? {
    let lines = text.split(whereSeparator: \.isNewline).map {
        $0.replacingOccurrences(of: " ", with: "").uppercased()
    }
    guard let first = lines.first(where: { line in
        line.count >= 28 && line.count <= 46 && line.contains("<<")
            && line.allSatisfy { $0.isASCII && ($0.isLetter || $0.isNumber || $0 == "<") }
            && ["P", "I", "A", "C", "V"].contains(line.first.map(String.init) ?? "")
    }) else { return nil }
    let kind: String
    switch first.first {
    case "P": kind = "passport"
    case "V": kind = "visa"
    default: kind = "identity card"
    }
    let country = String(first.dropFirst(2).prefix(3)).replacingOccurrences(of: "<", with: "")
    return "Machine-readable zone: \(kind)\(country.count == 3 ? ", issued by \(country)" : "")"
}

/// Labels that mean the image shows paper rather than a scene.
func looksLikeDocument(_ labels: [String]) -> Bool {
    labels.isEmpty || labels.contains { ["document", "text", "paper", "receipt", "letter", "menu", "screenshot"].contains($0) }
}
