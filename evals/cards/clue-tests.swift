// Assertions for TuckClues.swift. Run:
//   swiftc -parse-as-library src-tauri/swift/TuckClues.swift evals/cards/clue-tests.swift -o /tmp/clue-tests && /tmp/clue-tests
import Foundation

@main struct ClueTests {
    static func check(_ condition: Bool, _ message: String) {
        if !condition { print("FAIL:", message); exit(1) }
    }
    static func main() {
        check(clueDomain("https://www.nordea.se/statements/123?token=abc") == "nordea.se", "URL keeps only the domain")
        check(clueDomain("Jordan Example <jordan@migrationsverket.se>") == "migrationsverket.se", "email keeps only the domain")
        check(clueDomain("Statement March") == nil, "plain text is not a domain")
        check(barcodeClue("BCD\n002\n1\nSCT\nNDEASESS\nShop\nSE45500000000583\nEUR10", symbology: "QR") == "Payment QR code (SEPA transfer)",
              "SEPA payment QR is named without account details")
        check(barcodeClue("M1EXAMPLE/JORDAN     EK7Q2ZP ARNLHRSK 1531", symbology: "Barcode") == "Boarding pass barcode", "boarding pass")
        check(barcodeClue("https://tickets.example.com/t/9f3", symbology: "QR") == "QR code linking to tickets.example.com", "link domain")
        check(machineReadableClue("PASSPORT\nP<GBREXAMPLE<<JORDAN<<<<<<<<<<<<<<<<<<<<<<<<<\n1234567890GBR8001014M3008150<<<<<<<<<<<<<<04")
              == "Machine-readable zone: passport, issued by GBR", "passport MRZ")
        check(machineReadableClue("IDSWE1234567<8<<<<<<<<<<<<<<<\n9001019M3008150SWE<<<<<<<<<<<6\nEXAMPLE<<JORDAN<<<<<<<<<<<<<<")
              == "Machine-readable zone: identity card, issued by SWE", "ID card MRZ")
        check(machineReadableClue("Bank statement\nOpening balance 1,250\nTotal <<< 3") == nil, "ordinary text has no MRZ")
        check(looksLikeDocument(["document", "text"]) && looksLikeDocument([]), "paper labels")
        check(!looksLikeDocument(["beach", "sky"]), "scenery is not paper")
        print("clue tests passed")
    }
}
