import { useState, type ReactNode } from "react";
import {
  ChevronRight,
  Folder,
  FolderOpen,
  Plus,
  SlidersHorizontal,
  Sparkles,
  Trash2,
} from "lucide-react";
import type { Rule } from "./types";

type Branch = {
  path: string;
  name: string;
  index?: number;
  children: Branch[];
};
type Props = {
  root: string;
  rules: Rule[];
  rootRule: string;
  suggested?: boolean;
  disabled: boolean;
  onRules: (rules: Rule[]) => void;
  onRootRule: (value: string) => void;
  onSuggest?: (index: number) => void;
};

// Build a display hierarchy without reordering rules: their order determines precedence.
function hierarchy(rules: Rule[]) {
  const roots: Branch[] = [];
  rules.forEach((rule, index) => {
    const parts = rule.folder.split("/").filter(Boolean);
    if (!parts.length) {
      roots.push({
        path: `draft-${index}`,
        name: "New folder",
        index,
        children: [],
      });
      return;
    }
    let siblings = roots;
    parts.forEach((name, depth) => {
      const path = parts.slice(0, depth + 1).join("/");
      let branch = siblings.find((entry) => entry.path === path);
      if (!branch) {
        branch = { path, name, children: [] };
        siblings.push(branch);
      }
      if (depth === parts.length - 1) branch.index = index;
      siblings = branch.children;
    });
  });
  return roots;
}

