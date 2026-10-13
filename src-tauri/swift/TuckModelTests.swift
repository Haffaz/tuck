import Foundation
import FoundationModels

@main struct ModelTests {
    static func main() async throws {
        let budget=GenerationBudget()
        for _ in 0..<32 {try await budget.take()}
        do {try await budget.take();assertionFailure("Generation budget was exceeded")} catch let error as LocalFailure {assert(error.code=="call_limit")}

        let endpoint = ModelConnection(kind:"openai",baseUrl:"https://example.test/v1",model:"any-model")
        let sameEndpoint = ModelConnection(kind:"openai",baseUrl:"https://example.test/v1/",model:"another-model")
        let differentEndpoint = ModelConnection(kind:"openai",baseUrl:"https://other.test/v1",model:"any-model")
        let account = try endpoint.account
        let sameAccount = try sameEndpoint.account
        let otherAccount = try differentEndpoint.account
        assert(account == sameAccount && account != otherAccount)
        for url in ["http://remote.test", "https://user:secret@remote.test", "https://remote.test?key=secret", "file:///tmp/file"] {
            do { _ = try ModelConnection(kind:"openai",baseUrl:url,model:"m").validatedURL(); assertionFailure("Unsafe URL accepted") } catch {}
        }
        do { _ = try parseModelJSONObject("```json\n{}\n```"); assertionFailure("Fenced JSON accepted") }
        catch { assert((error as? LocalFailure)?.code == "invalid_response") }

        let reply = "kind: bank statement, checking\nissuer: Chase Bank\ndate: 2026-02\ntags: banking, checking, statement\nsummary: A monthly checking statement."
        let card = try cleanedCard(parsedTextCard(reply))
        assert(card["kind"] as? String == "bank statement" && card["issuer"] as? String == "Chase Bank" && card["date"] as? String == "2026-02")
        assert(card["tags"] as? [String] == ["banking", "checking", "statement"])
        let undated = parsedTextCard("kind: letter\nissuer: none\ndate: unknown\ntags: post")
        assert(undated["issuer"] as? String == "" && undated["date"] as? String == "")
        let unparsedDate = try cleanedCard(["kind": "letter", "date": "February 2026"])
        assert(unparsedDate["date"] as? String == "")
        for bad in ["", "a very long kind of document name here"] {
            do { _ = try cleanedCard(["kind": bad]); assertionFailure("Invalid kind accepted") } catch {}
        }
        assert(photoLabelsInText("[Photo labels: beach, dog]") == ["beach", "dog"])
        assert(photoLabelsInText("[Photo labels: sign]\nThis photo shows a long readable notice with text.") == nil)
        assert(cardExcerpt("[Page 1]\nabc\n[Text truncated]\ndef", limit: 100) == "[Page 1]\nabc\ndef")
        let subjects = parsedSubjects("- Payslips -> Work\nbank statements -> Finance.\nInvented -> Other\nPayslips -> Home", names: ["Payslips", "Bank Statements"])
        assert(subjects == ["Payslips": "Work", "Bank Statements": "Finance"])

        if #available(macOS 26.0, *) {
            let schema = try makeSchema("Test", [stringProperty("value", "A value")])
            do {
                _ = try await generate(model: .default, instructions: String(repeating: "long preference ", count: 20000), prompt: "test", schema: schema)
                assertionFailure("An oversized instruction must fail before generation")
            } catch { assert(errorCode(error) == "context_limit") }
        }
        print("Swift model support checks passed")
    }
}
