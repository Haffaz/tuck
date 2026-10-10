import Foundation
import FoundationModels
import PDFKit
import AppKit

func emit(_ value: [String: Any]) {
    guard let data = try? JSONSerialization.data(withJSONObject: value, options: [.sortedKeys]) else { return }
    FileHandle.standardOutput.write(data)
}
func normalized(_ text: String) -> String {
    text.split(whereSeparator: { $0.isWhitespace }).joined(separator: " ").lowercased()
}
func containsQuote(_ quote: String, in evidence: String) -> Bool {
    let quote = normalized(quote)
    return quote.count >= 5 && normalized(evidence).contains(quote)
}
let documentTypes = [
    "Residence Permit Card", "Residence Permit", "Visa", "Passport", "Identity Card", "Driving Licence", "Birth Certificate",
    "Immigration Application", "Immigration Decision", "Bank Statement", "Tax Return", "Tax Assessment",
    "Payslip", "Employment Contract", "Employment Certificate", "Rental Agreement", "Electricity Bill",
    "Utility Bill", "Home Insurance Policy", "Travel Insurance Policy", "Insurance Policy",
    "Insurance Certificate", "Medical Certificate", "Medical Report", "Prescription", "Invoice", "Receipt",
    "Education Certificate", "Other", "Unknown"
]
func templateTitle(_ type: String) -> String {
    documentTypes.contains(type) && type != "Other" && type != "Unknown" ? type : ""
}
struct LocalFailure: Error { let code: String }

// One helper handles one serial operation. This budget counts actual generations,
// including retries and tournament rounds, rather than just top-level roles.
actor GenerationBudget {
    static let shared = GenerationBudget()
    private var calls = 0
    func take() throws {
        guard calls < 32 else { throw LocalFailure(code:"call_limit") }
        calls += 1
    }
}


@available(macOS 26.0, *)
func errorCode(_ error: Error) -> String {
    if let error = error as? LocalFailure { return error.code }
    if error is DecodingError { return "invalid_response" }
    if #available(macOS 27.0, *), let error = error as? LanguageModelError {
        switch error {
        case .contextSizeExceeded: return "context_limit"
        case .rateLimited: return "busy"
        case .guardrailViolation: return "guardrail"
        case .refusal: return "refused"
        case .unsupportedLanguageOrLocale: return "unsupported_language"
        case .timeout: return "timeout"
        case .unsupportedCapability: return "unsupported_capability"
        case .unsupportedTranscriptContent: return "unsupported_attachment"
        case .unsupportedGenerationGuide: return "unsupported_schema"
        default: return "generation_failed"
        }
    }
    // Retain compatibility with macOS 26's error types. Never serialize descriptions:
    // Apple's diagnostic descriptions can contain prompt/document content.
    if let error = error as? LanguageModelSession.GenerationError {
        switch error {
        case .exceededContextWindowSize: return "context_limit"
        case .assetsUnavailable: return "model_unavailable"
        case .guardrailViolation: return "guardrail"
        case .unsupportedLanguageOrLocale: return "unsupported_language"
        case .decodingFailure: return "invalid_response"
        case .rateLimited: return "busy"
        case .refusal: return "refused"
        default: return "generation_failed"
        }
    }
    if #available(macOS 27.0, *), error is SystemLanguageModel.Error { return "model_unavailable" }
    return "generation_failed"
}
@available(macOS 26.0, *)
func availabilityError(_ model: SystemLanguageModel) -> String? {
    switch model.availability {
    case .available: return nil
    case .unavailable(let reason):
        switch reason {
        case .deviceNotEligible: return "This Mac does not support Apple Intelligence. Choose a folder manually."
        case .appleIntelligenceNotEnabled: return "Enable Apple Intelligence in System Settings, then check again."
        case .modelNotReady: return "Apple's on-device model is downloading or not ready. Try again when setup completes."
        @unknown default: return "Apple Intelligence is unavailable. Check Apple Intelligence in System Settings."
        }
    }
}

