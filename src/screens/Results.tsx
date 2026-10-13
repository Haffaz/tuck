import { useState } from "react";
import { CircleCheck, Eye, FileText, LoaderCircle } from "lucide-react";
import { FolderTile } from "../components/look";
import { nameOf, plural } from "../folders";
import type { ReviewItem, Snapshot } from "../types";

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
