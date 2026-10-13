# Folder rules

Each folder Tuck files into has a short rule in plain Markdown, stored next to the documents. The
files are the source of truth: you can read and edit them in any text editor, move the whole folder
to another Mac, or keep it in version control. There is no database.

```text
Documents/                          ← the folder you chose
├── .tuck/
│   ├── settings.json               ← model, folder levels, renaming
│   ├── instructions.md             ← filing instructions for every document
│   └── cache/cards/                ← rebuildable document cards
├── Finance/
│   ├── .tuck/instructions.md       ← "Documents such as bank statement, tax return."
│   └── Payslips/
│       └── .tuck/instructions.md
└── Home/
    └── .tuck/instructions.md
```

## A rule file

```markdown
---
enabled: true
priority: 0
document-types: ["bank statement","credit card statement"]
scope-hash: 3f5c…
---

Documents such as bank statement, credit card statement.
```

- The text after the front matter is the rule the model sees when it chooses a folder. Write it as
  a short sentence about what belongs here; Tuck generates one from the documents it grouped.
- `enabled` (default `true`) turns a folder on or off. Turning a parent off turns off its whole
  branch, and each child keeps its own setting for when the parent is turned on again.
- `priority` orders folders; lower comes first.
- `document-types` lists the kinds of documents the suggestion found in the folder. `scope-hash` ties
  that list to the rule text: if you edit the text, the list is ignored, so an old scope never
  overrides your wording.

The front matter is deliberately small, not full YAML. Unknown or repeated keys are errors, so a
typo is reported instead of silently ignored. An enabled folder needs a non-empty rule, and each file
is limited to 64 KB.

The root `.tuck/instructions.md` has no front matter. Its text applies to every document, for
example "Use short English filenames."

## Inheritance

When Tuck chooses a folder for a document, each candidate is shown with its own rule and its parents'
rules, so `Finance/Payslips` is described by both. A folder without a rule file is not a destination,
but it can contain folders that are.

## Limits

At most two folder levels and 40 folders. Rule files found deeper (from older versions) stay on disk
and are listed in Settings, but are not used for filing.

These rules guide the model; they are not a guarantee. Rust separately enforces that a document
only moves into an enabled folder inside the chosen root, and never over an existing file.
