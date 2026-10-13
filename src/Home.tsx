import { useRef, useState } from "react";
import {
  ChevronRight,
  CircleCheck,
  Eye,
  FileText,
  FolderOpen,
  FolderTree,
  Inbox,
  LoaderCircle,
  Pencil,
  ScanText,
  Sparkles,
  X,
} from "lucide-react";
import { FolderTile, folderLook, tint } from "./look";
import type {
  CategoryProposal,
  Placement,
  ReviewItem,
  Rule,
  Snapshot,
} from "./types";

export const nameOf = (path: string) =>
  path.split("/").filter(Boolean).pop() || path;
const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

/** Folder paths in display order: each parent followed by its children. */
function ordered(rules: Rule[]): Rule[] {
  const parents = rules.filter((r) => !r.folder.includes("/"));
  const orphans = rules.filter(
    (r) =>
      r.folder.includes("/") &&
      !parents.some((p) => r.folder.startsWith(`${p.folder}/`)),
  );
  return [
    ...parents.flatMap((p) => [
      p,
      ...rules.filter((r) => r.folder.startsWith(`${p.folder}/`)),
    ]),
    ...orphans,
  ];
}

const parentOf = (path: string) =>
  path.includes("/") ? path.slice(0, path.lastIndexOf("/") + 1) : "";

/** Inline rename: Enter or leaving the field saves a valid name; Escape cancels. */
function RenameField({
  folder,
  problem,
  onDone,
}: {
  folder: string;
  problem: (name: string) => string;
  onDone: (name: string | null) => void;
}) {
  const [value, setValue] = useState(nameOf(folder));
  const cancelled = useRef(false);
  const error = problem(value.trim());
  const finish = () => {
    if (cancelled.current) return;
    onDone(error ? null : value.trim());
  };
  return (
    <span className="rename">
      <input
        autoFocus
        aria-label={`Name for ${folder}`}
        aria-invalid={!!error}
        aria-describedby={error ? "rename-error" : undefined}
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onBlur={finish}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !error) e.currentTarget.blur();
          if (e.key === "Escape") {
            e.stopPropagation();
            cancelled.current = true;
            onDone(null);
          }
        }}
      />
      {error && (
        <small id="rename-error" role="alert">
          {error}
        </small>
      )}
    </span>
  );
}

const steps = [
  { Icon: ScanText, title: "Reads", text: "Every document, on this Mac" },
  { Icon: Sparkles, title: "Suggests", text: "Folders that fit what you have" },
  { Icon: FolderTree, title: "Organizes", text: "Only when you say so" },
];

export function ChooseFolder({
  disabled,
  onChoose,
}: {
  disabled: boolean;
  onChoose: () => void;
}) {
  return (
    <section className="hero">
      <span className="hero-tile" aria-hidden="true">
        <FolderOpen size={34} strokeWidth={1.6} />
      </span>
      <h1>A place for every document</h1>
      <p>
        Choose a messy folder. Tuck reads it and suggests a tidy home for each
        file.
      </p>
      <button className="primary large" disabled={disabled} onClick={onChoose}>
        <FolderOpen size={16} aria-hidden="true" />
        Choose folder
      </button>
      <ol className="steps">
        {steps.map(({ Icon, title, text }) => (
          <li key={title}>
            <Icon size={18} aria-hidden="true" />
            <strong>{title}</strong>
            <span>{text}</span>
          </li>
        ))}
      </ol>
    </section>
  );
}

const phases = ["Reading", "Suggesting", "Moving"];

