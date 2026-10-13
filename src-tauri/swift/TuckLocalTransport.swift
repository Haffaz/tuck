import Foundation

// Each helper process serves one request. TaskLocal also keeps nested role calls
// on the selected provider, without global mutable state or an Apple fallback.
enum LocalModelTransport {
    @TaskLocal static var selection = "apple"
    static let choices = ["apple", "gemmaE2b", "gemmaE4b", "custom"]
    static var tag: String { selection == "gemmaE2b" ? "gemma4:e2b-it-q4_K_M" : "gemma4:e4b-it-q4_K_M" }
    static var label: String { selection == "custom" ? "Connected model" : selection == "apple" ? "Apple Intelligence" : (selection == "gemmaE2b" ? "Gemma E2B" : "Gemma E4B") }

    // A fixed loopback endpoint, no proxies, cookies, redirects, cloud tags or downloads.
    private final class NoRedirect: NSObject, URLSessionTaskDelegate, @unchecked Sendable {
        func urlSession(_ session: URLSession, task: URLSessionTask, willPerformHTTPRedirection response: HTTPURLResponse, newRequest request: URLRequest, completionHandler: @escaping @Sendable (URLRequest?) -> Void) {
            completionHandler(nil)
        }
    }
    private static func request(_ path: String, body: [String:Any]? = nil, timeout: Double = 10) async throws -> [String:Any] {
        let config = URLSessionConfiguration.ephemeral
        config.connectionProxyDictionary = [:]
        config.httpCookieStorage = nil
        config.urlCache = nil
        config.timeoutIntervalForRequest = timeout
        config.timeoutIntervalForResource = timeout
        let session = URLSession(configuration:config, delegate:NoRedirect(), delegateQueue:nil)
        defer { session.invalidateAndCancel() }
        var request = URLRequest(url:URL(string:"http://127.0.0.1:11434/api/\(path)")!)
        request.timeoutInterval = timeout
        if let body {
            request.httpMethod = "POST"
            request.setValue("application/json", forHTTPHeaderField:"Content-Type")
            request.httpBody = try JSONSerialization.data(withJSONObject:body, options:[.sortedKeys])
        }
        do {
            let (stream, response) = try await session.bytes(for:request)
            guard let response = response as? HTTPURLResponse, response.statusCode == 200 else { throw LocalFailure(code:"ollama_response_error") }
            guard response.expectedContentLength <= 1024 * 1024 else { throw LocalFailure(code:"invalid_response") }
            var data = Data()
            for try await byte in stream {
                guard data.count < 1024 * 1024 else { throw LocalFailure(code:"invalid_response") }
                data.append(byte)
            }
            return try parseModelJSONObject(String(decoding:data,as:UTF8.self))
        } catch let error as URLError {
            throw LocalFailure(code:error.code == .timedOut ? "timeout" : "ollama_unavailable")
        }
    }
    static func installedDigest() async throws -> String {
        guard selection != "apple", choices.contains(selection) else { throw LocalFailure(code:"model_unavailable") }
        let tags = try await request("tags")
        guard let models = tags["models"] as? [[String:Any]],
              let installed = models.first(where: { $0["name"] as? String == tag }),
              let digest = installed["digest"] as? String, !digest.isEmpty else { throw LocalFailure(code:"model_missing") }
        let metadata = try await request("show", body:["model":tag])
        guard metadata["remote_model"] == nil, metadata["remote_host"] == nil,
              let info = metadata["model_info"] as? [String:Any], !info.isEmpty else { throw LocalFailure(code:"model_not_local") }
        return digest
    }
    static func setupError(_ error: Error) -> String {
        switch (error as? LocalFailure)?.code {
        case "model_missing": return "\(label) is not installed in Ollama. Install \(tag), then check again."
        case "model_not_local": return "Choose a downloaded local Gemma model. Tuck does not support cloud models."
        case "timeout": return "Ollama took too long to respond. Open Ollama and check again."
        default: return "Could not connect to Ollama on this Mac. Open Ollama, then check again."
        }
    }
    static func generate(instructions: String, prompt: String, schema: Any?, outputTokens: Int) async throws -> String {
        if selection == "custom" { return try await ConnectionTransport.generate(instructions:instructions, prompt:prompt, schema:schema, outputTokens:outputTokens) }
        var text = prompt
        if let schema {
            // Show the same schema that constrains decoding, including field descriptions.
            text += "\n\nReturn JSON matching this schema:\n" + String(decoding:try JSONSerialization.data(withJSONObject:schema,options:[.sortedKeys]),as:UTF8.self)
        }
        // Conservative byte bound avoids Ollama silently truncating input. The same
        // native excerpt/page limits apply to both providers; never shorten user rules.
        guard instructions.utf8.count + text.utf8.count + outputTokens + 512 < 16384 else { throw LocalFailure(code:"context_limit") }
        var body: [String:Any] = ["model":tag, "messages":[["role":"system","content":instructions],["role":"user","content":text]],
            "stream":false,"think":false,"keep_alive":"5m",
            "options":["temperature":0,"seed":42,"num_ctx":16384,"num_predict":outputTokens]]
        if let schema { body["format"] = schema }
        let response = try await request("chat",body:body,timeout:85)
        guard response["done"] as? Bool == true, response["done_reason"] as? String != "length",
              let message = response["message"] as? [String:Any], message["tool_calls"] == nil,
              let content = message["content"] as? String, !content.isEmpty else { throw LocalFailure(code:"invalid_response") }
        return content
    }
}
