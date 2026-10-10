"""Builds a synthetic, mixed-format corpus for the card pipeline evaluation.

Properties the earlier fixtures lacked: several documents of one kind from different
issuers, long multi-page texts (over 5,000 characters), non-English documents and
scanned images/PDFs. All names and numbers are invented. Usage:
    python3 evals/cards/make_corpus.py OUT_DIR
"""
import json, subprocess, sys, tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
RENDER = HERE.parent / "render-fixture.swift"

CLAUSES = [
    "The parties agree that this section applies for the full term unless terminated in writing.",
    "Notices must be delivered to the addresses stated above or to any address later notified.",
    "Any amendment to these terms is valid only if signed by both parties.",
    "Personal data is processed in accordance with applicable data protection law.",
    "Payment obligations survive termination until all outstanding amounts are settled.",
    "Disputes are resolved by the competent court of the district where the property is located.",
    "Each party bears its own costs in connection with the preparation of this document.",
    "Headings are for convenience only and do not affect interpretation.",
]

def long(head, kind_terms, pages=4):
    body = [head]
    n = 1
    for page in range(1, pages + 1):
        body.append(f"\n[Page {page}]\nSection {page}. {kind_terms}")
        for clause in CLAUSES + CLAUSES:
            body.append(f"{page}.{n} {clause}")
            n += 1
    return "\n".join(body)

