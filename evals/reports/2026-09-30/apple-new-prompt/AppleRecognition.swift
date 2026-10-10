import Foundation
import FoundationModels

struct GridInput: Decodable {
    let instructions: String
    let prompt: String
    let imagePaths: [String]
}
@main struct GridBenchmark {
    static func emit(_ value: [String: Any]) {
        if let data = try? JSONSerialization.data(withJSONObject: value, options: [.sortedKeys]) { FileHandle.standardOutput.write(data) }
    }
    static func main() async {
        guard #available(macOS 27.0, *) else { emit(["error":"macOS_27_required"]); return }
        let model = SystemLanguageModel.default
        if CommandLine.arguments.contains("--check") {
            emit(["available": availabilityError(model) == nil, "vision":supportsImages(model), "context":model.contextSize]); return
        }
        do {
            let input = try JSONDecoder().decode(GridInput.self, from: FileHandle.standardInput.readDataToEndOfFile())
            let schema = try GenerationSchema(root: DynamicGenerationSchema(name: "DocumentType", properties: [
                .init(name: "documentType", schema: DynamicGenerationSchema(type: String.self))
            ]), dependencies: [])
            if CommandLine.arguments.contains("--schema") {
                FileHandle.standardOutput.write(try JSONEncoder().encode(schema)); return
            }
            if let error = availabilityError(model) { emit(["error":error]); return }
            if !input.imagePaths.isEmpty && !supportsImages(model) { emit(["error":"vision_unavailable"]); return }
            let prompt = Prompt {
                input.prompt
                for path in input.imagePaths { Attachment(imageURL: URL(fileURLWithPath:path)) }
            }
            let start = Date()
            let session = LanguageModelSession(model:model, instructions:input.instructions)
            let result = try await session.respond(to:prompt, schema:schema,
                options:GenerationOptions(samplingMode:.greedy, maximumResponseTokens:700))
            let parsed = try JSONSerialization.jsonObject(with:Data(result.content.jsonString.utf8))
            emit(["result":parsed,"modelSeconds":Date().timeIntervalSince(start),"imagesSent":input.imagePaths.count])
        } catch { emit(["error":errorCode(error)]) }
    }
}
