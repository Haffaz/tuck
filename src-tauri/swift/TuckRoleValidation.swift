import Foundation

// Pure decoding and ID validation, shared by production and offline contract tests.
func validateDescriptionReply(_ raw: [String: Any], text: String, actions: [String]) throws -> [String: Any] {
    struct Result: Decodable {
        let action: String
        let page: Int?
        let description: String?
        let quote: String?
        let uncertainty: String?
    }
    let result = try JSONDecoder().decode(Result.self, from: JSONSerialization.data(withJSONObject: raw))
    guard actions.contains(result.action) else { throw LocalFailure(code: "invalid_response") }
    if result.action == "read_page" || result.action == "retry_ocr" {
        guard let page = result.page, (1...50).contains(page) else { throw LocalFailure(code: "invalid_response") }
        return ["action": result.action, "page": page]
    }
    guard result.action == "finish", let description = result.description,
          let quote = result.quote, let uncertainty = result.uncertainty,
          description.count <= 1200, quote.count <= 500, uncertainty.count <= 600,
          quote.isEmpty || (quote.count >= 5 && text.contains(quote)),
          !quote.contains("[Earlier excerpt]"), !quote.contains("[Middle of excerpt omitted;") else {
        throw LocalFailure(code: "invalid_response")
    }
    return ["action": "finish", "description": description,
            "quotes": quote.isEmpty ? [] : [quote], "uncertainty": uncertainty]
}

// Native guided output has three fields. Runtime bookkeeping is supplied by code.
func validateGuidedDescription(_ raw: [String: Any], text: String) throws -> [String: Any] {
    struct Description: Decodable { let description: String; let quote: String; let clear: Bool }
    let value = try JSONDecoder().decode(Description.self, from: JSONSerialization.data(withJSONObject:raw))
    return try validateDescriptionReply([
        "action":"finish", "description":value.description, "quote":value.quote,
        "uncertainty":value.clear ? "" : "The document type or subject is unclear."
    ], text:text, actions:["finish"])
}

struct CategoryChoices {
    let folders: [String]
    let fallback: String
    var allowed: [String] { folders + [fallback] }

    init(folders: [String]) throws {
        guard !folders.isEmpty, folders.allSatisfy({ !$0.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }),
              Set(folders).count == folders.count else { throw LocalFailure(code: "invalid_destinations") }
        self.folders = folders
        var fallback = "Needs input"
        while folders.contains(fallback) { fallback += " (unmatched)" }
        self.fallback = fallback
    }

    func reply(for selected: String, quotes: [String]) throws -> [String: Any] {
        if selected == fallback {
            return ["action": "uncertain", "reason": "No category clearly fits this document. Choose a folder."]
        }
        guard let index = folders.firstIndex(of: selected) else { throw LocalFailure(code: "invalid_destination") }
        // Code carries reader evidence forward. A choice alone does not prove grounding;
        // the Rust coordinator checks the quote and calls the separate verifier.
        return ["action": "finish", "selection": index, "quote": quotes.first ?? "",
                "source_group": "", "reason": "Selected \(selected) from the configured categories."]
    }
}

func validateVerificationReply(_ answer: String) throws -> Bool {
    let verdict = answer.trimmingCharacters(in:.whitespacesAndNewlines).uppercased()
    guard ["YES","NO","YES.","NO."].contains(verdict) else { throw LocalFailure(code:"invalid_response") }
    return verdict == "YES" || verdict == "YES."
}

// Decode the whole response, never extract JSON from refusal/explanatory prose.
func parseModelJSONObject(_ text: String) throws -> [String: Any] {
    guard let value = try? JSONSerialization.jsonObject(with: Data(text.utf8)) as? [String: Any] else {
        throw LocalFailure(code: "invalid_response")
    }
    return value
}

// A generated folder needs a subject. Qualifiers such as "personal" or "official"
// do not make a catch-all useful. User-created names are not constrained by this.
func isGenericCategoryName(_ name: String) -> Bool {
    let generic: Set<String> = ["a", "an", "the", "and", "or", "my", "all", "of", "document", "documents", "documentation", "file", "files", "record", "records", "paper", "papers", "paperwork", "item", "items", "stuff", "general", "other", "misc", "miscellaneous", "personal", "important", "official", "assorted", "uncategorized", "unclassified", "archive", "archives", "archived", "certificate", "certificates", "form", "forms", "letter", "letters"]
    let words = name.lowercased().split(whereSeparator: { !$0.isLetter && !$0.isNumber }).map(String.init)
    return words.isEmpty || words.allSatisfy { generic.contains($0) }
}

func validateCategoryPlan(_ value: [String: Any], maxDepth: Int = 1) throws -> [String: Any] {
    guard (1...2).contains(maxDepth), Set(value.keys) == ["rules"], let rules = value["rules"] as? [[String: Any]],
          (1...6).contains(rules.count) else { throw LocalFailure(code: "invalid_categories") }
    var folders = Set<String>()
    for rule in rules {
        guard Set(rule.keys) == ["folder", "instruction"],
              let folder = rule["folder"] as? String,
              let instruction = rule["instruction"] as? String,
              !folder.isEmpty, folder == folder.trimmingCharacters(in: .whitespacesAndNewlines),
              folder.count <= 120,
              folder.split(separator:"/", omittingEmptySubsequences:false).count <= maxDepth,
              folder.split(separator:"/", omittingEmptySubsequences:false).allSatisfy({
                  !$0.isEmpty && !$0.hasPrefix(".") && $0.count <= 40
                  && String($0) == $0.trimmingCharacters(in:.whitespacesAndNewlines)
                  && !isGenericCategoryName(String($0))
              }),
              !folder.contains("\\"), !folder.contains(":"),
              !folder.unicodeScalars.contains(where: { CharacterSet.controlCharacters.contains($0) }),
              folders.insert(folder.lowercased()).inserted,
              !instruction.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              instruction.count <= 600 else { throw LocalFailure(code: "invalid_categories") }
    }
    return value
}

// One fresh retry for malformed/invalid drafts. Model refusals, timeouts and other
// service errors propagate immediately. Rust revalidates all returned paths/rules.
func categoryPlanWithRetry(maxDepth: Int = 1, generate: (Bool) async throws -> [String: Any]) async throws -> [String: Any] {
    do {
        return try validateCategoryPlan(try await generate(false), maxDepth:maxDepth)
    } catch let error as LocalFailure where ["invalid_response", "invalid_categories"].contains(error.code) {
        return try validateCategoryPlan(try await generate(true), maxDepth:maxDepth)
    }
}
