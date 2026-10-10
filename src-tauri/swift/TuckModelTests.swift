import Foundation
import FoundationModels

@main struct ModelTests {
    static func main() async throws {
        let budget=GenerationBudget()
        for _ in 0..<32 {try await budget.take()}
        do {try await budget.take();assertionFailure("Generation budget was exceeded")} catch let error as LocalFailure {assert(error.code=="call_limit")}

        let academic:[[String:Any]]=(0..<6).map { i in ["id":"a\(i)","description":"Academic transcript","quote":"Academic transcript","detailEvidence":"Institution: \(i<3 ? "Harbor University" : "Valley University")"] }
        let broad=academic.map{["id":$0["id"] as! String,"category":"Education"]}
        for policy in ["Do not group documents by institution.","Group warranty documents by institution."] {
            let checked=consistentEvidenceAssignments(broad,samples:academic,maxDepth:2,preferences:policy)
            assert(checked.allSatisfy{$0["category"] == "Education"})
        }
        let manyDestinations=(0..<40).map{RoleDestination(folder:"Subject \($0)",instruction:String(repeating:"Accepted document records. ",count:20),context:String(repeating:"Accepted document records. ",count:20))}
        assert(destinationChoices(Array(manyDestinations.prefix(2))) == ["Subject 0","Subject 1"])
        var repeated=manyDestinations[0];repeated.folder="Subject 0 (scope 1)"
        let variants=destinationChoices([manyDestinations[0],manyDestinations[0],repeated])
        assert(Set(variants).count==3 && variants.allSatisfy{$0.hasPrefix("Subject 0")})
        let batches=try destinationBatches(manyDestinations)
        assert(batches.count>1 && batches.flatMap{$0}.map{$0.folder}==manyDestinations.map{$0.folder})
        assert(batches.allSatisfy{$0.count<=8})
        var excessive=manyDestinations[0];excessive.context=String(repeating:"x",count:6000)
        do {_ = try destinationBatches([excessive]);assertionFailure("Oversized rule accepted")} catch {}
        for depth in 1...2 {
            let prompt = try categoryGroupingInstructions(maxDepth:depth, extra:"Keep each employer separate.")
            assert(prompt.contains("maximum, not a target") && prompt.contains("Keep each employer separate.") && prompt.contains("Needs review"))
            let deep = ["rules":[["folder":"Income/Acme/Payslips","instruction":"Acme payslips."]]]
            do { _ = try validateCategoryPlan(deep, maxDepth:depth); assertionFailure("Excess nesting accepted") } catch {}
        }
        for depth in [0,3,4] {
            do { _ = try categoryGroupingInstructions(maxDepth:depth, extra:""); assertionFailure("Invalid nesting accepted") } catch {}
        }
        assert(explicitSubjectPolicy("Cabinet",documentType:"Technical inspection report",preferences:"File inspection reports under Cabinet, grouped by project."))
        assert(!explicitSubjectPolicy("Insurance",documentType:"Electricity bill",preferences:"File all insurance policies in Insurance. Keep tenancy agreements in Housing."))
        assert(!applicableGroupingClause("Group invoices by supplier",documentType:"Technical inspection report",subject:"Research"))
        assert(applicableGroupingClause("Group inspection reports by project",documentType:"Technical inspection report",subject:"Research"))
        assert(!applicableGroupingClause("Do not group inspections by supplier",documentType:"Inspection report",subject:"Research"))
        assert(requestedGroupingTerms("Group inspection reports by the explicitly named project") == ["project"])
        assert(requestedGroupingTerms("Group by project, supplier, clinic or institution, then document type") == ["project","supplier","clinic","institution"])
        assert(requestedGroupingTerms("Group reports by document type").isEmpty)
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
        let clearScope = try clearDocumentScope("CLEAR. The record identifies a specific document type.")
        assert(clearScope)
        let mixedScope = try clearDocumentScope("MIXED. Two unrelated records.")
        assert(!mixedScope)
        for bad in ["Not CLEAR", "CLEAR or UNCLEAR", "The document is CLEAR", "CLEAR but uncertain"] {
            do { _ = try clearDocumentScope(bad); assertionFailure("Invalid scope was accepted") } catch { }
        }
        assert(typeScopePolicy("Employment contracts issued by Example Org. Salary payslips belong under Income.").contains("Salary payslips belong under Income"))
        assert(typeScopePolicy("Product warranties. Purchase receipts belong in Receipts.").contains("Purchase receipts belong in Receipts"))
        assert(explicitTypePaths("Employment contracts belong in Employment/Acme/Contracts.",documentType:"Employment contract").isEmpty)
        assert(explicitTypePaths("Inspection reports belong under Research/Cedar.",documentType:"Technical inspection report") == ["Research/Cedar"])
        assert(explicitTypePaths("Invoices belong in Finance/Vendor if approved.",documentType:"Invoice").isEmpty)
        assert(explicitTypePaths("Warranties belong in Purchases.",documentType:"Clinical laboratory report").isEmpty)
        assert(redirectedTypeConflict("Product warranties. Purchase receipts belong in Receipts.",folder:"Warranties",documentType:"Retail purchase receipt"))
        assert(!redirectedTypeConflict("Purchase receipts belong in Receipts.",folder:"Receipts/Shop",documentType:"Retail purchase receipt"))
        assert(!redirectedTypeConflict("Employment contracts. Salary payslips belong under Income.",folder:"Employment/Acme",documentType:"Employment contract"))
        assert(redirectedTypeConflict("Employment contracts. Salary payslips belong under Income.",folder:"Employment/Acme",documentType:"Monthly salary payslip"))
        assert(requestedSubdivisionDepth("Group invoices by supplier.",documentType:"Supplier invoice",category:"Invoices/Harbor") == 2)
        assert(requestedSubdivisionDepth("Group invoices by supplier.",documentType:"Academic transcript",category:"Education/Transcripts") == 0)
        assert(requestedSubdivisionDepth("Use clear English names.",documentType:"Academic transcript",category:"Education/Transcripts") == 0)
        assert(requestedSubdivisionDepth("Do not separate education records by type.",documentType:"Academic transcript",category:"Education/Transcripts") == 0)
        assert(requestedSubdivisionDepth("Academic transcripts belong in Education/Transcripts.",documentType:"Academic transcript",category:"Education/Transcripts") == 2)
        assert(requestedSubdivisionDepth("Group invoices by supplier.",documentType:"Supplier invoice",category:"Invoices") == 0)
        assert(requestedSubdivisionDepth("Group invoices by supplier.",documentType:"Supplier invoice",category:"Invoices/Harbor/Invoices",fields:["f0"]) == 2)
        assert(requestedSubdivisionDepth("Group invoices by supplier, then document type.",documentType:"Supplier invoice",category:"Invoices/Harbor/Invoices",fields:["f0","type"]) == 2)
        let indexedLines = numberedSourceLines("[Document has 2 pages.]\n[Page 1]\nCover letter\n[Page 2]\nUPPEHÅLLSTILLSTÅND\nApproved residence permit.")
        assert(indexedLines.count == 3)
        let indexed = try indexedDescriptionReply("{\"line\":2,\"description\":\"Residence permit\"}", lines:indexedLines)
        assert(indexed["quotes"] as? [String] == ["UPPEHÅLLSTILLSTÅND"])
        let facts = try indexedDescriptionReply("{\"line\":1,\"description\":\"Cover\",\"detailLines\":[2,3]}", lines:indexedLines)
        assert(facts["quotes"] as? [String] == indexedLines)
        let noisy = try indexedDescriptionReply("{\"line\":2,\"description\":\"Permit\",\"detailLines\":[2,2,0,4,1,3,1]}", lines:indexedLines)
        assert(noisy["quotes"] as? [String] == [indexedLines[1], indexedLines[0], indexedLines[2]])
        let manyLines = (1...20).map { "Verified factual source line \($0)" }
        let many = try indexedDescriptionReply("{\"line\":1,\"description\":\"Inspection report\",\"detailLines\":[1,2,3,4,5,6,7,8,9,10,999]}", lines:manyLines)
        assert(many["quotes"] as? [String] == Array(manyLines.prefix(7)))
        let short = try indexedDescriptionReply("{\"line\":1,\"description\":\"Inspection report\",\"detailLines\":[2,3,4]}", lines:["Inspection report", "42", "", "Project: Cedar"])
        assert(short["quotes"] as? [String] == ["Inspection report", "Project: Cedar"])
        for (path,actualDepth) in [("Income",1),("Income/Acme",2),("Income/Acme/Payslips",3)] {
            for limit in 1...2 { assert(safeEvidenceCategory(path,maxDepth:limit) == (actualDepth <= limit)) }
        }
        for path in ["/Income","Income/../Acme","Income//Acme","Income/.hidden","Income/Documents","Income\\Acme"] {
            assert(!safeEvidenceCategory(path,maxDepth:2))
        }
        let sample:[String:Any] = ["description":"Technical inspection report","quote":"Technical inspection report","detailEvidence":"Project: Orion"]
        assert(groundedCategory("Research/Orion/Inspections",sample:sample,preferences:"") == "Research/Orion/Inspections")
        assert(groundedCategory("Research/Invented/Inspections",sample:sample,preferences:"") == "Research")
        assert(groundedCategory("Research/Client One/Inspections",sample:sample,preferences:"Use Research/Client One for project Orion inspections.") == "Research/Client One/Inspections")
        assert(groundedCategory("Research/Client One/Inspections",sample:sample,preferences:"Use Research/Client One for project Vega inspections.") == "Research")
        let unnamed:[String:Any] = ["description":"Monthly salary payslip","quote":"Monthly salary payslip","detailEvidence":"Employee: Alex Example"]
        assert(groundedCategory("Income/Gamma",sample:unnamed,preferences:"Infer Gamma as the employer even if not written.") == "Income")
        let namedProject:[String:Any] = ["id":"a","quote":"Meeting notes","detailEvidence":"Project: Orion"]
        let unnamedProject:[String:Any] = ["id":"b","quote":"Meeting notes","detailEvidence":"Date: October 2026"]
        let normalized = consistentEvidenceAssignments([["id":"a","category":"Work/Orion"],["id":"b","category":"Work/Meeting notes"]],samples:[namedProject,unnamedProject])
        assert(normalized[0]["category"] == "Work/Orion" && normalized[1]["category"] == "Work")
        let broadProject:[String:Any] = ["id":"c","quote":"Technical inspection report","detailEvidence":"Project: Orion"]
        let corrected = consistentEvidenceAssignments([["id":"a","category":"Work/Orion/Notes"],["id":"c","category":"Work"]],samples:[namedProject,broadProject])
        assert(corrected[1]["category"] == "Work/Orion")
        let namedVega:[String:Any] = ["id":"v","quote":"Meeting notes","detailEvidence":"Project: Vega"]
        let counterpart = consistentEvidenceAssignments([["id":"a","category":"Work/Orion"],["id":"v","category":"Work/Meeting notes"]],samples:[namedProject,namedVega],maxDepth:2)
        assert(counterpart[1]["category"] == "Work/Vega")
        let typesOnly = consistentEvidenceAssignments([["id":"b","category":"Work/Meeting notes"]],samples:[unnamedProject])
        assert(typesOnly[0]["category"] == "Work/Meeting notes")
        assert(scopeTypeLabel("Policies",in:"Home insurance policy"))
        let unclear = try indexedDescriptionReply("{\"line\":0,\"description\":\"Unclear\"}", lines:indexedLines)
        assert(!(unclear["uncertainty"] as! String).isEmpty)
        let uncertainType = try indexedDescriptionReply("{\"line\":2,\"description\":\" Unclear \",\"detailLines\":[1]}", lines:indexedLines)
        assert((uncertainType["quotes"] as! [String]).isEmpty)
        for bad in ["{\"line\":-1,\"description\":\"Policy\"}", "{\"line\":4,\"description\":\"Policy\"}", "{\"line\":1.5,\"description\":\"Policy\"}", "{\"line\":1,\"description\":\"\"}", "```json\n{\"line\":1,\"description\":\"Policy\"}\n```"] {
            do { _ = try indexedDescriptionReply(bad, lines:indexedLines); assertionFailure("Invalid indexed evidence was accepted") } catch { }
        }
        assert(sourcePageHeads("[Document has 2 pages.]\n\n[Page 1]\nPayslip\n[Page 2]\nResidence permit") == ["Payslip", "Residence permit"])
        assert(containsQuote("KONTOUTDRAG  Bank", in: "Header\nKONTOUTDRAG\nBank\nBalance"))
        assert(!containsQuote("Residence Permit", in: "Employment Certificate"))
        assert(!containsQuote("", in: "anything"))
        assert(templateTitle("Residence Permit Card") == "Residence Permit Card")
        assert(templateTitle("Other").isEmpty)
        assert(templateTitle("Unknown").isEmpty)
        assert(templateTitle("Current Visa 2026").isEmpty)
        if #available(macOS 26.0, *) {
            let schema = try makeSchema("Test", [stringProperty("value", "A value")])
            do {
                _ = try await generate(model: .default, instructions: String(repeating: "long preference ", count: 20000), prompt: "test", schema: schema)
                assertionFailure("An oversized instruction must fail before generation")
            } catch { assert(errorCode(error) == "context_limit") }
        }
        assert(!needsSuggestionCheck(type: "Bank Statement", grounded: true, sourceAssociation: false, toolCount: 0, fit: "clear"))
        assert(needsSuggestionCheck(type: "Bank Statement", grounded: true, sourceAssociation: false, toolCount: 1, fit: "clear"))
        assert(needsSuggestionCheck(type: "Other", grounded: true, sourceAssociation: false, toolCount: 0, fit: "clear"))
        assert(needsSuggestionCheck(type: "Unknown", grounded: false, sourceAssociation: true, toolCount: 0, fit: "clear"))
        assert(needsSuggestionCheck(type: "Bank Statement", grounded: true, sourceAssociation: false, toolCount: 0, fit: "uncertain"))
        let evidence = "Bank statement with transactions"
        var check: [String: Any] = ["result": "supported", "evidenceQuote": evidence]
        assert(checkedSuggestion(check, title: "Bank Statement", evidence: evidence, sourceAssociation: false).folderOK)
        check["result"] = "conflicting_folder"
        assert(!checkedSuggestion(check, title: "Bank Statement", evidence: evidence, sourceAssociation: false).folderOK)
        check["result"] = "supported"; check["evidenceQuote"] = "Invented quote"
        assert(!checkedSuggestion(check, title: "Bank Statement", evidence: evidence, sourceAssociation: false).folderOK)
        check["evidenceQuote"] = evidence; check["result"] = "unsupported_name"
        let checked = checkedSuggestion(check, title: "Bank Statement", evidence: evidence, sourceAssociation: false)
        assert(checked.folderOK && !checked.titleOK)
        assert(!checkedSuggestion([:], title: "Bank Statement", evidence: evidence, sourceAssociation: true).folderOK)
        assert(duplicateRule("Keep bank statements here.", among: ["KEEP bank statements here."]))
        assert(!duplicateRule("Keep personal rentals here.", among: ["Only commercial leases belong here."]))
        assert(sameRuleScope("Finances", "Banking"))
        assert(!sameRuleScope("Home/Invoices", "Work/Invoices"))
        assert(destinationIndexes(["Home", "Home/Rent"]).contains("1: Home/Rent"))
        let selected = try selectedFolder(["selection": 1], allowed: ["Home", "Home/Rent"])
        assert(selected == "Home/Rent")
        for bad: Any in [-1, 2, "1", true] {
            do { _ = try selectedFolder(["selection": bad], allowed: ["Home", "Home/Rent"]); assertionFailure("Invalid selection accepted") }
            catch { assert((error as? LocalFailure)?.code == "invalid_response") }
        }
        do { _ = try selectedFolder(["selection": 0], allowed: []); assertionFailure("Empty list accepted") }
        catch { assert((error as? LocalFailure)?.code == "invalid_response") }
        assert(literalScopeLabel("Harbor Clinic",in:"Clinic: Harbor Clinic"))
        assert(!literalScopeLabel("Harbor Clinic",in:"Clinic: Meadow Clinic"))
        assert(!literalScopeLabel("NIM",in:"AGENCY documents"))
        assert(literalScopeLabel("Orion",in:"Project: Orion"))
        assert(sharedSourceName(["AGENCY"], rule: "Keep AGENCY application documents together.") == "agency")
        assert(sharedSourceName(["Cedar Services"], rule: "Keep Cedar Services documents here.") == "cedar services")
        assert(sharedSourceName(["2026 AGENCY files"], rule: "AGENCY supporting documents.") == "agency")
        assert(sharedSourceName(["NIM"], rule: "AGENCY documents.") == nil)
        assert(sharedSourceName(["Incoming"], rule: "Incoming documents.") == nil)
        assert(sharedSourceName(["Application Files"], rule: "Application files.") == nil)
        assert(sharedSourceName(["2026"], rule: "Keep 2026 documents.") == nil)
        assert(sharedSourceName([""], rule: "") == nil)
        assert(sharedSourceName(["Cedar"], rule: "Keep Pine documents.") == nil)
        assert(templateTitle("Driving Licence") == "Driving Licence")
        assert(templateTitle("Birth Certificate") == "Birth Certificate")
        assert(topicRules("Shared naming rules\n\nIdentity: Identification documents", shared: "Shared naming rules") == "Identity: Identification documents")
        let inherited=RoleDestination(folder:"Work/Orion",instruction:"Inspection reports for Orion.",context:"Naming rules\n\nWork: Signed inspection reports only.\n\nWork/Orion: Inspection reports for Orion.")
        assert(ancestorRules(inherited,rootRules:"Naming rules") == "Work: Signed inspection reports only.")
        assert(scopeEvidence(quotes:["First-page type"],pageHeads:["First record","Second record"])["recordFacts"] == nil)
        let knownScope="Required source details: (Project: Orion) AND (Supplier: Harbor Office). Each subgroup must be factual."
        assert(!contradictorySourceDetails(knownScope,quotes:["Project: Orion","Supplier: Harbor Office Ltd"]))
        assert(contradictorySourceDetails(knownScope,quotes:["Project: Vega","Supplier: Harbor Office Ltd"]))
        assert(!contradictorySourceDetails(knownScope,quotes:["Projekt: Orion","Leverantör: Harbor Office"]))
        assert(applyFilenamePrefix("Meeting minutes",rules:"Use filenames start with Archive.") == "Archive Meeting minutes")
        assert(applyFilenamePrefix("Archive Meeting minutes",rules:"Use filenames starting with Archive.") == "Archive Meeting minutes")
        assert(applyFilenamePrefix("",rules:"Use filenames starting with Archive.") == "")
        assert(applyFilenamePrefix("Technical report",rules:"Use filenames beginning with \"Project File\".") == "Project File Technical report")
        let types:[[String:Any]]=[["id":"a","quote":"Inspection report","description":"Inspection report"],["id":"b","quote":"Monthly inspection report","description":"Inspection report"]]
        let normalizedTypes=consistentEvidenceAssignments([["id":"a","category":"Work/Inspection reports"],["id":"b","category":"Work"]],samples:types,maxDepth:2)
        assert(normalizedTypes[1]["category"] == "Work/Inspection reports")
        let named=consistentEvidenceAssignments([["id":"a","category":"Work/Orion"],["id":"b","category":"Work"]],samples:types,maxDepth:2)
        assert(named[1]["category"] == "Work")
        let department:[[String:Any]]=[["id":"d","quote":"Inspection report","description":"Inspection report","detailEvidence":"Department: Research"]]
        let explicit=consistentEvidenceAssignments([["id":"d","category":"Work/Inspection reports"]],samples:department,maxDepth:2,preferences:"Group by department, then document type.")
        assert(explicit[0]["category"] == "Work/Research")
        let missingDepartment=consistentEvidenceAssignments([["id":"a","category":"Work"]],samples:types,maxDepth:2,preferences:"Group by department. Infer Research if absent.")
        assert(missingDepartment[0]["category"] == "Work")
        let translatedFields:[[String:Any]]=[["id":"t","quote":"Inspection report","description":"Technical inspection report","detailEvidence":"Projekt: Orion\nLeverantör: Harbor Office"]]
        let guessedField=[["id":"t","category":"Work/Harbor Office"]]
        assert(consistentEvidenceAssignments(guessedField,samples:translatedFields,maxDepth:2,recordGroupingFields:["t":["f0"]])[0]["category"] == "Work/Orion")
        assert(consistentEvidenceAssignments(guessedField,samples:translatedFields,maxDepth:2,recordGroupingFields:["t":["f0","f1"]])[0]["category"] == "Work/Orion")
        assert(consistentEvidenceAssignments(guessedField,samples:translatedFields,maxDepth:2,recordGroupingFields:["t":["missing"]])[0]["category"] == "Work")
        assert(consistentEvidenceAssignments(guessedField,samples:translatedFields,maxDepth:2,recordGroupingFields:["t":["f0","missing"]])[0]["category"] == "Work/Orion")
        let requiredProject=RoleDestination(folder:"Work/Orion/Notes",instruction:"Meeting minutes explicitly for Orion only.",context:"")
        assert(unsupportedScopeLabel(requiredProject,description:"Meeting minutes",quotes:["Meeting minutes","No project is identified."]))
        assert(!unsupportedScopeLabel(requiredProject,description:"Meeting minutes",quotes:["Meeting minutes","Project: Orion"]))
        let aliased=RoleDestination(folder:"Work/Client One",instruction:"Meeting minutes for project Orion. The folder name is an alias.",context:"")
        assert(!unsupportedScopeLabel(aliased,description:"Meeting minutes",quotes:["Meeting minutes","Project: Orion"]))
        assert(typeScopePolicy("Inspection reports whose department is Research only.") == "Inspection reports")
        assert(typeScopePolicy("File documents here. Examples: Inspection report from Research.") == "Inspection report")
        let fallbackRule="Signed inspection reports where no department is identified. Named departments have their own child folders."
        let childRule=withoutOwnFallback(fallbackRule)
        assert(childRule.contains("Signed inspection reports") && !childRule.contains("no department"))
        assert(withoutOwnFallback("Signed inspection reports only.") == "Signed inspection reports only.")
        assert(missingFieldPolicyContradiction("Inspection reports where department name is absent.",quotes:["Department: Research"]))
        assert(!missingFieldPolicyContradiction("Inspection reports where department name is absent.",quotes:["Inspection report"]))
        assert(!missingFieldPolicyContradiction("Inspection reports for Research only. Do not use this folder when the department is missing or different.",quotes:["Department: Research"]))
        assert(needsSubjectContext(RoleDestination(folder:"Research",instruction:"File Research documents here. Other subjects have their own folders.",context:"")))
        assert(!needsSubjectContext(RoleDestination(folder:"Research",instruction:"Inspection reports for project Atlas.",context:"")))
        assert(!needsSubjectContext(RoleDestination(folder:"Research",instruction:"File documents about Research here. Required document scope: Inspection report.",context:"")))
        let equivalentTypes:[[String:Any]]=[
            ["id":"a","quote":"Inspection report","description":"Technical inspection report","detailEvidence":"Department: Research"],
            ["id":"b","quote":"Inspection report","description":"Inspection report","detailEvidence":"Department: Operations"],
            ["id":"c","quote":"Technical inspection report","description":"Inspection report","detailEvidence":""]]
        let mergedTypes=consistentEvidenceAssignments([["id":"a","category":"Work/Inspection"],["id":"b","category":"Work/Inspections"],["id":"c","category":"Work"]],samples:equivalentTypes)
        assert(mergedTypes.allSatisfy{$0["category"]=="Work/Inspection"})
        let explicitTypes=consistentEvidenceAssignments([["id":"a","category":"Work/Inspection"],["id":"b","category":"Work/Inspections"]],samples:equivalentTypes,preferences:"Keep the chosen folder labels.")
        assert(explicitTypes[1]["category"]=="Work/Inspections")
        let namedTypeSample:[[String:Any]]=[["id":"a","quote":"Research inspection report","description":"Inspection report from Research","detailEvidence":"Department: Research"]]
        let noRepeatedDetail=consistentEvidenceAssignments([["id":"a","category":"Work/Research"]],samples:namedTypeSample,preferences:"Group by department.")
        assert(noRepeatedDetail[0]["category"]=="Work/Research")
        let usefulTypeDetail=consistentEvidenceAssignments([["id":"a","category":"Work/Research/Inspection"]],samples:namedTypeSample,preferences:"Group by department.")
        assert(usefulTypeDetail[0]["category"]=="Work/Research")
        let policyTypes=RoleDestination(folder:"Travel/Insurance/Policies",instruction:"Travel insurance certificates and policies. Home insurance belongs in Insurance.",context:"")
        assert(!unsupportedScopeLabel(policyTypes,description:"Travel insurance certificate",quotes:["Travel insurance certificate"]))
        let folders = ["Identity", "Migration", "Needs Review"]
        let identityEvidence = "DRIVING LICENCE"
        let supported = try checkedDestination(["folderSelection": 0, "nameSelection": 1, "evidenceQuote": identityEvidence], candidate: "Identity", folders: folders, title: "Driving Licence", evidence: identityEvidence)
        assert(supported.folderOK && supported.titleOK)
        let wrongName = try checkedDestination(["folderSelection": 0, "nameSelection": 0, "evidenceQuote": identityEvidence], candidate: "Identity", folders: folders, title: "Residence Permit", evidence: identityEvidence)
        assert(wrongName.folderOK && !wrongName.titleOK)
        let missingEvidence = try checkedDestination(["folderSelection": 0, "nameSelection": 1, "evidenceQuote": ""], candidate: "Identity", folders: folders, title: "Driving Licence", evidence: identityEvidence)
        assert(!missingEvidence.folderOK && missingEvidence.reason.contains("supporting passage"))
        let conflict = try checkedDestination(["folderSelection": 1], candidate: "Identity", folders: folders, title: "Driving Licence", evidence: identityEvidence)
        assert(!conflict.folderOK && conflict.reason.contains("different folders"))
        let uncertain = try checkedDestination(["folderSelection": 2], candidate: "Identity", folders: folders, title: "Driving Licence", evidence: identityEvidence)
        assert(!uncertain.folderOK && uncertain.reason.contains("could not identify"))
        do { _ = try checkedDestination(["folderSelection": -1], candidate: "Identity", folders: folders, title: "", evidence: identityEvidence); assertionFailure("Invalid index accepted") }
        catch { assert((error as? LocalFailure)?.code == "invalid_response") }
        if #available(macOS 27.0, *) {
            assert(visualPrompt(text: "test", path: "relative.jpg", page: 1) == nil)
            assert(visualPrompt(text: "test", path: "/private/tmp/not-a-tuck-image.jpg", page: 0) == nil)
            assert(visualPrompt(text: "test", path: "/private/tmp/not-a-tuck-image.jpg", page: 51) == nil)
        }
        print("Swift model support checks passed")
    }
}
