import AppKit
import Foundation
import PDFKit
import Vision

struct Input: Decodable { let path: String; let pages: [Int] }
@main struct Reader {
    static func emit(_ value: [String: Any]) {
        if let data = try? JSONSerialization.data(withJSONObject: value, options: [.sortedKeys]) { FileHandle.standardOutput.write(data) }
    }
    static func main() async {
        guard #available(macOS 26.0, *) else { emit(["error":"unsupported_os"]); return }
        do {
            var request = RecognizeDocumentsRequest()
            request.textRecognitionOptions.automaticallyDetectLanguage = true
            request.textRecognitionOptions.useLanguageCorrection = true
            if CommandLine.arguments.contains("--check") {
                emit(["revision":String(describing:request.revision),"languages":request.supportedRecognitionLanguages.map { $0.minimalIdentifier }]); return
            }
            let input = try JSONDecoder().decode(Input.self, from:FileHandle.standardInput.readDataToEndOfFile())
            guard let pdf = PDFDocument(url:URL(fileURLWithPath:input.path)), !pdf.isLocked else { emit(["error":"pdf_unavailable"]); return }
            var pages: [[String:Any]] = []
            for number in input.pages {
                guard number > 0, let page = pdf.page(at:number-1) else { emit(["error":"invalid_page"]); return }
                let start = Date()
                let preview = page.thumbnail(of:NSSize(width:1600,height:2200),for:.mediaBox)
                guard let image = preview.cgImage(forProposedRect:nil,context:nil,hints:nil) else { emit(["error":"render_failed"]); return }
                let observations = try await request.perform(on:image)
                let documents: [[String:Any]] = observations.map { observation in
                    let doc = observation.document
                    func rect(_ region: NormalizedRegion) -> [Double] {
                        let r = region.normalizedPath.boundingBox
                        return [Double(r.minX),Double(r.minY),Double(r.width),Double(r.height)]
                    }
                    return [
                        "title": doc.title?.transcript ?? "",
                        "paragraphs": doc.paragraphs.map { ["text":$0.transcript,"bounds":rect($0.boundingRegion)] as [String:Any] },
                        "tables": doc.tables.map { table in
                            ["bounds":rect(table.boundingRegion),"rows":table.rows.map { row in row.map { cell in
                                ["text":cell.content.text.transcript,"rowStart":cell.rowRange.lowerBound,"rowEnd":cell.rowRange.upperBound,"colStart":cell.columnRange.lowerBound,"colEnd":cell.columnRange.upperBound] as [String:Any]
                            }}] as [String:Any]
                        },
                        "lists":doc.lists.map { list in ["bounds":rect(list.boundingRegion),"items":list.items.map { ["marker":$0.markerString,"text":$0.itemString] }] as [String:Any] }
                    ]
                }
                pages.append(["page":number,"seconds":Date().timeIntervalSince(start),"documents":documents,"transcript":observations.map { $0.document.text.transcript }.joined(separator:"\n\n"),"tables":observations.reduce(0) { $0+$1.document.tables.count },"lists":observations.reduce(0) { $0+$1.document.lists.count },"paragraphs":observations.reduce(0) { $0+$1.document.paragraphs.count }])
            }
            emit(["pageCount":pdf.pageCount,"pages":pages])
        } catch { emit(["error":"extraction_failed"]) }
    }
}