@available(macOS 26.0, *)
func makeSchema(_ name: String, _ properties: [DynamicGenerationSchema.Property]) throws -> GenerationSchema {
    try GenerationSchema(root: DynamicGenerationSchema(name: name, properties: properties), dependencies: [])
}
@available(macOS 26.0, *)
func stringProperty(_ name: String, _ description: String) -> DynamicGenerationSchema.Property {
    .init(name: name, description: description, schema: DynamicGenerationSchema(type: String.self))
}
@available(macOS 26.0, *)
func generate(model: SystemLanguageModel, instructions: String, prompt: String, schema: GenerationSchema,
              outputTokens: Int = 600, imagePath: String? = nil, imagePage: Int = 1) async throws -> [String: Any] {
    try await GenerationBudget.shared.take()
    if LocalModelTransport.selection != "apple" {
        let schemaObject = try JSONSerialization.jsonObject(with: JSONEncoder().encode(schema))
        return try parseModelJSONObject(try await LocalModelTransport.generate(instructions: instructions, prompt: prompt, schema: schemaObject, outputTokens: outputTokens))
    }
    if #available(macOS 27.0, *), supportsImages(model), let imagePath,
       let visual = visualPrompt(text: prompt, path: imagePath, page: imagePage) {
        let base = try await model.tokenCount(for: Instructions(instructions)) + model.tokenCount(for: schema)
        // SDK 27 currently rejects image prompts in tokenCount on some Macs even when
        // generation supports them. Reserve a bounded image allowance, then let the
        // framework enforce its actual context limit. Never drop user rules.
        let promptCount: Int
        if let exact = try? await model.tokenCount(for: visual) { promptCount = exact }
        else { promptCount = try await model.tokenCount(for: Prompt(prompt)) + 3072 }
        if base + promptCount + outputTokens + 256 < model.contextSize {
            do {
                let session = LanguageModelSession(model: model, instructions: instructions)
                let response = try await session.respond(to: visual, schema: schema,
                    options: GenerationOptions(samplingMode: .greedy, maximumResponseTokens: outputTokens))
                guard var result = try JSONSerialization.jsonObject(with: Data(response.content.jsonString.utf8)) as? [String: Any] else { throw LocalFailure(code: "invalid_response") }
                result["_inputMode"] = "image+text"
                return result
            } catch {
                // Refusals/guardrails propagate. Only attachment/context compatibility
                // problems may use the established text reader instead.
                guard ["context_limit", "unsupported_capability", "unsupported_attachment"].contains(errorCode(error)) else { throw error }
                // The failed attachment attempt already consumed one request.
                try await GenerationBudget.shared.take()
            }
        }
    }
    // Count the exact instructions + structured schema + prompt. Leave room for the answer
    // and framework overhead. Never truncate user preferences or silently drop a destination.
    if #available(macOS 26.4, *) {
        let count = try await model.tokenCount(for: Instructions(instructions))
            + model.tokenCount(for: schema) + model.tokenCount(for: Prompt(prompt))
        guard count + outputTokens + 256 < model.contextSize else { throw LocalFailure(code: "context_limit") }
    } else {
        // Conservative fallback; the framework remains the final context-limit authority.
        guard instructions.utf8.count + prompt.utf8.count + schema.debugDescription.utf8.count + outputTokens + 256 < 4096 else {
            throw LocalFailure(code: "context_limit")
        }
    }
    let session = LanguageModelSession(model: model, instructions: instructions)
    let response = try await session.respond(to: prompt, schema: schema,
        options: GenerationOptions(sampling: .greedy, maximumResponseTokens: outputTokens))
    guard let result = try JSONSerialization.jsonObject(with: Data(response.content.jsonString.utf8)) as? [String: Any] else {
        throw LocalFailure(code: "invalid_response")
    }
    return result
}

// A second opinion is conditional and cannot grant permission to move files.
func needsSuggestionCheck(type: String, grounded: Bool, sourceAssociation: Bool, toolCount: Int, fit: String) -> Bool {
    sourceAssociation || !grounded || type == "Other" || toolCount > 0 || fit != "clear"
}
func checkedSuggestion(_ check: [String: Any], title: String, evidence: String, sourceAssociation: Bool) -> (folderOK: Bool, titleOK: Bool) {
    let result = check["result"] as? String ?? ""
    let folderOK = ["supported", "unsupported_name"].contains(result)
        && (sourceAssociation || containsQuote(check["evidenceQuote"] as? String ?? "", in: evidence))
    return (folderOK, title.isEmpty || result == "supported")
}
func duplicateRule(_ instruction: String, among others: [String]) -> Bool {
    !normalized(instruction).isEmpty && others.contains { normalized($0) == normalized(instruction) }
}

