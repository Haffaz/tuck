import { useState } from "react";
import type { RuleIssue } from "./types";

export function exampleText(
  rule: string,
  belongs: string,
  excludes: string,
): string {
  const clean = (s: string) => s.trim().replace(/\s+/g, " ");
  if (!clean(belongs) || belongs.length > 240 || excludes.length > 240)
    throw new Error("Describe the example in 240 characters or fewer.");
  const block = `## Filing example\nBelongs here: ${clean(belongs)}${clean(excludes) ? `\nDoes not belong here: ${clean(excludes)}` : ""}`;
  if (rule.includes(block))
    throw new Error("This example is already in the rule.");
  if ((rule.match(/^## Filing example$/gm) || []).length >= 3)
    throw new Error(
      "This folder already has three examples. Edit or replace one in Folder rules.",
    );
  return `${rule.trim()}\n\n${block}`;
}

export function ExampleDraft({
  folder,
  title,
  disabled,
  onAdd,
  onCancel,
}: {
  folder: string;
  title: string;
  disabled: boolean;
  onAdd: (belongs: string, excludes: string) => void;
  onCancel: () => void;
}) {
  const [belongs, setBelongs] = useState(title.slice(0, 240));
  const [excludes, setExcludes] = useState("");
  return (
    <section className="learning-panel" aria-label="Remember a filing example">
      <h3>Help Tuck recognize this next time.</h3>
      <p>
        Describe what belongs in <strong>{folder}</strong>. Add the context that
        makes this filing choice useful. Only this text will be added to your
        rules.
      </p>
      <label>
        Belongs here
        <textarea
          aria-label="Example that belongs here"
          rows={2}
          maxLength={240}
          value={belongs}
          disabled={disabled}
          onChange={(e) => setBelongs(e.target.value)}
          placeholder="A monthly bank statement showing account transactions."
        />
      </label>
      <label>
        <span>Does not belong here (optional)</span>
        <textarea
          aria-label="Example that does not belong here"
          rows={2}
          maxLength={240}
          value={excludes}
          disabled={disabled}
          onChange={(e) => setExcludes(e.target.value)}
          placeholder="An unrelated monthly bank statement."
        />
      </label>
      <p>
        You can edit or remove examples in Folder rules. Existing suggestions
        need analysis again after you save a rule change.
      </p>
      <div className="mvp-actions">
        <button className="text-button" disabled={disabled} onClick={onCancel}>
          Cancel example
        </button>
        <button
          className="secondary compact"
          disabled={disabled || !belongs.trim()}
          onClick={() => onAdd(belongs, excludes)}
        >
          Add to rule draft
        </button>
      </div>
    </section>
  );
}

export function RuleCheckResults({
  issues,
  stale,
  disabled,
  onAdd,
  onDismiss,
}: {
  issues: RuleIssue[];
  stale: boolean;
  disabled: boolean;
  onAdd: (index: number, text: string) => void;
  onDismiss: () => void;
}) {
  const [edits, setEdits] = useState<Record<number, string>>({});
  return (
    <section className="learning-panel" aria-label="Rule check results">
      <h2>
        {issues.length
          ? "Some folders have the same rule."
          : "No duplicate rules found."}
      </h2>
      <p>
        {stale
          ? "Your rules changed. Run Check rules again for fresh suggestions."
          : "This checks matching descriptions in the same branch. Different wording can still overlap. Add a distinction, then Save rules."}
      </p>
      {issues.map((issue, i) => (
        <div className="rule-check-issue" key={i}>
          <h3>
            {issue.folder} · {issue.otherFolder}
          </h3>
          <p>{issue.issue}</p>
          <label>
            Clarification for {issue.folder}
            <textarea
              rows={2}
              aria-label={`Clarification ${i + 1}`}
              disabled={disabled || stale}
              maxLength={600}
              value={edits[i] ?? issue.clarification}
              onChange={(e) => setEdits({ ...edits, [i]: e.target.value })}
              placeholder="Describe which documents belong here."
            />
          </label>
          <button
            className="secondary compact"
            disabled={
              disabled || stale || !(edits[i] ?? issue.clarification).trim()
            }
            onClick={() => onAdd(i, edits[i] ?? issue.clarification)}
          >
            Add clarification to draft
          </button>
        </div>
      ))}
      <button className="text-button" disabled={disabled} onClick={onDismiss}>
        Dismiss check
      </button>
    </section>
  );
}
