// Evaluation only: reuse production prompts, schemas, language preparation and validators.
// All inference goes to loopback. Never used or bundled by the application.
import Foundation
import FoundationModels
import PDFKit
import AppKit

// Keep Apple's property order: Ollama's constrained decoder follows JSON property order.
func orderedJSON(_ value: Any, order: [String]? = nil) throws -> String {
    if let object = value as? [String: Any] {
        let preferred = order ?? []
        let keys = preferred.filter { object[$0] != nil } + object.keys.filter { !preferred.contains($0) }.sorted()
        return "{" + (try keys.map { key in
            let childOrder = key == "properties" ? object["x-order"] as? [String] : nil
            return try orderedJSON(key) + ":" + orderedJSON(object[key]!, order: childOrder)
        }).joined(separator: ",") + "}"
    }
    if let array = value as? [Any] { return "[" + (try array.map { try orderedJSON($0) }).joined(separator: ",") + "]" }
    return String(decoding: try JSONSerialization.data(withJSONObject: value, options: [.fragmentsAllowed, .sortedKeys]), as: UTF8.self)
}

func ollamaVisual(path: String, page: Int) -> (Data, String)? {
    let url = URL(fileURLWithPath: path)
    guard path.hasPrefix("/"), (1...50).contains(page),
          let values = try? url.resourceValues(forKeys: [.isRegularFileKey, .isSymbolicLinkKey, .fileSizeKey]),
          values.isRegularFile == true, values.isSymbolicLink != true,
          (values.fileSize ?? Int.max) <= 100 * 1024 * 1024 else { return nil }
    if url.pathExtension.lowercased() == "pdf" {
        guard let document = PDFDocument(url: url), !document.isLocked,
              let pdfPage = document.page(at: page - 1),
              let image = pdfPage.thumbnail(of: NSSize(width: 1600, height: 2200), for: .mediaBox)
                .cgImage(forProposedRect: nil, context: nil, hints: nil),
              let data = NSBitmapImageRep(cgImage: image).representation(using: .png, properties: [:]) else { return nil }
        return (data, "The attached image shows PDF page \(page). Use its layout and heading together with the extracted text. Quotes must still come from the supplied text.")
    }
    guard ["png", "jpg", "jpeg", "heic", "tiff", "tif"].contains(url.pathExtension.lowercased()), page == 1,
          let data = try? Data(contentsOf: url) else { return nil }
    return (data, "The attached image is the original document. Use its layout and heading together with the extracted text. Quotes must still come from the supplied text.")
}

func logOllama(_ entry: [String: Any]) {
    guard let path = ProcessInfo.processInfo.environment["TUCK_OLLAMA_METRICS"],
          let data = try? JSONSerialization.data(withJSONObject: entry, options: [.sortedKeys]) else { return }
    if !FileManager.default.fileExists(atPath: path) { FileManager.default.createFile(atPath: path, contents: nil) }
    guard let handle = FileHandle(forWritingAtPath: path) else { return }
    defer { try? handle.close() }
    _ = try? handle.seekToEnd()
    try? handle.write(contentsOf: data + Data("\n".utf8))
}

@available(macOS 26.0, *)
func generate(model: SystemLanguageModel, instructions: String, prompt: String, schema: GenerationSchema,
              outputTokens: Int = 600, imagePath: String? = nil, imagePage: Int = 1) async throws -> [String: Any] {
    guard let tag = ProcessInfo.processInfo.environment["TUCK_OLLAMA_MODEL"],
          ["qwen3.5:4b", "qwen3.5:9b"].contains(tag) else { throw LocalFailure(code: "model_unavailable") }
    let schemaObject = try JSONSerialization.jsonObject(with: JSONEncoder().encode(schema))
    let schemaJSON = try orderedJSON(schemaObject)
    var text = prompt
    var visual: (Data, String)?
    // Use the same production attachment eligibility and Apple context gate, so models
    // get the same evidence. Apple remains responsible for OCR/language preprocessing.
    if #available(macOS 27.0, *), supportsImages(model), let imagePath,
       let appleVisual = visualPrompt(text: prompt, path: imagePath, page: imagePage) {
        let base = try await model.tokenCount(for: Instructions(instructions)) + model.tokenCount(for: schema)
        let count: Int
        if let exact = try? await model.tokenCount(for: appleVisual) { count = exact }
        else { count = try await model.tokenCount(for: Prompt(prompt)) + 3072 }
        if base + count + outputTokens + 256 < model.contextSize { visual = ollamaVisual(path: imagePath, page: imagePage) }
    }
    if visual == nil, #available(macOS 26.4, *) {
        let count = try await model.tokenCount(for: Instructions(instructions)) + model.tokenCount(for: schema) + model.tokenCount(for: Prompt(prompt))
        guard count + outputTokens + 256 < model.contextSize else { throw LocalFailure(code: "context_limit") }
    }
    var user: [String: Any] = ["role": "user"]
    if let visual { text += "\n" + visual.1; user["images"] = [visual.0.base64EncodedString()] }
    // Ollama's format enforces syntax but does not show descriptions to the model.
    // Render the identical schema as well, preserving the meaning of Apple's guides.
    user["content"] = text + "\n\nReturn JSON matching this schema:\n" + schemaJSON
    let messages: [[String: Any]] = [["role": "system", "content": instructions], user]
    var request = URLRequest(url: URL(string: "http://127.0.0.1:11434/api/chat")!)
    request.httpMethod = "POST"
    request.setValue("application/json", forHTTPHeaderField: "Content-Type")
    request.timeoutInterval = 85
    request.httpBody = Data(("{\"model\":" + (try orderedJSON(tag)) + ",\"messages\":" + (try orderedJSON(messages)) + ",\"format\":" + schemaJSON + ",\"stream\":false,\"think\":false,\"keep_alive\":\"10m\",\"options\":{\"temperature\":0,\"seed\":42,\"num_ctx\":8192,\"num_predict\":\(outputTokens),\"presence_penalty\":0,\"repeat_penalty\":1}}").utf8)
    let config = URLSessionConfiguration.ephemeral
    config.timeoutIntervalForRequest = 85
    config.timeoutIntervalForResource = 85
    let session = URLSession(configuration: config)
    defer { session.invalidateAndCancel() }
    let start = Date()
    do {
        let (data, response) = try await session.data(for: request)
        guard (response as? HTTPURLResponse)?.statusCode == 200,
              let body = try JSONSerialization.jsonObject(with: data) as? [String: Any],
              let message = body["message"] as? [String: Any],
              let content = message["content"] as? String else { throw LocalFailure(code: "ollama_response_error") }
        var metrics: [String: Any] = ["model": tag, "schema": (schemaObject as? [String: Any])?["title"] ?? "unknown", "image": visual != nil, "seconds": Date().timeIntervalSince(start)]
        for key in ["total_duration", "load_duration", "prompt_eval_count", "prompt_eval_duration", "eval_count", "eval_duration", "done_reason"] { metrics[key] = body[key] }
        logOllama(metrics)
        guard body["done_reason"] as? String != "length",
              var result = try JSONSerialization.jsonObject(with: Data(content.utf8)) as? [String: Any] else { throw LocalFailure(code: "invalid_response") }
        if visual != nil { result["_inputMode"] = "image+text" }
        return result
    } catch {
        let code = (error as? URLError)?.code == .timedOut ? "timeout" : errorCode(error)
        logOllama(["model": tag, "error": code, "seconds": Date().timeIntervalSince(start)])
        throw LocalFailure(code: code)
    }
}