func sameRuleScope(_ first: String, _ second: String) -> Bool {
    first.split(separator: "/").dropLast().joined(separator: "/") == second.split(separator: "/").dropLast().joined(separator: "/")
}

// The model chooses small numeric labels; only this lookup can produce a real folder path.
func destinationIndexes(_ folders: [String]) -> String {
    folders.enumerated().map { "\($0.offset): \($0.element)" }.joined(separator: "\n")
}
func selectedFolder(_ result: [String: Any], allowed: [String]) throws -> String {
    struct Selection: Decodable { let selection: Int }
    guard let data = try? JSONSerialization.data(withJSONObject: result),
          let selected = try? JSONDecoder().decode(Selection.self, from: data),
          allowed.indices.contains(selected.selection) else { throw LocalFailure(code: "invalid_response") }
    return allowed[selected.selection]
}

// Verify the model's association using a real shared name. This does not select a folder.
func sharedSourceName(_ sources: [String], rule: String) -> String? {
    let generic: Set<String> = ["documents", "files", "incoming", "supporting", "supporting files", "application", "applications", "folder", "folders", "source", "scans", "scan", "pdf"]
    let words: (String) -> [String] = { $0.lowercased().split(whereSeparator: { !$0.isLetter && !$0.isNumber }).map(String.init) }
    let ruleWords = words(rule)
    for source in sources {
        let parts = words(source)
        // Prefer the full name; then allow a distinctive name in a labelled source folder.
        let candidates = [parts.joined(separator: " ")] + parts
        for candidate in candidates where candidate.count >= 3 && candidate.contains(where: { $0.isLetter }) && !generic.contains(candidate) {
            let tokens = words(candidate)
            guard tokens.contains(where: { !generic.contains($0) && $0.contains(where: { $0.isLetter }) }),
                  tokens.count <= ruleWords.count else { continue }
            if (0...(ruleWords.count - tokens.count)).contains(where: { Array(ruleWords[$0..<($0 + tokens.count)]) == tokens }) { return candidate }
        }
    }
    return nil
}

// Keep shared naming preferences separate from the topic rules being compared.
func topicRules(_ rules: String, shared: String) -> String {
    guard !shared.isEmpty, rules.hasPrefix(shared) else { return rules }
    return String(rules.dropFirst(shared.count)).trimmingCharacters(in: .whitespacesAndNewlines)
}
// A verifier chooses a destination, not a supported/unsupported token with an overlapping name.
func checkedDestination(_ check: [String: Any], candidate: String, folders: [String], title: String, evidence: String) throws -> (folderOK: Bool, titleOK: Bool, reason: String) {
    let selected = try selectedFolder(["selection": check["folderSelection"] ?? NSNull()], allowed: folders)
    if selected == "Needs Review" { return (false, false, "The independent check could not identify one matching folder from the document evidence.") }
    if selected != candidate { return (false, false, "The two checks selected different folders. Please choose the destination.") }
    guard containsQuote(check["evidenceQuote"] as? String ?? "", in: evidence) else {
        return (false, false, "The folder check did not provide a verifiable supporting passage. Please choose the destination.")
    }
    let name = try selectedFolder(["selection": check["nameSelection"] ?? NSNull()], allowed: ["keep_original", "supported"])
    return (true, title.isEmpty || name == "supported", "")
}

