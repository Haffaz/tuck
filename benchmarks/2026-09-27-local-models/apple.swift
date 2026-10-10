import Foundation
import FoundationModels

let folders = ["Migration/AGENCY", "Migration", "Finances", "Work", "Home", "Insurance", "Health", "Needs Review"]
func decisionSchema() throws -> GenerationSchema {
    let schema = DynamicGenerationSchema(name: "Decision", properties: [
        .init(name: "folder", schema: DynamicGenerationSchema(type: String.self, guides: [.anyOf(folders)])),
        .init(name: "title", schema: DynamicGenerationSchema(type: String.self)),
        .init(name: "reason", schema: DynamicGenerationSchema(type: String.self)),
        .init(name: "confident", schema: DynamicGenerationSchema(type: Bool.self))
    ])
    return try GenerationSchema(root: schema, dependencies: [])
}
struct Input: Decodable { var system: String; var prompt: String }
func emit(_ value: [String: Any]) {
    let data = try! JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
    FileHandle.standardOutput.write(data)
    FileHandle.standardOutput.write(Data([10]))
}
@main struct Benchmark {
    static func main() async {
        let model = SystemLanguageModel.default
        emit(["availability": String(describing: model.availability), "swedish": model.supportsLocale(Locale(identifier: "sv_SE"))])
        guard model.isAvailable else { return }
        while let line = readLine() {
            let started = ContinuousClock.now
            do {
                let input = try JSONDecoder().decode(Input.self, from: Data(line.utf8))
                let session = LanguageModelSession(model: model, instructions: input.system)
                let response = try await session.respond(to: input.prompt, schema: try decisionSchema(), options: GenerationOptions(sampling: .greedy, maximumResponseTokens: 600))
                let elapsed = started.duration(to: .now)
                let seconds = Double(elapsed.components.seconds) + Double(elapsed.components.attoseconds) / 1e18
                let output = try JSONSerialization.jsonObject(with: Data(response.content.jsonString.utf8))
                emit(["seconds":seconds,"output":output])
            } catch {
                let elapsed = started.duration(to: .now)
                emit(["seconds":Double(elapsed.components.seconds)+Double(elapsed.components.attoseconds)/1e18,"error":String(describing:error)])
            }
        }
    }
}
