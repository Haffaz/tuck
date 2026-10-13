"""Converts the "diverse" fixtures into the card corpus layout.

Gold folder is the fixture subject; fixtures without a subject are intentionally unclear
or mixed and have gold "Needs Review". Rules reuse the suite's own one-level wording.
    python3 evals/cards/from_fixtures.py OUT_DIR
"""
import json, sys
from pathlib import Path

SUITES = ["diverse", "diverse-heldout"]
ROOT = Path(__file__).resolve().parents[1] / "fixtures"

def main(out: Path):
    if out.exists():
        sys.exit("Use a new output directory")
    (out / "files").mkdir(parents=True)
    documents, subjects = [], []
    for suite in SUITES:
        for fixture in json.loads((ROOT / suite / "fixtures.json").read_text()):
            name = f"{suite}-{fixture['id']}.txt"
            (out / "files" / name).write_text(fixture["text"], encoding="utf-8")
            subject = fixture.get("subject") or "Needs Review"
            documents.append({"file": name, "folder": subject, "kind": fixture.get("kind"), "format": "txt", "chars": len(fixture["text"])})
            if fixture.get("subject") and subject not in subjects:
                subjects.append(subject)
    rules = [{"folder": s, "instruction": f"File {s} documents here. Other subjects have their own folders."} for s in subjects]
    (out / "gold.json").write_text(json.dumps({"rules": rules, "documents": documents}, indent=1))
    print(f"{len(documents)} documents, {len(subjects)} subjects in {out}")

if __name__ == "__main__":
    main(Path(sys.argv[1]).resolve())
