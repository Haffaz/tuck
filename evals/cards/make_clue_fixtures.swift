// Synthetic files that carry clues outside their text, for the reader's clue extraction.
// Usage: swiftc -parse-as-library make_clue_fixtures.swift -o gen && ./gen OUT_DIR
// All names, numbers and accounts are invented.
import AppKit
import CoreImage
import Foundation
import PDFKit

func render(_ lines: [String], size: NSSize = NSSize(width: 1240, height: 1754), font: NSFont = .systemFont(ofSize: 30),
            extra: ((NSRect) -> Void)? = nil) -> NSImage {
    let image = NSImage(size: size)
    image.lockFocus()
    NSColor.white.setFill()
    NSRect(origin: .zero, size: size).fill()
    var y = size.height - 120
    for line in lines {
        (line as NSString).draw(at: NSPoint(x: 90, y: y), withAttributes: [.font: font, .foregroundColor: NSColor.black])
        y -= font.pointSize * 1.6
    }
    extra?(NSRect(origin: .zero, size: size))
    image.unlockFocus()
    return image
}

func savePNG(_ image: NSImage, _ url: URL) throws {
    let rep = NSBitmapImageRep(data: image.tiffRepresentation!)!
    try rep.representation(using: .png, properties: [:])!.write(to: url)
}

/// Marks a file as downloaded from `sources`, the way Safari and Mail do.
func setWhereFrom(_ url: URL, _ sources: [String]) throws {
    let data = try PropertyListSerialization.data(fromPropertyList: sources, format: .binary, options: 0)
    let result = data.withUnsafeBytes {
        setxattr(url.path, "com.apple.metadata:kMDItemWhereFroms", $0.baseAddress, data.count, 0, 0)
    }
    guard result == 0 else { throw NSError(domain: "xattr", code: Int(errno)) }
}

func qr(_ payload: String) -> NSImage {
    let filter = CIFilter(name: "CIQRCodeGenerator")!
    filter.setValue(Data(payload.utf8), forKey: "inputMessage")
    let scaled = filter.outputImage!.transformed(by: CGAffineTransform(scaleX: 8, y: 8))
    let rep = NSCIImageRep(ciImage: scaled)
    let image = NSImage(size: rep.size)
    image.addRepresentation(rep)
    return image
}

@main struct Generate {
    static func main() throws {
        let out = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
        try FileManager.default.createDirectory(at: out, withIntermediateDirectories: true)

        // 1. A text PDF whose properties, form fields and download source name the issuer.
        let form = PDFDocument()
        let page = PDFPage(image: render(["Application form", "Reference 48319", "Please complete all fields."]))!
        let field = PDFAnnotation(bounds: NSRect(x: 90, y: 900, width: 400, height: 40), forType: .widget, withProperties: nil)
        field.widgetFieldType = .text
        field.fieldName = "ResidencePermitNumber"
        page.addAnnotation(field)
        let second = PDFAnnotation(bounds: NSRect(x: 90, y: 820, width: 400, height: 40), forType: .widget, withProperties: nil)
        second.widgetFieldType = .text
        second.fieldName = "Employer"
        page.addAnnotation(second)
        form.insert(page, at: 0)
        form.documentAttributes = [PDFDocumentAttribute.titleAttribute: "Ansökan om uppehållstillstånd för arbete",
                                   PDFDocumentAttribute.authorAttribute: "Migrationsverket",
                                   PDFDocumentAttribute.creatorAttribute: "Migrationsverket e-tjänst"]
        let formURL = out.appendingPathComponent("application-form.pdf")
        guard form.write(to: formURL) else { fatalError("PDF") }
        try setWhereFrom(formURL, ["https://www.migrationsverket.se/download/form-123.pdf", "https://www.migrationsverket.se/"])

        // 2. An invoice image with a SEPA payment QR code.
        let payment = "BCD\n002\n1\nSCT\nNDEASESS\nHarbor Office Ltd\nSE4550000000058398257466\nEUR180.00\n\n\nInvoice 2026-114"
        let code = qr(payment)
        let invoice = render(["Faktura 2026-114", "Harbor Office Ltd", "Att betala: 180,00 EUR"]) { _ in
            code.draw(in: NSRect(x: 840, y: 120, width: 300, height: 300))
        }
        try savePNG(invoice, out.appendingPathComponent("invoice-qr.png"))

        // 3. A passport data page: the machine-readable zone identifies it in any language.
        let mrz = NSFont(name: "OCR-B", size: 34) ?? .monospacedSystemFont(ofSize: 34, weight: .regular)
        let passport = render([
            "PASAPORTE / PASSPORT",
            "Apellidos / Surname: EXAMPLE",
            "", "", "", "", "", "", "", "", "", "", "", "", "",
        ]) { _ in
            let lines = ["P<ESPEXAMPLE<<JORDAN<<<<<<<<<<<<<<<<<<<<<<<<", "X1234567<8ESP9001019M3008150<<<<<<<<<<<<<<04"]
            for (index, line) in lines.enumerated() {
                (line as NSString).draw(at: NSPoint(x: 60, y: 220 - CGFloat(index) * 60), withAttributes: [.font: mrz, .foregroundColor: NSColor.black])
            }
        }
        try savePNG(passport, out.appendingPathComponent("passport-mrz.png"))
        print("Wrote clue fixtures to \(out.path)")
    }
}
