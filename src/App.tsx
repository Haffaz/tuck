import { useCallback, useEffect, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import {
  ArrowDown,
  ArrowDownToLine,
  ArrowRight,
  ArrowUp,
  Check,
  CheckCheck,
  ChevronRight,
  CircleHelp,
  FileText,
  Folder,
  FolderOpen,
  History,
  Inbox,
  Leaf,
  LoaderCircle,
  MoreHorizontal,
  Pause,
  Play,
  Plus,
  Settings2,
  ShieldCheck,
  Sparkles,
  Trash2,
  Undo2,
  X,
} from "lucide-react";
import {
  initial,
  type Activity,
  type Preferences,
  type Snapshot,
} from "./types";
import "./App.css";

type Page = "Overview" | "Activity" | "Folder rules" | "Preferences";
const desktop = isTauri();
const nameOf = (path: string) =>
  path.split("/").filter(Boolean).pop() || "Documents";
const shortPath = (path: string) => path.replace(/^\/Users\/[^/]+/, "~");

function Mark({ small = false }: { small?: boolean }) {
  return (
    <span className={`brand-mark ${small ? "small" : ""}`} aria-hidden="true">
      <svg viewBox="0 0 32 32" fill="none">
        <path
          d="M7 11.5 16 7l9 4.5v10L16 26l-9-4.5v-10Z"
          stroke="currentColor"
          strokeWidth="1.7"
          strokeLinejoin="round"
        />
        <path
          d="m7 12 9 4.5 9-4.5M16 17v9M12 9l9 4.5"
          stroke="currentColor"
          strokeWidth="1.7"
          strokeLinejoin="round"
        />
      </svg>
    </span>
  );
}
function Switch({
  checked,
  onChange,
  label,
  disabled = false,
}: {
  checked: boolean;
  onChange: () => void;
  label: string;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={onChange}
      className={`switch ${checked ? "on" : ""}`}
    >
      <span />
    </button>
  );
}
function DropArt() {
  return (
    <div className="drop-art" aria-hidden="true">
      <span className="orbit one" />
      <span className="orbit two" />
      <div className="paper paper-back">
        <span />
        <span />
      </div>
      <div className="paper paper-front">
        <FileText size={22} />
        <span />
        <span />
      </div>
      <div className="folder-art">
        <ArrowDown size={25} strokeWidth={1.8} />
      </div>
      <span className="art-spark">✦</span>
      <span className="art-dot" />
    </div>
  );
}

function App() {
  const [page, setPage] = useState<Page>("Overview");
  const [state, setState] = useState<Snapshot>(initial);
  const [draft, setDraft] = useState<Preferences>(initial.preferences);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [dragging, setDragging] = useState(false);
  const [filter, setFilter] = useState("all");
  const [modelStatus, setModelStatus] = useState("Not checked");
  const [cleanup, setCleanup] = useState(false);
  const [selected, setSelected] = useState<Activity | null>(null);
  const initialized = useRef(false);
  const refresh = useCallback(async () => {
    if (!desktop) return;
    const value = await invoke<Snapshot>("get_state");
    setState(value);
    if (!initialized.current) {
      setDraft(value.preferences);
      initialized.current = true;
    }
  }, []);
  const run = useCallback(
    async (action: () => Promise<unknown>, success = "") => {
      setBusy(true);
      setError("");
      setMessage("");
      try {
        await action();
        await refresh();
        if (success) setMessage(success);
      } catch (e) {
        setError(String(e));
      } finally {
        setBusy(false);
      }
    },
    [refresh],
  );
  useEffect(() => {
    void refresh().catch((e) => setError(String(e)));
    const timer = window.setInterval(() => {
      void refresh().catch((e) => setError(String(e)));
    }, 1500);
    return () => clearInterval(timer);
  }, [refresh]);
  useEffect(() => {
    if (!desktop) return;
    let unlisten: (() => void) | undefined;
    let disposed = false;
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === "over" || event.payload.type === "enter")
          setDragging(true);
        else setDragging(false);
        if (event.payload.type === "drop") {
          void run(
            () =>
              invoke("import_documents", {
                paths: event.payload.type === "drop" ? event.payload.paths : [],
              }),
            "Documents added to the queue.",
          );
        }
      })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch((e) => setError(String(e)));
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [run]);
  useEffect(() => {
    if (message) {
      const timeout = setTimeout(() => setMessage(""), 5000);
      return () => clearTimeout(timeout);
    }
  }, [message]);
  useEffect(() => {
    const close = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setCleanup(false);
        setSelected(null);
      }
    };
    document.addEventListener("keydown", close);
    return () => document.removeEventListener("keydown", close);
  }, []);
  const chooseFolder = () =>
    run(async () => {
      const folder = await open({
        directory: true,
        multiple: false,
        title: "Choose the folder Tuck should organize",
      });
      if (typeof folder === "string") {
        const preferences = { ...state.preferences, root: folder };
        await invoke("save_preferences", { preferences });
        setDraft((previous) => ({ ...previous, root: folder }));
      }
    });
  const addFiles = () =>
    run(async () => {
      const paths = await open({
        multiple: true,
        title: "Add documents to Tuck",
        filters: [
          {
            name: "Documents",
            extensions: [
              "pdf",
              "png",
              "jpg",
              "jpeg",
              "heic",
              "tif",
              "tiff",
              "txt",
              "md",
            ],
          },
        ],
      });
      if (paths)
        await invoke("import_documents", {
          paths: typeof paths === "string" ? [paths] : paths,
        });
    });
  const save = () =>
    run(async () => {
      await invoke("save_preferences", { preferences: draft });
    }, "Preferences saved. New documents will follow these rules.");
  const checkModel = () =>
    run(async () => {
      const models = await invoke<string[]>("check_model");
      setModelStatus(
        models.includes(draft.model)
          ? `${draft.model} is ready`
          : `Ollama connected. Install ${draft.model} to use it.`,
      );
    });
  const dirty = JSON.stringify(draft) !== JSON.stringify(state.preferences);
  const active = state.activity.filter(
    (a) => a.status === "sorted" || a.status === "review",
  );
  const reviewed = active.filter((a) => a.status === "review").length;
  const sorted = active.filter((a) => a.status === "sorted").length;
  const visibleActivity = state.activity.filter(
    (a) => filter === "all" || a.status === filter,
  );
  const disabled = busy || !desktop;
  const updateRule = (
    index: number,
    key: "folder" | "keywords",
    value: string,
  ) =>
    setDraft((prev) => ({
      ...prev,
      rules: prev.rules.map((rule, i) =>
        i === index ? { ...rule, [key]: value } : rule,
      ),
    }));
  const moveRule = (index: number, offset: number) =>
    setDraft((prev) => {
      const rules = [...prev.rules];
      [rules[index], rules[index + offset]] = [
        rules[index + offset],
        rules[index],
      ];
      return { ...prev, rules };
    });

  function activityRows(rows: Activity[]) {
    return rows.map((item) => (
      <div className="activity-row" key={item.id}>
        <span
          className={`file-icon ${item.status === "review" || item.status === "failed" ? "amber" : ""}`}
        >
          <FileText size={19} />
        </span>
        <button className="activity-name" onClick={() => setSelected(item)}>
          <strong>{nameOf(item.destination || item.original)}</strong>
          <span>
            {item.folder || "Original kept"}{" "}
            <span className="separator">·</span>{" "}
            {new Date(item.timestamp * 1000).toLocaleTimeString([], {
              hour: "2-digit",
              minute: "2-digit",
            })}
            {item.duplicate ? " · Duplicate kept" : ""}
          </span>
        </button>
        <span className={`status ${item.status}`}>
          {
            {
              sorted: "Tucked away",
              review: "Needs review",
              undone: "Undone",
              pending: "Sorting",
              undo_pending: "Undoing",
              failed: "Original kept",
            }[item.status]
          }
        </span>
        <button
          className="icon-button"
          aria-label={`Details for ${nameOf(item.original)}`}
          onClick={() => setSelected(item)}
        >
          <MoreHorizontal size={19} />
        </button>
      </div>
    ));
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <Mark />
          <span>
            Tuck<span className="brand-period">.</span>
          </span>
        </div>
        <div className="workspace-label">YOUR WORKSPACE</div>
        <nav aria-label="Main navigation">
          {(
            [
              ["Overview", Inbox],
              ["Activity", History],
              ["Folder rules", Folder],
              ["Preferences", Settings2],
            ] as const
          ).map(([label, Icon]) => (
            <button
              key={label}
              onClick={() => setPage(label)}
              className={`nav-item ${page === label ? "active" : ""}`}
            >
              <Icon size={18} />
              <span>{label}</span>
              {label === "Activity" && reviewed > 0 && <b>{reviewed}</b>}
            </button>
          ))}
        </nav>
        <div className="sidebar-divider" />
        <div className="workspace-label">WATCHED FOLDER</div>
        <button
          className="sidebar-folder"
          disabled={disabled}
          onClick={chooseFolder}
        >
          <FolderOpen size={17} />
          <span>
            {state.preferences.root
              ? nameOf(state.preferences.root)
              : "Choose a folder"}
          </span>
          {state.preferences.root && (
            <span className={`dot ${state.watching ? "live" : ""}`} />
          )}
        </button>
        <div className="sidebar-bottom">
          <div className="local-card">
            <Leaf size={18} />
            <div>
              <strong>Happily on your Mac</strong>
              <p>Your documents stay local.</p>
            </div>
          </div>
          <div className="version">
            <Mark small />
            <span>Tuck for Mac</span>
            <span>v0.1</span>
          </div>
        </div>
      </aside>
      <main>
        <header className="topbar">
          <span>
            Workspace <ChevronRight size={14} /> <strong>{page}</strong>
          </span>
          <span className="privacy-pill">
            <ShieldCheck size={14} /> Local by default
          </span>
        </header>
        <div className="main-content">
          {!desktop && (
            <div className="preview-note">
              Interface preview · Folder access and sorting are available in the
              Tuck desktop app.
            </div>
          )}
          {(error || state.error) && (
            <div className="notice error" role="alert">
              <CircleHelp size={18} />
              <span>{error || state.error}</span>
              <button aria-label="Dismiss error" onClick={() => setError("")}>
                <X size={16} />
              </button>
            </div>
          )}
          {message && (
            <div className="notice success" role="status">
              <Check size={18} />
              {message}
            </div>
          )}
          {page === "Overview" && (
            <>
              <div className="page-heading">
                <div>
                  <div className="eyebrow">A LITTLE LESS CLUTTER</div>
                  <h1>Everything, in its place.</h1>
                  <p>Drop it here. We’ll tuck it away.</p>
                </div>
                <button
                  className="primary"
                  onClick={addFiles}
                  disabled={disabled || !state.preferences.root}
                >
                  <Plus size={17} /> Add documents
                </button>
              </div>
              <section
                className={`drop-zone ${dragging ? "dragging" : ""}`}
                aria-label="Document drop area"
              >
                <div className="drop-copy">
                  <span className="tiny-label">FROM A PILE TO A PLACE</span>
                  <h2>
                    Your documents.
                    <br />A little more organized.
                  </h2>
                  <p>
                    Drop PDFs, scans, and text files here.
                    <br />
                    Tuck finds their home using your rules.
                  </p>
                  <button
                    className="text-button"
                    disabled={disabled}
                    onClick={state.preferences.root ? addFiles : chooseFolder}
                  >
                    {state.preferences.root
                      ? "or browse your files"
                      : "Choose your first folder"}
                    <ArrowRight size={16} />
                  </button>
                </div>
                <DropArt />
                {dragging && (
                  <div className="drop-overlay">
                    <ArrowDownToLine size={34} />
                    <strong>Drop to tuck them away</strong>
                  </div>
                )}
                <div className="drop-foot">
                  <ShieldCheck size={13} /> Processed on your Mac <span>·</span>{" "}
                  Originals kept when importing
                </div>
              </section>
              <div className="stats">
                <div>
                  <span className="stat-icon green">
                    <CheckCheck size={19} />
                  </span>
                  <div>
                    <strong>{sorted}</strong>
                    <span>Documents tucked away</span>
                  </div>
                </div>
                <div>
                  <span className="stat-icon amber">
                    <CircleHelp size={19} />
                  </span>
                  <div>
                    <strong>{reviewed}</strong>
                    <span>Need a little guidance</span>
                  </div>
                </div>
                <div>
                  <span className="stat-icon lavender">
                    <Folder size={19} />
                  </span>
                  <div>
                    <strong>{state.preferences.rules.length}</strong>
                    <span>Places to call home</span>
                  </div>
                </div>
              </div>
              <section className="watch-card">
                <div className="section-title">
                  <h3>
                    <FolderOpen size={18} /> Your watched folder
                  </h3>
                  <span
                    className={`watch-badge ${state.watching ? "enabled" : ""}`}
                  >
                    <span className="dot" />
                    {state.watching
                      ? "Watching"
                      : state.preferences.root
                        ? "Paused"
                        : "Not connected"}
                  </span>
                </div>
                <div className="watch-body">
                  <span className="folder-tile">
                    <Folder size={25} />
                  </span>
                  <div className="watch-path">
                    <strong>
                      {state.preferences.root
                        ? nameOf(state.preferences.root)
                        : "Give your documents a home"}
                    </strong>
                    <span>
                      {state.preferences.root
                        ? shortPath(state.preferences.root)
                        : "Pick a folder. Tuck can keep an eye on new documents."}
                    </span>
                  </div>
                  <button
                    className="secondary compact"
                    onClick={chooseFolder}
                    disabled={disabled}
                  >
                    {state.preferences.root ? "Change" : "Choose folder"}
                  </button>
                </div>
                <div className="watch-footer">
                  <span>
                    {state.processing ? (
                      <>
                        <LoaderCircle className="spin" size={14} /> Sorting{" "}
                        {nameOf(state.processing)}
                      </>
                    ) : (
                      <>
                        <span
                          className={`dot ${state.watching ? "live" : ""}`}
                        />
                        {state.pending
                          ? `${state.pending} documents waiting`
                          : state.watching
                            ? "New documents will be sorted automatically"
                            : "Start watching when you’re ready"}
                      </>
                    )}
                  </span>
                  <div>
                    {state.preferences.root && (
                      <button
                        className="text-button muted"
                        onClick={() => setCleanup(true)}
                        disabled={disabled}
                      >
                        Sort existing
                      </button>
                    )}
                    <button
                      className="text-button"
                      disabled={disabled || !state.preferences.root}
                      onClick={() =>
                        run(() =>
                          invoke("set_watching", { enabled: !state.watching }),
                        )
                      }
                    >
                      {state.watching ? (
                        <Pause size={14} />
                      ) : (
                        <Play size={14} />
                      )}{" "}
                      {state.watching ? "Pause" : "Start watching"}
                    </button>
                  </div>
                </div>
              </section>
              <section className="recent">
                <div className="section-title">
                  <h3>Recently tucked away</h3>
                  <button
                    className="text-button muted"
                    onClick={() => {
                      setFilter("all");
                      setPage("Activity");
                    }}
                  >
                    View activity <ArrowRight size={14} />
                  </button>
                </div>
                {state.activity.length ? (
                  <div className="activity-list">
                    {activityRows(state.activity.slice(0, 4))}
                  </div>
                ) : (
                  <div className="empty-activity">
                    <span>
                      <History size={19} />
                    </span>
                    <div>
                      <strong>A fresh start feels good.</strong>
                      <p>
                        Your organized documents will appear here, with an undo
                        for every move.
                      </p>
                    </div>
                  </div>
                )}
              </section>
              <div className="bottom-note">
                <Sparkles size={14} />
                <span>
                  A system that feels like you.{" "}
                  <button onClick={() => setPage("Folder rules")}>
                    Make the rules your own <ArrowRight size={12} />
                  </button>
                </span>
              </div>
            </>
          )}
          {page === "Activity" && (
            <>
              <div className="page-heading">
                <div>
                  <div className="eyebrow">EVERY LITTLE MOVE</div>
                  <h1>A place for the details.</h1>
                  <p>
                    See where things went. Undo a move whenever you need to.
                  </p>
                </div>
              </div>
              <div
                className="filter-tabs"
                role="group"
                aria-label="Filter activity"
              >
                {[
                  ["all", "All activity"],
                  ["sorted", "Tucked away"],
                  ["review", "Needs review"],
                  ["failed", "Original kept"],
                  ["undone", "Undone"],
                ].map(([value, label]) => (
                  <button
                    key={value}
                    className={filter === value ? "selected" : ""}
                    onClick={() => setFilter(value)}
                  >
                    {label}
                  </button>
                ))}
              </div>
              <section className="panel">
                {visibleActivity.length ? (
                  activityRows(visibleActivity)
                ) : (
                  <div className="empty-large">
                    <History size={32} />
                    <h3>No activity here yet</h3>
                    <p>
                      Once Tuck sorts a document, you’ll find its journey here.
                    </p>
                  </div>
                )}
              </section>
            </>
          )}
          {page === "Folder rules" && (
            <>
              <div className="page-heading">
                <div>
                  <div className="eyebrow">YOUR FILES, YOUR WAY</div>
                  <h1>A home for everything.</h1>
                  <p>Choose your folders and the words that belong in them.</p>
                </div>
                <button
                  className="primary"
                  onClick={save}
                  disabled={disabled || !dirty}
                >
                  <Check size={16} /> Save rules
                </button>
              </div>
              <div className="rule-intro">
                <Sparkles size={19} />
                <p>
                  Rules run from top to bottom. Put specific folders first, like{" "}
                  <strong>Migration/AGENCY</strong>. Local AI handles the
                  documents your keywords don’t catch.
                </p>
              </div>
              <div className="rule-table">
                <div className="rule-table-head">
                  <span>DESTINATION FOLDER</span>
                  <span>MATCH ANY OF THESE WORDS</span>
                  <span />
                </div>
                {draft.rules.map((rule, index) => (
                  <div className="rule-row" key={index}>
                    <div className="rule-folder">
                      <Folder size={17} />
                      <input
                        aria-label={`Folder ${index + 1}`}
                        value={rule.folder}
                        onChange={(event) =>
                          updateRule(index, "folder", event.target.value)
                        }
                        placeholder="e.g. Migration/AGENCY"
                      />
                    </div>
                    <input
                      aria-label={`Keywords ${index + 1}`}
                      value={rule.keywords}
                      onChange={(event) =>
                        updateRule(index, "keywords", event.target.value)
                      }
                      placeholder="Separate keywords with commas"
                    />
                    <div className="rule-actions">
                      <button
                        className="icon-button"
                        disabled={index === 0}
                        aria-label={`Move rule ${index + 1} up`}
                        onClick={() => moveRule(index, -1)}
                      >
                        <ArrowUp size={13} />
                      </button>
                      <button
                        className="icon-button"
                        disabled={index === draft.rules.length - 1}
                        aria-label={`Move rule ${index + 1} down`}
                        onClick={() => moveRule(index, 1)}
                      >
                        <ArrowDown size={13} />
                      </button>
                      <button
                        className="icon-button"
                        aria-label={`Remove rule ${index + 1}`}
                        onClick={() =>
                          setDraft({
                            ...draft,
                            rules: draft.rules.filter((_, i) => i !== index),
                          })
                        }
                      >
                        <Trash2 size={14} />
                      </button>
                    </div>
                  </div>
                ))}
                <button
                  className="add-rule"
                  onClick={() =>
                    setDraft({
                      ...draft,
                      rules: [...draft.rules, { folder: "", keywords: "" }],
                    })
                  }
                  disabled={draft.rules.length >= 40}
                >
                  <Plus size={16} /> Add a folder rule
                </button>
              </div>
              <div className="review-note">
                <CircleHelp size={19} />
                <div>
                  <strong>There’s always room for “not sure.”</strong>
                  <p>
                    Uncertain documents go to <b>Needs Review</b> with their
                    original filename. Duplicate copies are kept.
                  </p>
                </div>
              </div>
            </>
          )}
          {page === "Preferences" && (
            <>
              <div className="page-heading">
                <div>
                  <div className="eyebrow">MAKE YOURSELF AT HOME</div>
                  <h1>The way you like things.</h1>
                  <p>A few small preferences. A much tidier folder.</p>
                </div>
                <button
                  className="primary"
                  onClick={save}
                  disabled={disabled || !dirty}
                >
                  <Check size={16} /> Save preferences
                </button>
              </div>
              <section className="settings-panel">
                <h3>
                  <Folder size={18} /> Organizing
                </h3>
                <div className="setting-row">
                  <div>
                    <strong>Include subfolders</strong>
                    <p>Watch documents inside your chosen folder, too.</p>
                  </div>
                  <Switch
                    checked={draft.recursive}
                    label="Include subfolders"
                    onChange={() =>
                      setDraft({ ...draft, recursive: !draft.recursive })
                    }
                  />
                </div>
                <div className="setting-row">
                  <div>
                    <strong>Give documents clear names</strong>
                    <p>Use local AI to suggest readable English filenames.</p>
                  </div>
                  <Switch
                    checked={draft.rename}
                    label="Rename documents"
                    onChange={() =>
                      setDraft({ ...draft, rename: !draft.rename })
                    }
                  />
                </div>
                <div className="setting-row">
                  <div>
                    <strong>Keep folders simple</strong>
                    <p>Maximum number of folder levels beneath your root.</p>
                  </div>
                  <select
                    aria-label="Maximum folder depth"
                    value={draft.maxDepth}
                    onChange={(event) =>
                      setDraft({
                        ...draft,
                        maxDepth: Number(event.target.value),
                      })
                    }
                  >
                    <option value={1}>One level</option>
                    <option value={2}>Two levels</option>
                  </select>
                </div>
              </section>
              <section className="settings-panel">
                <h3>
                  <Sparkles size={18} /> Local intelligence
                </h3>
                <div className="setting-row">
                  <div>
                    <strong>Use a local model</strong>
                    <p>
                      Connect to Ollama on this Mac. Keyword rules always work.
                    </p>
                  </div>
                  <Switch
                    checked={draft.useAi}
                    label="Use local AI"
                    onChange={() => setDraft({ ...draft, useAi: !draft.useAi })}
                  />
                </div>
                <div className="model-settings">
                  <label htmlFor="model">Ollama model</label>
                  <div className="inline-field">
                    <input
                      id="model"
                      value={draft.model}
                      onChange={(event) =>
                        setDraft({ ...draft, model: event.target.value })
                      }
                    />
                    <button
                      className="secondary"
                      onClick={checkModel}
                      disabled={disabled}
                    >
                      Check connection
                    </button>
                  </div>
                  <p className="field-help">
                    {modelStatus} · Install with{" "}
                    <code>ollama pull {draft.model || "qwen3:4b"}</code>
                  </p>
                  <label htmlFor="instructions">
                    Tell Tuck how you like things organized
                  </label>
                  <textarea
                    id="instructions"
                    rows={4}
                    value={draft.instructions}
                    onChange={(event) =>
                      setDraft({ ...draft, instructions: event.target.value })
                    }
                  />
                  <p className="field-help">
                    These instructions guide the local model. Folder names and
                    keyword rules take priority.
                  </p>
                </div>
              </section>
              <div className="privacy-note">
                <ShieldCheck size={19} />
                <p>
                  Document text recognition runs on your Mac. AI requests go
                  only to Ollama at <code>127.0.0.1:11434</code>. No cloud
                  provider is used in this version.
                </p>
              </div>
            </>
          )}
        </div>
      </main>
      {cleanup && (
        <div className="modal-backdrop">
          <section
            className="modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="cleanup-title"
          >
            <span className="modal-icon">
              <FolderOpen size={25} />
            </span>
            <h2 id="cleanup-title">Tidy the documents already here?</h2>
            <p>
              Tuck will organize supported documents in{" "}
              <strong>{shortPath(state.preferences.root)}</strong>
              {state.preferences.recursive ? " and its subfolders" : ""}, using
              your saved rules. It will also start watching for new documents.
            </p>
            <p>
              Every move appears in Activity and can be undone. Files already in
              the right place stay there.
            </p>
            <div className="modal-actions">
              <button className="secondary" onClick={() => setCleanup(false)}>
                Keep for later
              </button>
              <button
                className="primary"
                disabled={disabled}
                onClick={() => {
                  setCleanup(false);
                  void run(
                    () => invoke("organize_existing"),
                    "Existing documents queued for sorting.",
                  );
                }}
              >
                Sort existing documents
              </button>
            </div>
          </section>
        </div>
      )}
      {selected && (
        <div className="modal-backdrop">
          <section
            className="modal detail-modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="detail-title"
          >
            <button
              className="close-modal icon-button"
              aria-label="Close details"
              onClick={() => setSelected(null)}
            >
              <X size={19} />
            </button>
            <span className="modal-icon">
              <FileText size={25} />
            </span>
            <h2 id="detail-title">
              {nameOf(selected.destination || selected.original)}
            </h2>
            <p>{selected.reason}</p>
            <dl>
              <dt>Originally</dt>
              <dd>{shortPath(selected.original)}</dd>
              <dt>{selected.copied ? "Copied to" : "Moved to"}</dt>
              <dd>
                {selected.destination
                  ? shortPath(selected.destination)
                  : "Original location kept"}
              </dd>
            </dl>
            {selected.duplicate && (
              <p className="duplicate-note">
                A matching copy was found and kept. Application bundles can
                contain intentional copies.
              </p>
            )}
            <div className="modal-actions">
              <button
                className="secondary"
                disabled={
                  disabled ||
                  !selected.destination ||
                  selected.status === "undone"
                }
                onClick={() => run(() => revealItemInDir(selected.destination))}
              >
                <FolderOpen size={16} /> Show in Finder
              </button>
              <button
                className="primary"
                disabled={
                  disabled || !["sorted", "review"].includes(selected.status)
                }
                onClick={() => {
                  const id = selected.id;
                  setSelected(null);
                  void run(
                    () => invoke("undo_action", { id }),
                    "Done. The document is back where it was.",
                  );
                }}
              >
                <Undo2 size={16} /> Undo {selected.copied ? "copy" : "move"}
              </button>
            </div>
          </section>
        </div>
      )}
      {busy && (
        <div className="busy-indicator" role="status">
          <LoaderCircle size={16} className="spin" /> Just a moment…
        </div>
      )}
    </div>
  );
}
export default App;
