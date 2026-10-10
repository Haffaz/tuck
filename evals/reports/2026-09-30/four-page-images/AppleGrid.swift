import Foundation
import FoundationModels

struct GridInput: Decodable {
    let instructions: String
    let prompt: String
    let imagePaths: [String]
    let ids: [String]
    let folders: [String]
    let types: [String]
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
            let record = DynamicGenerationSchema(name: "Document", properties: [
                .init(name: "id", description: "Document ID from the manifest, once per document.", schema: DynamicGenerationSchema(type: String.self, guides: [.anyOf(input.ids)])),
                .init(name: "documentType", description: "Actual document type in English, using all supplied pages.", schema: DynamicGenerationSchema(type: String.self, guides: [.anyOf(input.types)])),
                .init(name: "folder", description: "Best matching destination under the supplied filing rules; Needs Review for insufficient or conflicting evidence.", schema: DynamicGenerationSchema(type: String.self, guides: [.anyOf(input.folders)])),
                .init(name: "marker", description: "Copy the four-character Verification marker when present. Otherwise empty. Do not return names, addresses or personal numbers.", schema: DynamicGenerationSchema(type: String.self))
            ])
            let schema = try GenerationSchema(root: DynamicGenerationSchema(name: "BatchDecisions", properties: [
                .init(name: "documents", schema: DynamicGenerationSchema(arrayOf: .init(referenceTo: "Document"), minimumElements: input.ids.count, maximumElements: input.ids.count))
            ]), dependencies: [record])
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
