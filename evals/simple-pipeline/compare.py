"""Summarize frozen-case results; matching review cases do not count as filings."""
import json
import sys
from pathlib import Path

def summary(report):
    rows = report["results"]
    substantive = [r for r in rows if r["expected_folder"] != "Needs Review"]
    manual = [r for r in rows if r["expected_folder"] == "Needs Review"]
    named = [r for r in substantive if r["decision"]["title"]]
    return {
        "substantive_documents": len(substantive),
        "correctly_filed": sum(r["folder_correct"] for r in substantive),
        "wrongly_filed": sum(not r["review"] and not r["folder_correct"] for r in rows),
        "unnecessary_fallbacks": sum(r["review"] for r in substantive),
        "expected_manual_cases": len(manual),
        "correct_manual_cases": sum(r["review"] for r in manual),
        "generated_names": len(named),
        "generated_names_passing_automated_checks": sum(r["automated_title_check"] for r in named),
        "all_name_checks_passed": sum(r["automated_title_check"] for r in substantive),
        "refusal_or_guardrail_cases": sum(any(token in r["decision"]["reason"] for token in ["(refused)", "(guardrail)"]) for r in rows),
        "median_seconds": report["summary"]["median_seconds"],
    }
reports = [json.loads(Path(p).read_text()) for p in sys.argv[1:]]
assert all([r["id"] for r in report["results"]] == [r["id"] for r in reports[0]["results"]] for report in reports), "Mismatched cases"
for report in reports:
    print(report["pipeline"], json.dumps(summary(report), indent=2))
if len(reports) == 2:
    for before, after in zip(reports[0]["results"], reports[1]["results"]):
        if (before["decision"]["folder"], before["decision"]["title"]) != (after["decision"]["folder"], after["decision"]["title"]):
            print(json.dumps({"case": before["id"], "expected": before["expected_folder"], "before": before["decision"], "after": after["decision"]}))
