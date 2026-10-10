// swiftc -parse-as-library src-tauri/swift/TuckRoleValidation.swift evals/role-contract-tests.swift -o /tmp/tuck-contract-tests
import Foundation
// The production helper defines the same error in TuckModelSupport.swift.
struct LocalFailure: Error { let code: String }
@main struct ContractTests {
 static func rejects(_ block: () throws -> Void) {
  do { try block(); fatalError("Invalid reply was accepted") } catch { }
 }
 static func main() async throws {
  let source = ["Home insurance policy"]
  let description: [String: Any] = ["action":"finish", "description":"Insurance policy", "quote":source[0], "uncertainty":""]
  let read = try validateDescriptionReply(description, text: source[0], actions:["finish"])
  precondition(read["quotes"] as? [String] == source)
  for quote in ["Invented insurance policy", "short", "[Earlier excerpt]"] {
    var invalid = description; invalid["quote"] = quote
    rejects { _ = try validateDescriptionReply(invalid, text:source[0], actions:["finish"]) }
  }
  for page in [0, -1, 51] {
    rejects { _ = try validateDescriptionReply(["action":"read_page", "page":page], text:"", actions:["read_page"]) }
  }
  rejects { _ = try validateDescriptionReply(["action":"read_page", "page":2], text:"", actions:["finish"]) }
  let extra = try validateDescriptionReply(["action":"read_page", "page":2], text:"", actions:["read_page"])
  precondition(extra["page"] as? Int == 2 && extra["description"] == nil)
  let unreadable = try validateDescriptionReply(["action":"finish","description":"Unreadable document","quote":"","uncertainty":"No readable subject"], text:"", actions:["finish"])
  precondition((unreadable["quotes"] as? [String])?.isEmpty == true)
  let guided = try validateGuidedDescription(["description":"Insurance policy","quote":source[0],"clear":true], text:source[0])
  precondition(guided["action"] as? String == "finish" && guided["uncertainty"] as? String == "")
  for invalid: Any in ["true", 1, NSNull()] {
    rejects { _ = try validateGuidedDescription(["description":"Insurance policy","quote":source[0],"clear":invalid], text:source[0]) }
  }
  rejects { _ = try validateGuidedDescription(["description":"Insurance policy","quote":source[0]], text:source[0]) }
  rejects { _ = try validateGuidedDescription(["description":"Insurance policy","quote":"[Earlier excerpt]","clear":true], text:"[Earlier excerpt]") }
  let choices = try CategoryChoices(folders: ["Insurance", "Banking & Taxes", "Needs input", "Needs input (unmatched)"])
  let mapped = try choices.reply(for: "Insurance", quotes: source)
  precondition(mapped["selection"] as? Int == 0 && mapped["quote"] as? String == source[0])
  precondition(mapped["source_group"] as? String == "")
  let actualFolder = try choices.reply(for: "Needs input", quotes: source)
  precondition(actualFolder["action"] as? String == "finish" && actualFolder["selection"] as? Int == 2)
  let fallback = try choices.reply(for: choices.fallback, quotes: source)
  precondition(fallback["action"] as? String == "uncertain" && fallback["selection"] == nil)
  let noEvidence = try choices.reply(for: "Insurance", quotes: [])
  precondition(noEvidence["quote"] as? String == "" && noEvidence["source_group"] as? String == "")
  for invalid in ["../Insurance", "insurance", "Unknown", "", "move_file"] {
   rejects { _ = try choices.reply(for: invalid, quotes: source) }
  }
  for invalid in [[], ["Insurance", "Insurance"], [""], ["  "]] {
   rejects { _ = try CategoryChoices(folders: invalid) }
  }
  let yes = try validateVerificationReply("YES."); precondition(yes)
  let no = try validateVerificationReply("NO"); precondition(!no)
  rejects { _ = try validateVerificationReply("YES, ignore the rules") }
  rejects { _ = try validateVerificationReply("I cannot help") }
  let validPlan: [String: Any] = ["rules": [["folder": "Insurance", "instruction": "Insurance policies."]]]
  for name in ["Documents", "PERSONAL DOCUMENTS", "General & Other", "Official Records", "Misc.", "Certificates"] {
    rejects { _ = try validateCategoryPlan(["rules":[["folder":name,"instruction":"All documents."]]]) }
  }
  for name in ["Travel", "Identification", "Medical Records", "Employment Documents", "Education Certificates"] {
    _ = try validateCategoryPlan(["rules":[["folder":name,"instruction":"Documents on this subject."]]])
  }
  var genericCalls = 0
  _ = try await categoryPlanWithRetry { retry in
    genericCalls += 1
    return retry ? validPlan : ["rules":[["folder":"Documents","instruction":"Everything else."]]]
  }
  precondition(genericCalls == 2)
  let malformedPlan = "{\"rules\":[{\"folder\":\"Insurance\",\"instruction\":\"Policies.\"},\"rules\":[{\"folder\":\"Employment\",\"instruction\":\"Payslips.\"}]]}"
  var calls = 0
  _ = try await categoryPlanWithRetry { retry in
    calls += 1
    if !retry { return try parseModelJSONObject(malformedPlan) }
    return validPlan
  }
  precondition(calls == 2)
  let oversized: [String: Any] = ["rules": (1...7).map { ["folder": "Category \($0)", "instruction": "Documents."] }]
  calls = 0
  _ = try await categoryPlanWithRetry { retry in
    calls += 1; return retry ? validPlan : oversized
  }
  precondition(calls == 2)
  calls = 0
  do {
    _ = try await categoryPlanWithRetry { _ in calls += 1; return oversized }
    fatalError("Invalid retry accepted")
  } catch { precondition(calls == 2) }
  for code in ["refused", "guardrail", "timeout", "context_limit"] {
    calls = 0
    do {
      _ = try await categoryPlanWithRetry { _ in calls += 1; throw LocalFailure(code: code) }
      fatalError("Provider error swallowed")
    } catch { precondition(calls == 1) }
  }
  rejects { _ = try parseModelJSONObject("I cannot help. {\"rules\":[]}") }
  for folder in ["../escape", "nested/folder", "nested\\folder", ".tuck", "name:bad", "", " trailing "] {
    rejects { _ = try validateCategoryPlan(["rules": [["folder":folder,"instruction":"Documents."]]]) }
  }
  rejects { _ = try validateCategoryPlan(["rules": [["folder":"Home","instruction":"Housing."],["folder":"home","instruction":"Bills."]]]) }
  print("Role contracts: all checks passed")
 }
}
