import Foundation

/// Which model serves this helper process: Apple Intelligence on device, or the user's
/// custom connection (Ollama, OpenAI-compatible or Anthropic; see TuckConnections.swift).
/// Each process serves one request, so a TaskLocal keeps nested calls on that choice.
enum ModelChoice {
    @TaskLocal static var selection = "apple"
    static let choices = ["apple", "custom"]
    static var label: String { selection == "custom" ? "Connected model" : "Apple Intelligence" }

    /// Text generation through the custom connection.
    static func generate(instructions: String, prompt: String, schema: Any?, outputTokens: Int) async throws -> String {
        try await ConnectionTransport.generate(instructions: instructions, prompt: prompt, schema: schema, outputTokens: outputTokens)
    }
}
