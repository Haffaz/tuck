import { useState } from "react";
import {
  ChevronRight,
  FileText,
  FolderTree,
  Inbox,
  Pencil,
  X,
} from "lucide-react";
import { FolderTile } from "../components/look";
import RenameField from "../components/RenameField";
import Spread from "../components/Spread";
import { nameOf, plural, ordered, parentOf, isWithin } from "../folders";
import type { CategoryProposal, Placement } from "../types";

type ProposalProps = {
  proposal: CategoryProposal;
  disabled: boolean;
  watch: boolean;
  onWatch: (watch: boolean) => void;
  onChange: (proposal: CategoryProposal) => void;
  onOrganize: () => void;
  onDiscard: () => void;
};

/** Suggested folders with their files; everything stays a draft until Organize. */
export function Proposal({
  proposal,
  disabled,
  watch,
  onWatch,
  onChange,
  onOrganize,
  onDiscard,
}: ProposalProps) {
  const [open, setOpen] = useState<string | null>(null);
  const [renaming, setRenaming] = useState<string | null>(null);
  const files = proposal.files || [];
  const folders = ordered(proposal.rules);
  const unplaced = proposal.reviewFiles || [];
  const total = files.length + unplaced.length;
  const inFolder = (folder: string) => files.filter((f) => f.folder === folder);
  const countUnder = (folder: string) =>
    files.filter((f) => isWithin(f.folder, folder)).length;

  /** Why a folder name can't be used, or "" when it can. */
  const problem = (from: string, name: string) => {
    if (!name) return "Enter a name.";
    if (name.startsWith(".") || /[\\/:*?"<>|]/.test(name))
      return 'Use letters, numbers and spaces; no / \\ : * ? " < > | or leading dot.';
    const target = parentOf(from) + name;
    if (
      target.toLowerCase() !== from.toLowerCase() &&
      proposal.rules.some(
        (r) => r.folder.toLowerCase() === target.toLowerCase(),
      )
    )
      return "Another folder already has this name.";
    return "";
  };
  const rename = (from: string, name: string) => {
    const target = parentOf(from) + name;
    const move = (path: string) =>
      isWithin(path, from) ? target + path.slice(from.length) : path;
    if (open && isWithin(open, from)) setOpen(move(open));
    onChange({
      ...proposal,
      rules: proposal.rules.map((r) => ({ ...r, folder: move(r.folder) })),
      files: files.map((f) => ({ ...f, folder: move(f.folder) })),
    });
  };
  const remove = (folder: string) => {
    const gone = (path: string) => isWithin(path, folder);
    // Files of a removed subfolder go to its parent when the parent is a folder too.
    const parent = parentOf(folder).replace(/\/$/, "");
    const keep = !!parent && proposal.rules.some((r) => r.folder === parent);
    onChange({
      ...proposal,
      rules: proposal.rules.filter((r) => !gone(r.folder)),
      files: keep
        ? files.map((f) => (gone(f.folder) ? { ...f, folder: parent } : f))
        : files.filter((f) => !gone(f.folder)),
      reviewFiles: keep
        ? unplaced
        : [
            ...unplaced,
            ...files
              .filter((f) => gone(f.folder))
              .map((f) => ({
                path: f.path,
                reason: "Its folder was removed.",
              })),
          ],
    });
  };
  const describe = (folder: string, instruction: string) =>
    onChange({
      ...proposal,
      rules: proposal.rules.map((r) =>
        r.folder === folder ? { ...r, instruction } : r,
      ),
    });
  const moveFile = (file: Placement, folder: string) =>
    onChange({
      ...proposal,
      files: files.map((f) => (f.path === file.path ? { ...f, folder } : f)),
    });

  return (
    <section aria-label="Suggested folders">
      <header className="list-heading">
        <div>
          <h2>Suggested folders</h2>
          <p>
            {plural(
              folders.filter((r) => !r.folder.includes("/")).length,
              "folder",
            )}{" "}
            for {plural(total, "file")}. Rename, remove or move anything before
            organizing.
          </p>
        </div>
      </header>
      <Spread
        counts={folders
          .filter((r) => !r.folder.includes("/"))
          .map((r) => [r.folder, countUnder(r.folder)])}
      />
      <ul className="folders">
        {folders.map((rule) => {
          const child = rule.folder.includes("/");
          const expanded = open === rule.folder;
          return (
            <li key={rule.folder} className={child ? "child" : undefined}>
              <div className="folder-row">
                <button
                  className="disclosure"
                  aria-expanded={expanded}
                  aria-label={`Show files in ${rule.folder}`}
                  onClick={() => setOpen(expanded ? null : rule.folder)}
                >
                  <ChevronRight
                    size={14}
                    className={expanded ? "turned" : ""}
                  />
                </button>
                <FolderTile path={nameOf(rule.folder)} size={child ? 26 : 30} />
                {renaming === rule.folder ? (
                  <RenameField
                    folder={rule.folder}
                    problem={(name) => problem(rule.folder, name)}
                    onDone={(name) => {
                      if (name !== null) rename(rule.folder, name);
                      setRenaming(null);
                    }}
                  />
                ) : (
                  <span className="folder-name">
                    <span>{nameOf(rule.folder)}</span>
                    {!!rule.documentTypes?.length && (
                      <small>
                        {rule.documentTypes.slice(0, 3).join(" · ")}
                      </small>
                    )}
                  </span>
                )}
                <span className="count">{countUnder(rule.folder)}</span>
                <button
                  className="icon"
                  disabled={disabled}
                  aria-label={`Rename ${rule.folder}`}
                  onClick={() => setRenaming(rule.folder)}
                >
                  <Pencil size={13} />
                </button>
                <button
                  className="icon"
                  disabled={disabled}
                  aria-label={`Remove ${rule.folder}`}
                  onClick={() => remove(rule.folder)}
                >
                  <X size={14} />
                </button>
              </div>
              {expanded && (
                <div className="folder-detail">
                  <label>
                    What belongs here
                    <textarea
                      rows={2}
                      aria-label={`What belongs in ${rule.folder}`}
                      value={rule.instruction}
                      onChange={(e) => describe(rule.folder, e.target.value)}
                    />
                  </label>
                  <ul className="files">
                    {inFolder(rule.folder).map((file) => (
                      <li key={file.path}>
                        <FileText size={13} aria-hidden="true" />
                        <span title={file.path}>
                          {file.title || nameOf(file.path)}
                        </span>
                        <select
                          aria-label={`Folder for ${nameOf(file.path)}`}
                          value={file.folder}
                          onChange={(e) => moveFile(file, e.target.value)}
                        >
                          {folders.map((r) => (
                            <option key={r.folder} value={r.folder}>
                              {r.folder}
                            </option>
                          ))}
                        </select>
                      </li>
                    ))}
                    {!inFolder(rule.folder).length && (
                      <li className="muted">No files yet.</li>
                    )}
                  </ul>
                </div>
              )}
            </li>
          );
        })}
      </ul>
      {!!unplaced.length && (
        <details className="unplaced">
          <summary>
            <Inbox size={14} aria-hidden="true" />
            {plural(unplaced.length, "file")} will wait for you
          </summary>
          <ul>
            {unplaced.map((file) => (
              <li key={file.path}>
                <strong>{nameOf(file.path)}</strong> <span>{file.reason}</span>
              </li>
            ))}
          </ul>
        </details>
      )}
      <footer className="actions">
        <label className="check">
          <input
            type="checkbox"
            checked={watch}
            onChange={(e) => onWatch(e.target.checked)}
          />
          Keep watching for new files
        </label>
        <button className="link" disabled={disabled} onClick={onDiscard}>
          Start over
        </button>
        <button
          className="primary"
          disabled={disabled || !proposal.rules.length}
          onClick={onOrganize}
        >
          <FolderTree size={15} aria-hidden="true" />
          Organize {plural(files.length, "file")}
        </button>
      </footer>
    </section>
  );
}