/** Progress for long steps; shows a bar when the message says "N of M". */
export function Working({ message }: { message: string }) {
  const match = message.match(/(\d+) of (\d+)/);
  const progress = match
    ? Number(match[1]) / Math.max(1, Number(match[2]))
    : null;
  const phase = Math.max(
    0,
    phases.findIndex((p) => message.startsWith(p)),
  );
  return (
    <section className="working" role="status">
      <span className="working-tile" aria-hidden="true">
        <LoaderCircle size={22} className="spin" />
      </span>
      <h2>{message}</h2>
      <div className="bar" aria-hidden="true">
        <span
          className={progress === null ? "indeterminate" : undefined}
          style={
            progress === null
              ? undefined
              : { width: `${Math.round(progress * 100)}%` }
          }
        />
      </div>
      <ol className="phases" aria-hidden="true">
        {phases.map((name, index) => (
          <li
            key={name}
            className={index < phase ? "done" : index === phase ? "now" : ""}
          >
            {name}
          </li>
        ))}
      </ol>
    </section>
  );
}

/** How files split across top-level folders, as one coloured bar. */
function Spread({ counts }: { counts: [string, number][] }) {
  const total = counts.reduce((sum, [, n]) => sum + n, 0);
  if (!total) return null;
  return (
    <div className="spread" aria-hidden="true">
      {counts
        .filter(([, n]) => n)
        .map(([folder, n]) => (
          <span
            key={folder}
            title={`${folder}: ${n}`}
            style={{ ...tint(folderLook(folder).tint), flexGrow: n }}
          />
        ))}
    </div>
  );
}

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
    files.filter(
      (f) => f.folder === folder || f.folder.startsWith(`${folder}/`),
    ).length;

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
      path === from || path.startsWith(`${from}/`)
        ? target + path.slice(from.length)
        : path;
    if (open && (open === from || open.startsWith(`${from}/`)))
      setOpen(move(open));
    onChange({
      ...proposal,
      rules: proposal.rules.map((r) => ({ ...r, folder: move(r.folder) })),
      files: files.map((f) => ({ ...f, folder: move(f.folder) })),
    });
  };
  const remove = (folder: string) => {
    const gone = (path: string) =>
      path === folder || path.startsWith(`${folder}/`);
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

/** A folder that already has Tuck folders: file new documents into them, or start over. */
export function SavedFolders({
  state,
  disabled,
  watch,
  onWatch,
  onOrganize,
  onSuggest,
}: {
  state: Snapshot;
  disabled: boolean;
  watch: boolean;
  onWatch: (watch: boolean) => void;
  onOrganize: () => void;
  onSuggest: () => void;
}) {
  const counts = new Map(
    (state.collection?.folders || []).map((f) => [f.path, f.documents]),
  );
  return (
    <section aria-label="Your folders">
      <header className="list-heading">
        <div>
          <h2>Your folders</h2>
          <p>New files in this folder go where they belong.</p>
        </div>
        <button className="link" disabled={disabled} onClick={onSuggest}>
          <Sparkles size={13} aria-hidden="true" />
          Suggest again
        </button>
      </header>
      <Spread
        counts={state.preferences.rules
          .filter((r) => !r.folder.includes("/"))
          .map((r) => [
            r.folder,
            [...counts]
              .filter(
                ([path]) =>
                  path === r.folder || path.startsWith(`${r.folder}/`),
              )
              .reduce((sum, [, n]) => sum + n, 0),
          ])}
      />
      <ul className="folders">
        {ordered(state.preferences.rules).map((rule) => (
          <li
            key={rule.folder}
            className={rule.folder.includes("/") ? "child" : undefined}
          >
            <div className="folder-row">
              <FolderTile
                path={nameOf(rule.folder)}
                size={rule.folder.includes("/") ? 26 : 30}
              />
              <span className="folder-name">{nameOf(rule.folder)}</span>
              <span className="count">{counts.get(rule.folder) ?? 0}</span>
            </div>
          </li>
        ))}
      </ul>
      <footer className="actions">
        <label className="check">
          <input
            type="checkbox"
            checked={watch}
            onChange={(e) => onWatch(e.target.checked)}
          />
          Keep watching for new files
        </label>
        <button className="primary" disabled={disabled} onClick={onOrganize}>
          Organize new files
        </button>
      </footer>
    </section>
  );
}

type ResultsProps = {
  state: Snapshot;
  disabled: boolean;
  onFile: (item: ReviewItem, folder: string) => void;
  onSkip: (item: ReviewItem) => void;
  onPreview: (item: ReviewItem) => void;
  onReveal: () => void;
  onStop: () => void;
  onDone: () => void;
};

/** After organizing: a one-line summary and only the files that need a decision. */
export function Results({
  state,
  disabled,
  onFile,
  onSkip,
  onPreview,
  onReveal,
  onStop,
  onDone,
}: ResultsProps) {
  const [choices, setChoices] = useState<Record<string, string>>({});
  const filed = state.items.filter((i) => i.status === "organized").length;
  const busy = state.items.filter(
    (i) => i.status === "queued" || i.status === "reading",
  ).length;
  const waiting = state.items.filter(
    (i) => (i.status === "review" && !i.confident) || i.status === "failed",
  );
  const folders = state.preferences.rules
    .filter((r) => r.enabled)
    .map((r) => r.folder);
  const byFolder = new Map<string, number>();
  for (const item of state.items)
    if (item.status === "organized") {
      const top = item.folder.split("/")[0];
      byFolder.set(top, (byFolder.get(top) || 0) + 1);
    }
  return (
    <section aria-label="Results">
      <header className="summary">
        <span className={`done-tile${busy ? " busy" : ""}`} aria-hidden="true">
          {busy ? (
            <LoaderCircle size={22} className="spin" />
          ) : (
            <CircleCheck size={24} />
          )}
        </span>
        <div className="summary-text">
          <h2>{plural(filed, "file")} organized</h2>
          <p>
            {busy
              ? `Reading ${plural(busy, "more file")}…`
              : state.watching
                ? "Watching for new files."
                : waiting.length
                  ? `${plural(waiting.length, "file")} need${waiting.length === 1 ? "s" : ""} you.`
                  : "All done."}
          </p>
        </div>
        <div className="summary-actions">
          <button className="link" onClick={onReveal}>
            <Eye size={13} aria-hidden="true" />
            Show in Finder
          </button>
          {state.watching ? (
            <button className="secondary" disabled={disabled} onClick={onStop}>
              Stop watching
            </button>
          ) : (
            <button
              className="secondary"
              disabled={disabled || !!busy}
              onClick={onDone}
            >
              Done
            </button>
          )}
        </div>
      </header>
      {!!byFolder.size && (
        <ul className="chips" aria-label="Organized by folder">
          {[...byFolder].map(([folder, n]) => (
            <li key={folder}>
              <FolderTile path={folder} size={26} />
              {folder}
              <span>{n}</span>
            </li>
          ))}
        </ul>
      )}
      {!!waiting.length && <h3 className="section-title">Needs you</h3>}
      {!!waiting.length && (
        <ul className="waiting">
          {waiting.map((item) => {
            const picked = choices[item.id] ?? item.folder;
            const folder = folders.includes(picked) ? picked : "";
            return (
              <li key={item.id}>
                <button
                  className="file-name"
                  title={item.reason}
                  onClick={() => onPreview(item)}
                >
                  <FileText size={14} aria-hidden="true" />
                  {nameOf(item.original)}
                </button>
                <span className="reason">{item.reason}</span>
                <select
                  aria-label={`Folder for ${nameOf(item.original)}`}
                  value={folder}
                  onChange={(e) =>
                    setChoices({ ...choices, [item.id]: e.target.value })
                  }
                >
                  <option value="">Choose folder…</option>
                  {folders.map((f) => (
                    <option key={f} value={f}>
                      {f}
                    </option>
                  ))}
                </select>
                <button
                  className="secondary"
                  disabled={disabled || !folder || item.status !== "review"}
                  onClick={() => onFile(item, folder)}
                >
                  File
                </button>
                <button
                  className="link"
                  disabled={disabled}
                  onClick={() => onSkip(item)}
                >
                  Skip
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
