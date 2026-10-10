import Foundation
import Security
import CryptoKit

struct ModelConnection: Codable, Sendable {
    let kind: String
    let baseUrl: String
    let model: String
    func validatedURL() throws -> URL {
        guard ["ollama", "openai", "anthropic"].contains(kind), !model.trimmingCharacters(in:.whitespacesAndNewlines).isEmpty,
              model.count <= 200, !model.unicodeScalars.contains(where:CharacterSet.controlCharacters.contains),
              let url = URL(string:baseUrl), let host = url.host, !host.isEmpty,
              url.user == nil, url.password == nil, url.query == nil, url.fragment == nil,
              url.scheme == "https" || (url.scheme == "http" && ["localhost", "127.0.0.1", "::1", "[::1]"].contains(host)) else {
            throw LocalFailure(code:"invalid_connection")
        }
        return url
    }
    var account: String {
        get throws {
            let url = try validatedURL().absoluteString.trimmingCharacters(in:CharacterSet(charactersIn:"/"))
            return SHA256.hash(data:Data((kind + "|" + url).utf8)).map { String(format:"%02x",$0) }.joined()
        }
    }
}
enum ConnectionTransport {
    @TaskLocal static var connection: ModelConnection?
    @TaskLocal static var apiKey: String?
    private static func keyQuery(_ c: ModelConnection) throws -> [String:Any] {
        [kSecClass as String:kSecClassGenericPassword, kSecAttrService as String:"app.tuck.model-connections", kSecAttrAccount as String:try c.account]
    }
    static func saveKey(_ key: String, for c: ModelConnection) throws {
        guard !key.isEmpty, key.utf8.count <= 8192, !key.contains("\n"), !key.contains("\r") else { throw LocalFailure(code:"invalid_key") }
        var query = try keyQuery(c)
        let data = Data(key.utf8)
        let update = SecItemUpdate(query as CFDictionary, [kSecValueData as String:data] as CFDictionary)
        if update == errSecItemNotFound {
            query[kSecValueData as String] = data
            query[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
            guard SecItemAdd(query as CFDictionary, nil) == errSecSuccess else { throw LocalFailure(code:"keychain_error") }
        } else if update != errSecSuccess { throw LocalFailure(code:"keychain_error") }
    }
    static func savedKey(_ c: ModelConnection) throws -> String? {
        var query = try keyQuery(c)
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        var value: CFTypeRef?
        let result = SecItemCopyMatching(query as CFDictionary, &value)
        if result == errSecItemNotFound { return nil }
        guard result == errSecSuccess, let data = value as? Data else { throw LocalFailure(code:"keychain_error") }
        return String(data:data,encoding:.utf8)
    }
    private final class NoRedirect: NSObject, URLSessionTaskDelegate, @unchecked Sendable {
        func urlSession(_ session: URLSession, task: URLSessionTask, willPerformHTTPRedirection response: HTTPURLResponse, newRequest request: URLRequest, completionHandler: @escaping @Sendable (URLRequest?) -> Void) { completionHandler(nil) }
    }
    static func request(_ path: String, body: [String:Any]? = nil) async throws -> [String:Any] {
        guard let c = connection else { throw LocalFailure(code:"invalid_connection") }
        let url = try c.validatedURL().appendingPathComponent(path)
        let key = try apiKey.flatMap { $0.isEmpty ? nil : $0 } ?? savedKey(c)
        let config = URLSessionConfiguration.ephemeral
        config.httpCookieStorage = nil; config.urlCache = nil; config.connectionProxyDictionary = [:]
        config.timeoutIntervalForRequest = 85; config.timeoutIntervalForResource = 85
        let session = URLSession(configuration:config, delegate:NoRedirect(), delegateQueue:nil)
        defer { session.invalidateAndCancel() }
        var req = URLRequest(url:url)
        req.setValue("application/json", forHTTPHeaderField:"Content-Type")
        if let key {
            guard !key.contains("\r"), !key.contains("\n"), key.utf8.count <= 8192 else { throw LocalFailure(code:"invalid_key") }
            req.setValue(c.kind == "anthropic" ? key : "Bearer \(key)", forHTTPHeaderField:c.kind == "anthropic" ? "x-api-key" : "Authorization")
        }
        if c.kind == "anthropic" { req.setValue("2023-06-01",forHTTPHeaderField:"anthropic-version") }
        if let body { req.httpMethod = "POST"; req.httpBody = try JSONSerialization.data(withJSONObject:body) }
        let (stream, response) = try await session.bytes(for:req)
        guard let http = response as? HTTPURLResponse else { throw LocalFailure(code:"connection_error") }
        guard (200...299).contains(http.statusCode) else {
            throw LocalFailure(code:[401,403].contains(http.statusCode) ? "authentication_failed" : http.statusCode == 429 ? "rate_limited" : "provider_http_\(http.statusCode)")
        }
        guard response.expectedContentLength <= 1024 * 1024 else { throw LocalFailure(code:"invalid_response") }
        var data = Data()
        for try await byte in stream {
            guard data.count < 1024 * 1024 else { throw LocalFailure(code:"invalid_response") }
            data.append(byte)
        }
        return try parseModelJSONObject(String(decoding:data,as:UTF8.self))
    }
    static func models() async throws -> [String] {
        guard let c = connection else { throw LocalFailure(code:"invalid_connection") }
        let result = try await request(c.kind == "ollama" ? "api/tags" : c.kind == "anthropic" ? "v1/models" : "models")
        guard let values = result[c.kind == "ollama" ? "models" : "data"] as? [[String:Any]] else { throw LocalFailure(code:"invalid_response") }
        return values.compactMap { $0[c.kind == "ollama" ? "name" : "id"] as? String }.sorted()
    }
    static func generate(instructions: String, prompt: String, schema: Any?, outputTokens: Int) async throws -> String {
        guard let c = connection else { throw LocalFailure(code:"invalid_connection") }
        var text = prompt
        if let schema { text += "\nReturn only JSON matching this schema:\n" + String(decoding:try JSONSerialization.data(withJSONObject:schema),as:UTF8.self) }
        guard instructions.utf8.count + text.utf8.count + outputTokens + 512 < 16384 else { throw LocalFailure(code:"context_limit") }
        let messages = [["role":"system","content":instructions],["role":"user","content":text]]
        var body: [String:Any] = ["model":c.model, "messages":messages, "stream":false]
        let path: String
        switch c.kind {
        case "ollama":
            path = "api/chat"
            body["think"] = false
            body["options"] = ["temperature":0,"num_ctx":16384,"num_predict":outputTokens]
            if let schema { body["format"] = schema }
        case "anthropic":
            path = "v1/messages"
            body["system"] = instructions
            body["messages"] = [["role":"user","content":text]]
            body["max_tokens"] = max(outputTokens, 256)
        default:
            path = "chat/completions"
            let official = try c.validatedURL().host == "api.openai.com"
            body[official ? "max_completion_tokens" : "max_tokens"] = max(outputTokens, official ? 4096 : 256)
        }
        let response = try await request(path, body:body)
        let result: String
        switch c.kind {
        case "ollama":
            guard response["done"] as? Bool == true, response["done_reason"] as? String != "length",
                  let msg = response["message"] as? [String:Any], msg["tool_calls"] == nil,
                  let content = msg["content"] as? String else { throw LocalFailure(code:"invalid_response") }
            result = content
        case "anthropic":
            guard response["stop_reason"] as? String == "end_turn", let blocks = response["content"] as? [[String:Any]],
                  blocks.allSatisfy({ $0["type"] as? String == "text" }) else { throw LocalFailure(code:"invalid_response") }
            result = blocks.compactMap { $0["text"] as? String }.joined(separator:"\n")
        default:
            guard let choices = response["choices"] as? [[String:Any]], let choice = choices.first,
                  choice["finish_reason"] as? String == "stop", let msg = choice["message"] as? [String:Any],
                  msg["tool_calls"] == nil, msg["refusal"] == nil || msg["refusal"] is NSNull,
                  let content = msg["content"] as? String else { throw LocalFailure(code:"invalid_response") }
            result = content
        }
        guard !result.trimmingCharacters(in:.whitespacesAndNewlines).isEmpty else { throw LocalFailure(code:"invalid_response") }
        return result
    }
    static func message(_ error: Error) -> String {
        switch (error as? LocalFailure)?.code {
        case "authentication_failed": return "The provider rejected the API key. Check the key and endpoint."
        case "rate_limited": return "The provider’s rate limit was reached. Try again later."
        case "keychain_error": return "Tuck could not access macOS Keychain. Allow access and try again."
        case "invalid_connection": return "Enter a model ID and a valid HTTPS API URL, or HTTP for a server on this Mac."
        default: return "The connection could not complete this request (\(errorCode(error))). Check the endpoint, model ID and server."
        }
    }
}