export default function FolderTree({
  root,
  rules,
  rootRule,
  suggested = false,
  disabled,
  onRules,
  onRootRule,
  onSuggest,
}: Props) {
  const [selection, setSelection] = useState<number | string | null>(
    suggested && rules.length ? 0 : null,
  );
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const rule = typeof selection === "number" ? rules[selection] : undefined;
  const rootSelected = selection === "root";
  const group =
    typeof selection === "string" && selection !== "root" ? selection : null;
  const blockedBy = (path: string) =>
    rules.find((r) => !r.enabled && path.startsWith(`${r.folder}/`));
  const update = (patch: Partial<Rule>) => {
    if (patch.instruction !== undefined) patch.documentTypes = [];
    onRules(rules.map((r, i) => (i === selection ? { ...r, ...patch } : r)));
  };
  const add = (path = "") => {
    onRules([...rules, { folder: path, instruction: "", enabled: true }]);
    setSelection(rules.length);
    setCollapsed(new Set());
  };
  const draw = (nodes: Branch[]): ReactNode => (
    <ul className="folder-branches">
      {nodes.map((node) => {
        const selected = selection === (node.index ?? node.path);
        const inactive =
          (node.index !== undefined && !rules[node.index].enabled) ||
          !!blockedBy(node.path);
        return (
          <li key={node.path}>
            <div
              className={`folder-tree-row ${selected ? "selected" : ""} ${inactive ? "inactive" : ""}`}
            >
              {node.children.length > 0 ? (
                <button
                  className="branch-toggle"
                  aria-label={`${collapsed.has(node.path) ? "Expand" : "Collapse"} ${node.path}`}
                  aria-expanded={!collapsed.has(node.path)}
                  onClick={() =>
                    setCollapsed((current) => {
                      const next = new Set(current);
                      if (next.has(node.path)) next.delete(node.path);
                      else next.add(node.path);
                      return next;
                    })
                  }
                >
                  <ChevronRight size={13} />
                </button>
              ) : (
                <span className="branch-spacer" />
              )}
              <button
                className="folder-tree-select"
                aria-label={`Rules for ${node.path}`}
                aria-pressed={selected}
                onClick={() => setSelection(node.index ?? node.path)}
              >
                <Folder size={17} />
                <span>{node.name}</span>
                {inactive && <small>Off</small>}
                {suggested && !inactive && (
                  <span className="suggestion-dot" aria-label="Suggested" />
                )}
              </button>
            </div>
            {!!node.children.length &&
              !collapsed.has(node.path) &&
              draw(node.children)}
          </li>
        );
      })}
    </ul>
  );
  return (
    <section
      className={`folder-tree ${suggested ? "is-proposal" : ""}`}
      aria-label={suggested ? "Suggested folder tree" : "Folder tree"}
    >
      <div className="folder-tree-browser">
        <div className="folder-tree-caption">
          <span>{suggested ? "PROPOSED STRUCTURE" : "YOUR STRUCTURE"}</span>
          <span>
            {rules.length} {rules.length === 1 ? "category" : "categories"}
          </span>
        </div>
        <button
          className={`folder-tree-root ${rootSelected ? "selected" : ""}`}
          aria-label="Rules for all documents"
          aria-pressed={rootSelected}
          onClick={() => setSelection("root")}
        >
          <FolderOpen size={20} />
          <span>{root.split("/").filter(Boolean).pop() || "Documents"}</span>
          <small>Root</small>
        </button>
        {rules.length ? (
          draw(hierarchy(rules))
        ) : (
          <p className="folder-tree-empty">
            A little order starts here.
            <br />
            Add a folder, or let Tuck suggest a few.
          </p>
        )}
        <button className="tree-add" disabled={disabled} onClick={() => add()}>
          <Plus size={14} /> Add folder
        </button>
      </div>
      <div className="folder-rule-panel" key={selection ?? "empty"}>
        {rule || rootSelected ? (
          <>
            <div className="folder-rule-heading">
              <span className="folder-rule-icon">
                {rootSelected ? (
                  <SlidersHorizontal size={19} />
                ) : (
                  <FolderOpen size={19} />
                )}
              </span>
              <div>
                <span className="folder-rule-kicker">
                  {rootSelected
                    ? "APPLIES EVERYWHERE"
                    : suggested
                      ? "SUGGESTED CATEGORY"
                      : "FOLDER RULE"}
                </span>
                <h4>
                  {rootSelected
                    ? "Document instructions"
                    : rule!.folder.split("/").pop() || "New folder"}
                </h4>
              </div>
            </div>
            {rule && (
              <label className="tree-field">
                Folder path
                <input
                  aria-label={
                    suggested
                      ? `Suggested folder ${Number(selection) + 1}`
                      : "Category path"
                  }
                  value={rule.folder}
                  placeholder="e.g. Housing & Utilities"
                  disabled={disabled}
                  onChange={(e) => update({ folder: e.target.value })}
                />
              </label>
            )}
            <label className="tree-field">
              {rootSelected ? "Document instructions" : "What belongs here?"}
              <textarea
                aria-label={
                  rootSelected
                    ? "Document instructions"
                    : suggested
                      ? `Suggested rule ${Number(selection) + 1}`
                      : `Rule for ${rule!.folder || "new folder"}`
                }
                value={rootSelected ? rootRule : rule!.instruction}
                placeholder={
                  rootSelected
                    ? "For example: Use short English names."
                    : "e.g. Rental agreements and household utility bills belong here."
                }
                rows={4}
                disabled={disabled}
                onChange={(e) =>
                  rootSelected
                    ? onRootRule(e.target.value)
                    : update({ instruction: e.target.value })
                }
              />
            </label>
            {!!rule?.documentTypes?.length && (
              <details className="tree-scope">
                <summary>Document types ({rule.documentTypes.length})</summary>
                <ul>
                  {rule.documentTypes.map((type) => (
                    <li key={type}>{type}</li>
                  ))}
                </ul>
                <p>Editing the description replaces this suggested scope.</p>
              </details>
            )}
            {rule && onSuggest && !rule.instruction.trim() && (
              <button
                className="text-button"
                disabled={disabled || !rule.folder.trim()}
                onClick={() => onSuggest(Number(selection))}
              >
                <Sparkles size={13} /> Suggest description
              </button>
            )}
            {rule && (
              <>
                <div className="tree-rule-footer">
                  <label>
                    <input
                      type="checkbox"
                      aria-label={`Enable category ${rule.folder || "new folder"}`}
                      checked={rule.enabled}
                      disabled={disabled}
                      onChange={(e) => update({ enabled: e.target.checked })}
                    />{" "}
                    Use this category
                  </label>
                  <button
                    className="icon-button"
                    aria-label={
                      suggested
                        ? `Remove suggestion ${Number(selection) + 1}`
                        : `Remove category ${rule.folder || "new folder"}`
                    }
                    disabled={disabled}
                    onClick={() => {
                      onRules(rules.filter((_, i) => i !== selection));
                      setSelection(null);
                    }}
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
                {blockedBy(rule.folder) && (
                  <p className="tree-rule-hint">
                    Turn on {blockedBy(rule.folder)!.folder} to use this child
                    folder.
                  </p>
                )}
              </>
            )}
            <p className="tree-rule-hint">
              {rootSelected
                ? "Used when organizing and naming every document."
                : "A sentence or two is enough. Be specific about what belongs here."}
            </p>
          </>
        ) : group ? (
          <>
            <FolderOpen size={28} />
            <h4>{group.split("/").pop()}</h4>
            <p className="tree-rule-hint">
              This folder groups the categories below it. Add a rule if
              documents should also go here.
            </p>
            <button
              className="secondary compact"
              disabled={disabled}
              onClick={() => add(group)}
            >
              Add a rule here
            </button>
          </>
        ) : (
          <div className="tree-invitation">
            <span className="tree-invitation-art">
              <FolderOpen size={32} />
              <Sparkles size={15} />
            </span>
            <h4>A place for everything.</h4>
            <p>
              Select a folder to shape its rules.
              <br />
              Tuck takes care of the sorting.
            </p>
          </div>
        )}
      </div>
    </section>
  );
}
