import Foundation
@main struct SimpleChecks {
    static func main() throws {
        let input = Input(instructions: "", prompt: "", actions: ["finish", "read_page", "retry_ocr"], folders: ["Identification", "Home", "Needs Review"], folderInstructions: "", evidenceQuotes: ["DRIVING LICENCE"], routingEvidence: "DRIVING LICENCE", sourceFolders: [], folderRules: [FolderRule(folder: "Identification", instruction: "Identity and licensing documents."), FolderRule(folder: "Home", instruction: "Rental agreements.")], destinationPrompts: [DestinationPrompt(folder: "Identification", instructions: "Identity and licensing documents."), DestinationPrompt(folder: "Home", instructions: "Rental agreements.")], rename: true)
        let good: [String: Any] = ["action": "finish", "page": 0, "folder": "Identification", "folderEvidence": "DRIVING LICENCE", "documentType": "Driving Licence", "typeEvidence": "DRIVING LICENCE", "reason": "Matches licensing documents."]
        var checks = 0
        func check(_ condition: Bool) { precondition(condition); checks += 1 }
        func reply(_ changes: [String: Any] = [:], _ context: Input? = nil) throws -> [String: Any] {
            try validatedFiling(good.merging(changes, uniquingKeysWith: { _, new in new }), input: context ?? input)
        }
        let valid = try reply()
        check(valid["folder"] as? String == "Identification" && valid["title"] as? String == "Driving Licence")
        let uncertainName = try reply(["documentType": "Unknown", "typeEvidence": ""])
        check(uncertainName["folder"] as? String == "Identification" && uncertainName["title"] as? String == "")
        let falseName = try reply(["typeEvidence": "BIRTH CERTIFICATE"])
        check(falseName["folder"] as? String == "Identification" && falseName["title"] as? String == "")
        check(try reply(["folderEvidence": "INVENTED QUOTE"])["confident"] as? Bool == false)
        check(try reply(["folder": "Needs Review"])["confident"] as? Bool == false)
        var disabledRename = input; disabledRename.rename = false
        check(try reply([:], disabledRename)["title"] as? String == "")
        var duplicate = input
        duplicate.folderRules[1].instruction = duplicate.folderRules[0].instruction
        check(try reply([:], duplicate)["confident"] as? Bool == false)
        var source = input
        source.sourceFolders = ["Cedar Services"]
        source.folderRules[0].instruction = "Keep all documents from Cedar Services together."
        check(try reply(["folderEvidence": "", "typeEvidence": ""], source)["folder"] as? String == "Identification")
        source.sourceFolders = ["Incoming"]
        check(try reply(["folderEvidence": ""], source)["confident"] as? Bool == false)
        var more = input; more.nextRead = ReadHint(action: "read_page", page: 3)
        let requested = try reply(["folder": "Needs Review"], more)
        check(requested["action"] as? String == "read_page" && requested["page"] as? Int == 3)
        let explicit = try reply(["action": "retry_ocr", "page": 1])
        check(explicit["action"] as? String == "retry_ocr" && explicit["confident"] as? Bool == false)
        for invalid: [String: Any] in [["folder": "../outside"], ["folder": "Disabled"], ["folder": 0], ["action": "delete"], ["action": "read_page", "page": 51]] {
            do { _ = try reply(invalid); preconditionFailure("Invalid result accepted") }
            catch { checks += 1 }
        }
        print("\(checks) single-decision validation checks passed")
    }
}