@available(macOS 26.0, *)
func supportsImages(_ model: SystemLanguageModel) -> Bool {
    if #available(macOS 27.0, *) { return model.capabilities.contains(.vision) }
    return false
}
@available(macOS 27.0, *)
func visualPrompt(text: String, path: String, page: Int) -> Prompt? {
    let url = URL(fileURLWithPath: path)
    guard path.hasPrefix("/"), (1...50).contains(page),
          let values = try? url.resourceValues(forKeys: [.isRegularFileKey, .isSymbolicLinkKey, .fileSizeKey]),
          values.isRegularFile == true, values.isSymbolicLink != true,
          (values.fileSize ?? Int.max) <= 100 * 1024 * 1024 else { return nil }
    if url.pathExtension.lowercased() == "pdf" {
        guard let document = PDFDocument(url: url), !document.isLocked,
              let pdfPage = document.page(at: page - 1) else { return nil }
        let thumbnail = pdfPage.thumbnail(of: NSSize(width: 1600, height: 2200), for: .mediaBox)
        guard let image = thumbnail.cgImage(forProposedRect: nil, context: nil, hints: nil) else { return nil }
        return Prompt {
            text
            "The attached image shows PDF page \(page). Use its layout and heading together with the extracted text. Quotes must still come from the supplied text."
            Attachment(image)
        }
    }
    guard ["png", "jpg", "jpeg", "heic", "tiff", "tif"].contains(url.pathExtension.lowercased()), page == 1 else { return nil }
    return Prompt {
        text
        "The attached image is the original document. Use its layout and heading together with the extracted text. Quotes must still come from the supplied text."
        Attachment(imageURL: url)
    }
}

// String generation uses the caller-selected model policy. The JSON boundary is strict; refusals are never reinterpreted as answers.
@available(macOS 26.0, *)
func generateText(model: SystemLanguageModel, instructions: String, prompt: String, outputTokens: Int = 700) async throws -> String {
    try await GenerationBudget.shared.take()
    if LocalModelTransport.selection != "apple" {
        return try await LocalModelTransport.generate(instructions: instructions, prompt: prompt, schema: nil, outputTokens: outputTokens)
    }
    if #available(macOS 26.4, *) {
        let tokens = try await model.tokenCount(for: Instructions(instructions)) + model.tokenCount(for: Prompt(prompt))
        guard tokens + outputTokens + 256 < model.contextSize else { throw LocalFailure(code: "context_limit") }
    }
    else {
        guard instructions.utf8.count + prompt.utf8.count + outputTokens + 256 < 4096 else { throw LocalFailure(code: "context_limit") }
    }
    let session = LanguageModelSession(model: model, instructions: instructions)
    let response = try await session.respond(to: prompt, options: GenerationOptions(sampling: .greedy, maximumResponseTokens: outputTokens))
    return response.content
}
@available(macOS 26.0, *)
func generateJSON(model: SystemLanguageModel, instructions: String, prompt: String, outputTokens: Int = 700, schema: [String:Any]) async throws -> [String: Any] {
    if LocalModelTransport.selection != "apple" {
        try await GenerationBudget.shared.take()
        return try parseModelJSONObject(try await LocalModelTransport.generate(instructions:instructions, prompt:prompt, schema:schema, outputTokens:outputTokens))
    }
    let text = try await generateText(model:model, instructions:instructions, prompt:prompt, outputTokens:outputTokens)
    return try parseModelJSONObject(text)
}

// One-field guided matcher: read the typed property directly, without parsing model JSON.
@available(macOS 26.0, *)
func generateCategory(model: SystemLanguageModel, instructions: String, prompt: String, choices: [String]) async throws -> String {
    if LocalModelTransport.selection != "apple" {
        try await GenerationBudget.shared.take()
        let schema: [String:Any] = ["type":"object", "properties":["category":["type":"string","enum":choices]],"required":["category"],"additionalProperties":false]
        let result = try parseModelJSONObject(try await LocalModelTransport.generate(instructions:instructions, prompt:prompt, schema:schema, outputTokens:160))
        guard let category = result["category"] as? String, choices.contains(category) else { throw LocalFailure(code:"invalid_destination") }
        return category
    }
    let schema=try makeSchema("FilingBranch",[
        .init(name:"category",description:"The best supported branch, or the supplied noMatch choice.",schema:DynamicGenerationSchema(type:String.self,guides:[.anyOf(choices)]))
    ])
    let result=try await generate(model:model,instructions:instructions,prompt:prompt,schema:schema,outputTokens:160)
    guard let category=result["category"] as? String,choices.contains(category) else {throw LocalFailure(code:"invalid_destination")}
    return category
}