# (id, gold folder, kind, format, text). format: txt | scan-pdf | scan-png
DOCS = [
    # Banking & Taxes
    ("bank-nordea-sv", "Banking & Taxes", "bank statement", "txt", "Nordea Bank Abp\nKontoutdrag\nPeriod 2026-03-01 - 2026-03-31\nKonto 3300 12 34567\nIngående saldo 12 450,00 SEK\n2026-03-05 ICA Maxi -823,40\n2026-03-25 Lön +32 100,00\nUtgående saldo 40 112,60 SEK"),
    ("bank-chase", "Banking & Taxes", "bank statement", "txt", "Chase Bank\nChecking account statement\nStatement period: February 1 - February 28, 2026\nBeginning balance $2,310.55\nDeposits and additions $4,200.00\nWithdrawals $1,982.17\nEnding balance $4,528.38"),
    ("bank-deutsche-de", "Banking & Taxes", "bank statement", "txt", "Deutsche Bank AG\nKontoauszug Nr. 4/2026\nGirokonto 123456700\nAlter Kontostand 1.250,00 EUR\nGutschrift Gehalt 2.900,00\nLastschrift Miete -950,00\nNeuer Kontostand 3.200,00 EUR"),
    ("bank-barclays-scan", "Banking & Taxes", "bank statement", "scan-pdf", "Barclays Bank UK PLC\nCurrent Account Statement\n1 January 2026 to 31 January 2026\nStart balance 820.10 GBP\nMoney in 2,400.00\nMoney out 1,615.45\nEnd balance 1,604.65 GBP"),
    ("credit-card-amex", "Banking & Taxes", "credit card statement", "txt", "American Express\nCredit card statement\nClosing date 2026-04-12\nPrevious balance $612.40\nNew charges $1,034.99\nPayment due $1,647.39 by 2026-05-07\nMinimum payment $35.00"),
    ("tax-return-us", "Banking & Taxes", "tax return", "txt", long("Form 1040 U.S. Individual Income Tax Return 2025\nFiling status: Single\nWages, salaries, tips 64,200\nTotal tax 7,912\nRefund 408", "Income, deductions and credits reported for the tax year.", 4)),
    ("tax-skatteverket-sv", "Banking & Taxes", "tax assessment", "scan-png", "Skatteverket\nSlutskattebesked 2025\nBeskattningsbar förvärvsinkomst 412 300 kr\nSlutlig skatt 118 940 kr\nAtt få tillbaka 2 114 kr"),
    # Employment & Pay
    ("payslip-acme", "Employment & Pay", "payslip", "txt", "Acme Ltd\nPayslip - March 2026\nEmployee: Jordan Example\nGross pay 4,100.00\nIncome tax -820.00\nPension -205.00\nNet pay 3,075.00"),
    ("payslip-volvo-sv", "Employment & Pay", "payslip", "txt", "Volvo Cars AB\nLönespecifikation april 2026\nMånadslön 38 500 kr\nPreliminär skatt -11 200 kr\nAtt utbetala 27 300 kr"),
    ("payslip-scan", "Employment & Pay", "payslip", "scan-pdf", "Brightwave GmbH\nGehaltsabrechnung Mai 2026\nBrutto 4.350,00 EUR\nLohnsteuer -712,40\nSozialversicherung -880,10\nNetto 2.757,50 EUR"),
    ("contract-employment-long", "Employment & Pay", "employment contract", "txt", long("EMPLOYMENT CONTRACT\nEmployer: Northwind Traders Ltd\nEmployee: Jordan Example\nPosition: Senior Analyst\nStart date: 1 June 2026\nAnnual salary: 62,000 GBP", "Working hours, salary, holidays and notice periods of the employment.", 5)),
    ("contract-employment-short", "Employment & Pay", "employment contract", "txt", "Anställningsavtal\nArbetsgivare: Kista Software AB\nBefattning: Utvecklare\nTillsvidareanställning från 2026-08-01\nMånadslön 45 000 kr\nUppsägningstid 3 månader"),
    ("employment-certificate", "Employment & Pay", "employment certificate", "txt", "CERTIFICATE OF EMPLOYMENT\nThis is to certify that Jordan Example has been employed by Contoso Analytics as a Data Engineer from 2021-02-01 to 2025-12-31 on a full-time basis."),
    # Housing & Utilities
    ("lease-long", "Housing & Utilities", "rental agreement", "txt", long("RESIDENTIAL TENANCY AGREEMENT\nLandlord: Maple Property Group\nTenant: Jordan Example\nPremises: Apartment 4B, 12 River Street\nMonthly rent: 1,450 USD\nTerm: 12 months", "Rent, deposit, repairs and use of the rented apartment.", 5)),
    ("lease-sv", "Housing & Utilities", "rental agreement", "txt", "Hyreskontrakt för bostadslägenhet\nHyresvärd: Wallenstam AB\nHyresgäst: Jordan Example\nLägenhet 1102, 3 rum och kök\nHyra 9 850 kr per månad\nTillträde 2026-01-01"),
    ("electricity-vattenfall-sv", "Housing & Utilities", "electricity bill", "txt", "Vattenfall\nFaktura el\nFörbrukning mars 2026: 412 kWh\nElnät 310 kr\nElhandel 562 kr\nAtt betala 872 kr senast 2026-04-30"),
    ("electricity-edf-scan", "Housing & Utilities", "electricity bill", "scan-png", "EDF Energy\nElectricity bill\nBilling period 01/02/2026 - 28/02/2026\nUnits used 286 kWh\nAmount due 94.18 GBP\nDirect debit on 15/03/2026"),
    ("water-bill", "Housing & Utilities", "water bill", "txt", "Thames Water\nYour water bill\nCharges from 1 April 2026 to 30 September 2026\nWater supply 112.40\nWastewater 98.75\nTotal to pay 211.15 GBP"),
    # Insurance
    ("home-insurance-long", "Insurance", "insurance policy", "txt", long("HOME INSURANCE POLICY SCHEDULE\nInsurer: Lighthouse Mutual Insurance\nPolicyholder: Jordan Example\nPolicy number HM-778201\nPeriod of cover: 2026-01-01 to 2026-12-31\nBuildings and contents cover", "Covered perils, exclusions, excess and claims procedure for the insured home.", 4)),
    ("travel-insurance", "Insurance", "insurance certificate", "txt", "Certificate of travel insurance\nInsurer: Allianz Travel\nInsured person: Jordan Example\nCover: medical expenses up to EUR 1,000,000, trip cancellation\nValid 2026-06-10 to 2026-06-24, worldwide"),
    ("car-insurance-sv", "Insurance", "insurance policy", "txt", "Försäkringsbrev bilförsäkring\nFolksam\nFordon ABC 123, Volvo V60\nHelförsäkring\nFörsäkringsperiod 2026-02-01 - 2027-01-31\nPremie 6 840 kr per år"),
    # Medical Records
    ("lab-results", "Medical Records", "lab results", "txt", "Harbor Clinic Laboratory\nPatient: Jordan Example\nSample taken 2026-03-14\nHaemoglobin 142 g/L (ref 134-170)\nCholesterol 4.8 mmol/L\nHbA1c 36 mmol/mol"),
    ("lab-results-scan", "Medical Records", "lab results", "scan-pdf", "Karolinska Universitetslaboratoriet\nProvsvar\nPatient: Jordan Example\nP-Glukos 5,4 mmol/L\nB-Hemoglobin 138 g/L\nKreatinin 81 umol/L"),
    ("prescription", "Medical Records", "prescription", "txt", "Prescription\nDr. Avery Lin, General Practitioner\nPatient: Jordan Example\nAmoxicillin 500 mg, one capsule three times daily for 7 days\nDate: 2026-02-20"),
    ("radiology", "Medical Records", "radiology report", "txt", "Radiology report\nMeadow Imaging Centre\nExamination: chest X-ray, PA view\nFindings: lungs are clear, no consolidation, heart size normal.\nImpression: normal chest radiograph."),
    # Identity & Immigration
    ("passport-scan", "Identity & Immigration", "passport", "scan-png", "PASSPORT\nType P  Country code GBR\nSurname EXAMPLE\nGiven names JORDAN\nNationality BRITISH CITIZEN\nDate of expiry 14 AUG 2033\nP<GBREXAMPLE<<JORDAN<<<<<<<<<<<<<<"),
    ("residence-card-sv", "Identity & Immigration", "residence permit", "txt", "UPPEHÅLLSTILLSTÅNDSKORT\nMigrationsverket\nTillståndstyp: permanent uppehållstillstånd\nKortet visar att innehavaren har rätt att bo och arbeta i Sverige."),
    ("visa-decision", "Identity & Immigration", "visa decision", "txt", "Decision on your visa application\nHome Office UK Visas and Immigration\nApplicant: Jordan Example\nYour application for a Skilled Worker visa has been approved. Valid until 2029-05-31."),
    # Education
    ("transcript-aster", "Education", "academic transcript", "txt", "Academic transcript\nAster University\nStudent: Jordan Example\nBSc Computer Science\nAlgorithms A, Databases B+, Statistics A-\nCumulative GPA 3.71"),
    ("transcript-pine-scan", "Education", "academic transcript", "scan-pdf", "Pine Valley College\nOfficial Transcript of Records\nStudent: Jordan Example\nCalculus I 4.0\nPhysics I 3.7\nTotal credits 32"),
    ("diploma", "Education", "diploma", "txt", "Lakeside University\nhereby confers upon Jordan Example the degree of Master of Science in Data Science with all the rights and privileges thereto. Awarded 2024-06-15."),
    # Travel
    ("flight-booking", "Travel", "flight booking", "txt", "Booking confirmation\nSAS Scandinavian Airlines\nBooking reference K7Q2ZP\nStockholm ARN - London LHR, SK1531, 2026-06-10 07:40\nPassenger: Jordan Example\nFare paid 1 840 SEK"),
    ("flight-eticket", "Travel", "flight booking", "txt", "Electronic ticket receipt\nLufthansa\nTicket number 220-4455667788\nFrankfurt FRA - New York JFK, LH400, 2026-09-03\nPassenger EXAMPLE/JORDAN MR"),
    ("hotel-booking", "Travel", "hotel reservation", "txt", "Hotel reservation confirmed\nThe Harbour Hotel, Lisbon\nGuest: Jordan Example\nCheck-in 2026-07-02, check-out 2026-07-06\n4 nights, double room, breakfast included\nTotal 612 EUR"),
    ("hotel-booking-scan", "Travel", "hotel reservation", "scan-png", "Booking.com\nConfirmation number 4021.887.133\nAlpine Lodge Zermatt\nArrival 2026-12-20 Departure 2026-12-27\nTotal price CHF 1,980"),
]

