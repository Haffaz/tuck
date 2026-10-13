import Foundation
import FoundationModels

func emit(_ value: [String: Any]) {
    guard let data = try? JSONSerialization.data(withJSONObject: value, options: [.sortedKeys]) else { return }
    FileHandle.standardOutput.write(data)
}
struct LocalFailure: Error { let code: String }
func parseModelJSONObject(_ text: String) throws -> [String: Any] {
    guard let value = try? JSONSerialization.jsonObject(with: Data(text.utf8)) as? [String: Any] else {
        throw LocalFailure(code: "invalid_response")
    }
    return value
}

// One helper handles one serial operation. This budget counts actual generations,
// including retries, rather than just top-level operations.
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
              outputTokens: Int = 600) async throws -> [String: Any] {
    try await GenerationBudget.shared.take()
    if LocalModelTransport.selection != "apple" {
        let schemaObject = try JSONSerialization.jsonObject(with: JSONEncoder().encode(schema))
        return try parseModelJSONObject(try await LocalModelTransport.generate(instructions: instructions, prompt: prompt, schema: schemaObject, outputTokens: outputTokens))
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
