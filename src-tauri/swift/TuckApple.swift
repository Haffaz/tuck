import Foundation
import FoundationModels

@main struct TuckApple {
    static func main() async {
        guard #available(macOS 26.0, *) else { emit(["error": "Apple Intelligence requires macOS 26 or later."]); return }
        let data = CommandLine.arguments.contains("--check") ? Data() : FileHandle.standardInput.readDataToEndOfFile()
        let request = (try? JSONSerialization.jsonObject(with:data)) as? [String:Any]
        let selection = request?["model"] as? String ?? "apple"
        guard LocalModelTransport.choices.contains(selection) else { emit(["error":"Unsupported local model. Choose a model in Preferences."]); return }
        if selection == "custom" {
            do {
                guard let raw = request?["connection"] else { throw LocalFailure(code:"invalid_connection") }
                let c = try JSONDecoder().decode(ModelConnection.self, from:JSONSerialization.data(withJSONObject:raw))
                _ = try c.validatedURL()
                if request?["operation"] as? String == "save_key" {
                    try ConnectionTransport.saveKey(request?["apiKey"] as? String ?? "", for:c)
                    emit(["status":"Key saved in macOS Keychain"]); return
                }
                await ConnectionTransport.$connection.withValue(c) {
                    await ConnectionTransport.$apiKey.withValue(request?["apiKey"] as? String) {
                        await LocalModelTransport.$selection.withValue(selection) { await run(request:request ?? [:]) }
                    }
                }
            } catch { emit(["error":ConnectionTransport.message(error)]) }
        } else { await LocalModelTransport.$selection.withValue(selection) { await run(request:request ?? [:]) } }
    }
    @available(macOS 26.0, *)
    static func run(request: [String:Any]) async {
        let operation = request["operation"] as? String
        // Apple's documented mode for transforming user-provided source text.
        // String generation skips runtime guardrail checks. Model refusals,
        // strict response validation and Rust file checks remain.
        // Rule suggestions keep defaults.
        let transformsDocument = ["card", "name_group", "group_subjects", "merge_names", "choose_folder"].contains(operation ?? "")
        let model = transformsDocument ? SystemLanguageModel(guardrails:.permissiveContentTransformations) : SystemLanguageModel.default
        // On-device only; no cloud fallback.
        if LocalModelTransport.selection == "apple", let error = availabilityError(model) { emit(["error": error]); return }
        if LocalModelTransport.selection == "custom" {
            do {
                if operation == "models" { emit(["models":try await ConnectionTransport.models()]); return }
                if operation == "status" {
                    _ = try await ConnectionTransport.generate(instructions:"Reply briefly.", prompt:"Say OK.", schema:nil, outputTokens:32)
                    emit(["status":"Connection ready. The selected model responded."]); return
                }
            } catch { emit(["error":ConnectionTransport.message(error)]); return }
        }
        if LocalModelTransport.selection != "apple" && LocalModelTransport.selection != "custom" {
            do {
                let digest = try await LocalModelTransport.installedDigest()
                if operation == "identity" { emit(["digest":digest]); return }
                if operation == "status" {
                    _ = try await LocalModelTransport.generate(instructions:"Reply briefly.", prompt:"Say OK.", schema:nil, outputTokens:8)
                    emit(["status":"\(LocalModelTransport.label) is ready — on this Mac", "digest":digest]); return
                }
            } catch { emit(["error": LocalModelTransport.setupError(error)]); return }
        }
        if operation == "status" {
            do {
                _ = try await generateText(model:model, instructions:"Reply briefly.", prompt:"Say OK.", outputTokens:8)
            } catch {
                emit(["error":"Apple Intelligence is enabled but could not respond (\(errorCode(error))). Try again or choose another local model."]); return
            }
        }
        if CommandLine.arguments.contains("--check") || operation == "status" {
            emit(["status": "Apple Intelligence is ready — on-device", "contextTokens": model.contextSize]); return
        }
        if let response = await handleCardOperation(request, model:model) {
            emit(response); return
        }
        if operation == "rule" {
            do {
                let schema = try makeSchema("FolderDescription", [stringProperty("instruction", "One short plain-language sentence describing what belongs in this folder. At most 40 words.")])
                let result = try await generate(model: model,
                    instructions: "Help a person describe a document folder. Suggest a short filing preference using only the supplied folder name and shared rules. Do not invent organizations, identities, exclusions or document facts. An unfamiliar name should get a broad description, not invented details. Return a draft for the user to edit.",
                    prompt: "Folder: \(request["folder"] as? String ?? "")\nShared rules: \(request["context"] as? String ?? "")", schema: schema, outputTokens: 200)
                emit(result)
            } catch { emit(["error": "Could not suggest a description (\(errorCode(error))). Write a short sentence or try again."]) }
            return
        }
        emit(["error": "Unsupported operation."])
    }
}
