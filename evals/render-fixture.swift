// Synthetic OCR fixture renderer. No network and no model generation.
import AppKit
import Foundation
import PDFKit
let text = try String(contentsOfFile: CommandLine.arguments[1], encoding: .utf8)
let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: 1800, pixelsHigh: 1200, bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
NSColor.white.setFill()
NSRect(x: 0, y: 0, width: 1800, height: 1200).fill()
(text as NSString).draw(in: NSRect(x: 80, y: 80, width: 1640, height: 1040), withAttributes: [.font: NSFont.systemFont(ofSize: 35), .foregroundColor: NSColor.black])
NSGraphicsContext.restoreGraphicsState()
let output = URL(fileURLWithPath: CommandLine.arguments[2])
if output.pathExtension == "pdf" {
    let image = NSImage(size: bitmap.size)
    image.addRepresentation(bitmap)
    let pdf = PDFDocument()
    pdf.insert(PDFPage(image: image)!, at: 0)
    guard pdf.write(to: output) else { fatalError("Could not render synthetic PDF") }
} else {
    try bitmap.representation(using: .png, properties: [:])!.write(to: output)
}