GOLD_RULES = [
    {"folder": "Banking & Taxes", "instruction": "Bank statements, credit card statements, account records and tax documents."},
    {"folder": "Employment & Pay", "instruction": "Employment contracts, employment certificates, payslips and other pay records."},
    {"folder": "Housing & Utilities", "instruction": "Rental agreements and household utility bills such as electricity and water."},
    {"folder": "Insurance", "instruction": "Insurance policies and certificates of cover, including home, car and travel insurance."},
    {"folder": "Medical Records", "instruction": "Lab results, prescriptions, radiology and other clinical records."},
    {"folder": "Identity & Immigration", "instruction": "Passports, ID cards, residence permits and visa decisions."},
    {"folder": "Education", "instruction": "Transcripts, diplomas and other academic records."},
    {"folder": "Travel", "instruction": "Flight bookings, tickets and hotel reservations."},
]

def main(out: Path):
    if out.exists():
        sys.exit("Use a new output directory")
    files = out / "files"
    files.mkdir(parents=True)
    with tempfile.TemporaryDirectory() as tmp:
        renderer = Path(tmp) / "render"
        subprocess.run(["swiftc", "-O", str(RENDER), "-o", str(renderer)], check=True)
        gold = []
        for ident, folder, kind, fmt, text in DOCS:
            if fmt == "txt":
                name = f"{ident}.txt"
                (files / name).write_text(text, encoding="utf-8")
            else:
                name = f"{ident}.{'pdf' if fmt == 'scan-pdf' else 'png'}"
                source = Path(tmp) / f"{ident}.txt"
                source.write_text(text, encoding="utf-8")
                subprocess.run([str(renderer), str(source), str(files / name)], check=True)
            gold.append({"file": name, "folder": folder, "kind": kind, "format": fmt, "chars": len(text)})
    (out / "gold.json").write_text(json.dumps({"rules": GOLD_RULES, "documents": gold}, indent=1))
    print(f"{len(gold)} documents in {out}")

if __name__ == "__main__":
    main(Path(sys.argv[1]).resolve())
