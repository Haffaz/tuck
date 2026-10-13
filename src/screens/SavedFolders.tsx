import { Sparkles } from "lucide-react";
import { FolderTile } from "../components/look";
import Spread from "../components/Spread";
import { nameOf, ordered, isWithin } from "../folders";
import type { Snapshot } from "../types";

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
              .filter(([path]) => isWithin(path, r.folder))
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
