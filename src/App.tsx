import { useCallback, useEffect, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import {
  ArrowRight,
  Check,
  CircleHelp,
  FileText,
  Folder,
  FolderOpen,
  Inbox,
  LoaderCircle,
  Plus,
  Settings2,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import {
  initial,
  type Preferences,
  type CategoryProposal,
  type RuleIssue,
  type ReviewItem,
  type Snapshot,
} from "./types";
import Settings from "./Settings";
import FolderTree from "./FolderTree";
import DocumentDecision from "./DocumentDecision";
import Welcome, { welcomeKey } from "./Welcome";
import "./App.css";
import "./MacUI.css";
import { ExampleDraft, RuleCheckResults, exampleText } from "./RuleLearning";
const desktop = isTauri();
const nameOf = (path: string) =>
  path.split("/").filter(Boolean).pop() || "Documents";
type Page = "Organize" | "Folders" | "Settings";
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
function App() {
  const [state, setState] = useState<Snapshot>(initial);
  const [draft, setDraft] = useState<Preferences>(initial.preferences);
  const draftRef = useRef(draft);
  const previous = useRef(initial.preferences);
  const [page, setPage] = useState<Page>("Organize");
  useEffect(() => {
    document.querySelector(".content-scroll")?.scrollTo({ top: 0 });
  }, [page]);
  useEffect(() => {
    const navigate = (event: KeyboardEvent) => {
      if (!event.metaKey || event.altKey || event.ctrlKey) return;
      const target =
        event.key === ","
          ? "Settings"
          : event.key === "1"
            ? "Organize"
            : event.key === "2"
              ? "Folders"
              : null;
      if (target) {
        event.preventDefault();
        setPage(target);
      }
    };
    window.addEventListener("keydown", navigate);
    return () => window.removeEventListener("keydown", navigate);
  }, []);
  const [busy, setBusy] = useState(false);
  const busyRef = useRef(false);
  const [loaded, setLoaded] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const [dragging, setDragging] = useState(false);
  const [documentView, setDocumentView] = useState<"attention" | "activity">(
    "attention",
  );
  const [selected, setSelected] = useState<string[]>([]);
  const [edits, setEdits] = useState<
    Record<string, { folder: string; title: string }>
  >({});
  const [welcome, setWelcome] = useState(
    () => localStorage.getItem(welcomeKey) !== "seen",
  );
  const [categoryProposal, setCategoryProposal] =
    useState<CategoryProposal | null>(null);
  const [suggestingCategories, setSuggestingCategories] = useState(false);
  const [watchFolder, setWatchFolder] = useState(false);
  const [corrected, setCorrected] = useState<string[]>([]);
  const [exampleDraft, setExampleDraft] = useState<{
    id: string;
    folder: string;
    title: string;
    baseline: string;
  } | null>(null);
  const [ruleCheck, setRuleCheck] = useState<{
    issues: RuleIssue[];
    baseline: string;
    key: number;
  } | null>(null);
  const [checkingRules, setCheckingRules] = useState(false);
  const [apiKey, setApiKey] = useState("");
  const dirty =
    !!apiKey || JSON.stringify(draft) !== JSON.stringify(state.preferences);
  const disabled = busy || !loaded || !desktop;
  const setPreferences = (p: Preferences) => {
    draftRef.current = p;
    setDraft(p);
  };
  const refresh = useCallback(async () => {
    const next = await invoke<Snapshot>("get_state");
    next.preferences = { ...initial.preferences, ...next.preferences };
    if (JSON.stringify(draftRef.current) === JSON.stringify(previous.current)) {
      draftRef.current = next.preferences;
      setDraft(next.preferences);
    }
    previous.current = next.preferences;
    setState(next);
    setLoaded(true);
    return next;
  }, []);
  const run = useCallback(
    async (action: () => Promise<void>) => {
      if (busyRef.current) return;
      busyRef.current = true;
      setBusy(true);
      setError("");
      setMessage("");
      try {
        await action();
      } catch (e) {
        setError(String(e));
      } finally {
        try {
          await refresh();
        } catch (e) {
          setError(String(e));
        }
        busyRef.current = false;
        setBusy(false);
      }
    },
    [refresh],
  );
  useEffect(() => {
    if (!desktop) return;
    let disposed = false;
    const poll = async () => {
      if (!disposed) {
        try {
          await refresh();
        } catch (e) {
          if (!disposed) setError(String(e));
        }
      }
    };
    void poll();
    const timer = window.setInterval(() => void poll(), 1000);
    return () => {
      disposed = true;
      clearInterval(timer);
    };
  }, [refresh]);
  const importFiles = useCallback(
    (paths: string[]) =>
      run(async () => {
        const count = await invoke<number>("import_documents", { paths });
        setPage("Organize");
        setMessage(`${count} document${count === 1 ? "" : "s"} added to Tuck.`);
      }),
    [run],
  );
  useEffect(() => {
    if (!desktop) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        setDragging(
          event.payload.type === "over" || event.payload.type === "enter",
        );
        if (event.payload.type === "drop") {
          if (busyRef.current)
            setError(
              "An action is finishing. Drop the files again in a moment.",
            );
          else if (
            JSON.stringify(draftRef.current) !==
            JSON.stringify(previous.current)
          )
            setError(
              "Save or reload your preferences before adding documents.",
            );
          else void importFiles(event.payload.paths);
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
  }, [importFiles]);
  useEffect(() => {
    if (!message) return;
    const timeout = window.setTimeout(() => setMessage(""), 6000);
    return () => window.clearTimeout(timeout);
  }, [message]);
  const chooseFolder = () =>
    run(async () => {
      const root = await open({
        directory: true,
        multiple: false,
        title: "Choose a destination folder",
      });
      if (typeof root === "string") {
        await invoke("choose_folder", { root });
        setCategoryProposal(null);
        setRuleCheck(null);
        setCorrected([]);
        setExampleDraft(null);
        setWatchFolder(false);
        setPage("Organize");
        setMessage("");
        setDocumentView("attention");
        await refresh();
        await invoke("map_folder");
        setMessage("Your folder is ready.");
      }
    });
  const mapFolder = () =>
    run(async () => {
      await invoke("map_folder");
      setMessage("Folder refreshed.");
    });
  const reviewExisting = () =>
    run(async () => {
      if (dirty)
        throw new Error(
          "Save or reload your preferences before starting Tuck.",
        );
      if (!state.collection) await invoke("map_folder");
      if (
        JSON.stringify(draftRef.current) !== JSON.stringify(previous.current)
      ) {
        throw new Error("Save or reload your changes before starting Tuck.");
      }
      await invoke("start_tuck", { watch: watchFolder });
      setDocumentView("attention");
      setMessage("");
    });
  const useExisting = () =>
    run(async () => {
      if (!state.collection || dirty) return;
      await invoke("save_preferences", {
        preferences: {
          ...state.preferences,
          rules: state.collection.existingRules,
        },
      });
      const next = await refresh();
      setPreferences(next.preferences);
      setMessage("Existing folders selected.");
    });
  const browse = () =>
    run(async () => {
      const paths = await open({
        multiple: true,
        directory: false,
        title: "Choose documents to organize",
        filters: [
          {
            name: "Organize",
            extensions: [
              "pdf",
              "png",
              "jpg",
              "jpeg",
              "heic",
              "tiff",
              "tif",
              "txt",
              "md",
            ],
          },
        ],
      });
      if (paths) {
        const count = await invoke<number>("import_documents", {
          paths: Array.isArray(paths) ? paths : [paths],
        });
        setMessage(`${count} documents added to Tuck.`);
        setPage("Organize");
      }
    });
  const suggestRule = (index: number) =>
    run(async () => {
      const baseline = JSON.stringify(draftRef.current);
      const before = draftRef.current.rules[index];
      if (!before || before.instruction.trim()) return;
      const instruction = await invoke<string>("suggest_folder_rule", {
        folder: before.folder,
        preferences: draftRef.current,
      });
      const current = draftRef.current;
      if (JSON.stringify(current) !== baseline)
        throw new Error(
          "This folder changed while Tuck was writing. Try again when you're ready.",
        );
      setPreferences({
        ...current,
        rules: current.rules.map((r, i) =>
          i === index ? { ...r, instruction } : r,
        ),
      });
      setMessage("Description drafted. Make it yours, then Save rules.");
    });
  const checkRules = () =>
    run(async () => {
      const baseline = JSON.stringify(draftRef.current);
      setCheckingRules(true);
      try {
        const issues = await invoke<RuleIssue[]>("check_folder_rules", {
          preferences: draftRef.current,
        });
        if (JSON.stringify(draftRef.current) !== baseline)
          throw new Error(
            "Rules changed while checking. Run Check rules again.",
          );
        setRuleCheck({ issues, baseline, key: Date.now() });
      } finally {
        setCheckingRules(false);
      }
    });
  const addClarification = (index: number, text: string) => {
    if (
      !ruleCheck ||
      JSON.stringify(draftRef.current) !== ruleCheck.baseline ||
      draft.instructionsRevision !== state.preferences.instructionsRevision ||
      draft.settingsRevision !== state.preferences.settingsRevision
    ) {
      setError("Rules changed. Check them again first.");
      return;
    }
    const issue = ruleCheck.issues[index];
    setPreferences({
      ...draftRef.current,
      rules: draftRef.current.rules.map((r) =>
        r.folder === issue.folder
          ? { ...r, instruction: `${r.instruction.trim()}\n\n${text.trim()}` }
          : r,
      ),
    });
    setMessage(
      "Clarification added to your draft. Edit it if needed, then Save rules.",
    );
  };
  const addExample = (belongs: string, excludes: string) => {
    if (!exampleDraft) return;
    const item = state.items.find((i) => i.id === exampleDraft.id);
    if (
      !item ||
      item.folder !== exampleDraft.folder ||
      item.title !== exampleDraft.title ||
      edited(item)
    ) {
      setError(
        "The document preview changed. Cancel this example and open it again.",
      );
      return;
    }
    if (JSON.stringify(state.preferences) !== exampleDraft.baseline || dirty) {
      setError("Rules changed. Cancel this example and open it again.");
      return;
    }
    const rule = state.preferences.rules.find(
      (r) => r.folder === exampleDraft.folder,
    );
    if (!rule || !enabled(rule.folder)) {
      setError("Choose an enabled folder first.");
      return;
    }
    try {
      const instruction = exampleText(rule.instruction, belongs, excludes);
      setPreferences({
        ...state.preferences,
        rules: state.preferences.rules.map((r) =>
          r.folder === rule.folder ? { ...r, instruction } : r,
        ),
      });
      setExampleDraft(null);
      setSelected([]);
      setPage("Folders");
      setMessage(
        "Example added to the rule draft. Save rules to remember it. Existing suggestions will need Analyze again.",
      );
    } catch (e) {
      setError(String(e));
    }
  };
  const save = () =>
    run(async () => {
      await invoke("save_preferences", {
        preferences: { ...draft, rename: true },
        apiKey: apiKey || null,
      });
      setApiKey("");
      const next = await refresh();
      setPreferences(next.preferences);
      setSelected([]);
      setMessage(
        state.items.some((i) => i.status === "review")
          ? "Rules saved. Tuck stopped. Clear the session and map again to restart, or analyze fallback suggestions again."
          : "Rules and preferences saved.",
      );
    });
  const reload = () =>
    run(async () => {
      setApiKey("");
      const next = await refresh();
      setPreferences(next.preferences);
    });
  const suggestCategories = () =>
    run(async () => {
      if (dirty)
        throw new Error(
          "Save or reload your changes before asking for categories.",
        );
      setPage("Folders");
      setSuggestingCategories(true);
      try {
        if (!state.collection) await invoke("map_folder");
        const proposal = await invoke<CategoryProposal>("suggest_categories");
        setCategoryProposal(proposal);
      } finally {
        setSuggestingCategories(false);
      }
    });
  const createCategories = () =>
    run(async () => {
      if (!categoryProposal) return;
      if (dirty)
        throw new Error(
          "Save or reload your changes before creating categories.",
        );
      await invoke("save_preferences", {
        preferences: {
          ...categoryProposal.preferences,
          rules: categoryProposal.rules,
        },
      });
      const next = await refresh();
      setPreferences(next.preferences);
      setCategoryProposal(null);
      setPage("Organize");
      setMessage("Categories saved. Ready to start.");
    });
  const categoryPreview = categoryProposal && (
    <section className="category-preview" aria-label="Suggested categories">
      <h2>Your suggested categories</h2>
      <p>Adjust these categories, then save them to get started.</p>
      <div
        className={`proposal-coverage${categoryProposal.skipped ? " incomplete" : ""}`}
        role="status"
      >
        <strong>
          Based on {categoryProposal.sampled} of{" "}
          {categoryProposal.sampled + categoryProposal.skipped} documents
        </strong>
        {categoryProposal.skipped > 0 && (
          <p>
            {categoryProposal.skipped} files still need review. These
            suggestions do not cover the whole folder.
          </p>
        )}
        {categoryProposal.limited && (
          <p>
            Suggestions were prepared in smaller batches. Check for overlapping
            folders.
          </p>
        )}
      </div>
      {!!categoryProposal.reviewFiles?.length && (
        <details className="proposal-review-files">
          <summary>
            Files needing review ({categoryProposal.reviewFiles.length})
          </summary>
          <ul>
            {categoryProposal.reviewFiles.map((file) => (
              <li key={file.path}>
                <strong>{file.path.split("/").pop()}</strong>
                <span>{file.reason}</span>
              </li>
            ))}
          </ul>
        </details>
      )}
      <FolderTree
        key={`proposal:${categoryProposal.preferences.root}`}
        root={categoryProposal.preferences.root}
        rules={categoryProposal.rules}
        rootRule={categoryProposal.preferences.instructions}
        suggested
        disabled={busy}
        onRules={(rules) => setCategoryProposal({ ...categoryProposal, rules })}
        onRootRule={(instructions) =>
          setCategoryProposal({
            ...categoryProposal,
            preferences: { ...categoryProposal.preferences, instructions },
          })
        }
      />
      <div className="category-actions">
        <button
          className="text-button"
          disabled={busy}
          onClick={() => setCategoryProposal(null)}
        >
          Discard suggestions
        </button>
        <button
          className="primary"
          disabled={disabled || dirty || !categoryProposal.rules.length}
          onClick={createCategories}
        >
          Use these categories
        </button>
      </div>
    </section>
  );
  const enabled = (folder: string) =>
    state.preferences.rules.some((r) => r.folder === folder && r.enabled) &&
    !state.preferences.rules.some(
      (r) => !r.enabled && folder.startsWith(`${r.folder}/`),
    );
  const edited = (item: ReviewItem) =>
    !!edits[item.id] &&
    (edits[item.id].folder !== item.folder ||
      edits[item.id].title !== item.title);
  const ready = (item: ReviewItem) =>
    item.status === "review" &&
    !!item.destination &&
    !edited(item) &&
    enabled(item.folder);
  const chosen = state.items.filter(
    (item) => selected.includes(item.id) && ready(item),
  );
  const organize = (items: ReviewItem[]) =>
    run(async () => {
      await invoke("organize_documents", {
        approvals: items.map(({ id, destination }) => ({ id, destination })),
      });
      setSelected((ids) => ids.filter((id) => !items.some((i) => i.id === id)));
      setMessage(
        `${items.length} document${items.length === 1 ? "" : "s"} organized.`,
      );
    });
  const edit = (
    item: ReviewItem,
    patch: Partial<{ folder: string; title: string }>,
  ) => {
    setEdits((e) => ({
      ...e,
      [item.id]: {
        ...(e[item.id] || { folder: item.folder, title: item.title }),
        ...patch,
      },
    }));
    setSelected((ids) => ids.filter((id) => id !== item.id));
  };
  const resolveDecision = (item: ReviewItem, remember = false) =>
    run(async () => {
      const baseline = JSON.stringify(state.preferences);
      const values = edits[item.id] || {
        folder: item.folder,
        title: item.title,
      };
      const changed = edited(item);
      if (dirty || !enabled(values.folder) || !values.title.trim())
        throw new Error(
          "Choose an enabled category and save any rule changes first.",
        );
      const current = await invoke<Snapshot>("get_state");
      if (JSON.stringify(current.preferences) !== baseline)
        throw new Error(
          "Folder settings changed. Check the document and try again.",
        );
      if (changed) {
        await invoke("update_review", { id: item.id, ...values });
      }
      const fresh = await invoke<Snapshot>("get_state");
      const resolved = fresh.items.find(
        (candidate) => candidate.id === item.id,
      );
      if (
        JSON.stringify(fresh.preferences) !== baseline ||
        JSON.stringify(draftRef.current) !== baseline ||
        !resolved ||
        resolved.status !== "review" ||
        !resolved.destination ||
        resolved.folder !== values.folder ||
        resolved.title !== values.title.trim() ||
        (!changed && resolved.destination !== item.destination)
      )
        throw new Error(
          "The document or folder settings changed. Check it and try again.",
        );
      if (!remember) {
        await invoke("organize_documents", {
          approvals: [{ id: resolved.id, destination: resolved.destination }],
        });
        setMessage("Document filed.");
      }
      if (changed)
        setCorrected((ids) =>
          ids.includes(item.id) ? ids : [...ids, item.id],
        );
      setEdits((e) => {
        const next = { ...e };
        delete next[item.id];
        return next;
      });
      setSelected((ids) => ids.filter((id) => id !== item.id));
      setExampleDraft(
        remember
          ? {
              id: resolved.id,
              folder: resolved.folder,
              title: resolved.title,
              baseline,
            }
          : null,
      );
    });
  const retry = (item: ReviewItem) =>
    run(async () => {
      await invoke("retry_document", { id: item.id });
      setCorrected((ids) => ids.filter((id) => id !== item.id));
      setExampleDraft(null);
      setEdits((e) => {
        const next = { ...e };
        delete next[item.id];
        return next;
      });
      setSelected((ids) => ids.filter((id) => id !== item.id));
    });
  const closeWelcome = () => {
    localStorage.setItem(welcomeKey, "seen");
    setWelcome(false);
  };
  const attentionItems = state.items.filter((item) =>
    ["review", "failed"].includes(item.status),
  );
  const activityItems = state.items.filter(
    (item) => !["review", "failed"].includes(item.status),
  );
  const organizedCount = state.items.filter(
    (item) => item.status === "organized",
  ).length;
  const pendingCount = state.items.filter((item) =>
    ["queued", "reading"].includes(item.status),
  ).length;
  const visibleItems =
    documentView === "attention" ? attentionItems : activityItems;
  const activeRules = state.preferences.rules.filter((rule) =>
    enabled(rule.folder),
  );
  const setupLocked =
    disabled || dirty || !!state.processing || !!state.items.length;
  const sameExisting =
    JSON.stringify(state.collection?.existingRules) ===
    JSON.stringify(state.preferences.rules);
  const folderTree = (
    <>
      <FolderTree
        key={state.preferences.root}
        root={state.preferences.root}
        rules={draft.rules}
        rootRule={draft.instructions}
        onSuggest={
          state.preferences.useAi && state.preferences.root
            ? suggestRule
            : undefined
        }
        disabled={disabled || !!state.processing}
        onRules={(rules) => setPreferences({ ...draftRef.current, rules })}
        onRootRule={(instructions) =>
          setPreferences({ ...draftRef.current, instructions })
        }
      />
    </>
  );
  const clearSession = () =>
    run(async () => {
      await invoke("clear_batch");
      setCorrected([]);
      setExampleDraft(null);
      setSelected([]);
      setEdits({});
      setDocumentView("attention");
      setCategoryProposal(null);
      setMessage("Session cleared. Your files stay in place.");
    });
  return (
    <div className="app-shell">
      <header className="app-header topbar">
        <div className="brand">
          <Mark />
          Tuck
        </div>
        <nav aria-label="Main navigation">
          {(
            [
              ["Organize", Inbox],
              ["Folders", Folder],
              ["Settings", Settings2],
            ] as const
          ).map(([name, Icon]) => (
            <button
              key={name}
              className={`nav-item ${page === name ? "active" : ""}`}
              aria-current={page === name ? "page" : undefined}
              onClick={() => setPage(name)}
            >
              <Icon size={16} />
              {name}
              {name === "Organize" && attentionItems.length > 0 && (
                <b>{attentionItems.length}</b>
              )}
            </button>
          ))}
        </nav>
        <div className="header-tools">
          <span
            className="privacy-pill"
            title={
              state.preferences.aiModel === "apple"
                ? "Apple Intelligence processes text on this Mac."
                : "Your model connection receives document text."
            }
          >
            <ShieldCheck size={13} />
            {!state.preferences.useAi
              ? "Manual"
              : state.preferences.aiModel === "apple"
                ? "On-device AI"
                : "Connected model"}
          </span>
          <button
            className="icon-button"
            aria-label="Getting started"
            onClick={() => setWelcome(true)}
          >
            <CircleHelp size={18} />
          </button>
        </div>
      </header>
      <main>
        <div className="content-scroll">
          <div className="main-content">
            {!desktop && (
              <div className="notice">
                Open the Tuck desktop app to organize files. This browser view
                is a preview.
              </div>
            )}
            {(error || state.error) && (
              <div role="alert" className="mvp-alert">
                {error || state.error}
              </div>
            )}
            {message && (
              <div role="status" className="mvp-message">
                {message}
              </div>
            )}
            {dirty && page === "Organize" && (
              <div className="mvp-message notice-with-actions">
                <span>You have unsaved preferences.</span>
                <div className="notice-actions">
                  <button
                    className="text-button"
                    disabled={disabled}
                    onClick={save}
                  >
                    Save
                  </button>
                  <button
                    className="text-button"
                    disabled={disabled}
                    onClick={reload}
                  >
                    Reload from disk
                  </button>
                </div>
              </div>
            )}
            {page === "Organize" && (
              <>
                <div className="page-heading overview-heading">
                  <div>
                    <div className="page-kicker">A PLACE FOR EVERYTHING</div>
                    <h1>Organize</h1>
                    <p>
                      {state.running
                        ? "Tuck is filing clear matches. Review the rest here."
                        : "Turn a folder of documents into a place you can find things."}
                    </p>
                  </div>
                  {state.preferences.root && (
                    <span
                      className={`workspace-status ${state.running ? "is-running" : ""}`}
                    >
                      <i />
                      {state.running ? "Organizing" : "Ready when you are"}
                    </span>
                  )}
                </div>
                {!state.preferences.root ? (
                  <section
                    className="folder-welcome"
                    aria-label="Choose your folder"
                  >
                    <div className="welcome-folder-icon">
                      <FolderOpen size={36} strokeWidth={1.4} />
                    </div>
                    <h2>Start with a folder</h2>
                    <p>
                      Choose where your documents live. You’ll review the folder
                      structure before organizing.
                    </p>
                    <button
                      className="primary"
                      disabled={disabled}
                      onClick={chooseFolder}
                    >
                      <FolderOpen size={17} /> Choose folder
                    </button>
                  </section>
                ) : (
                  <section className="tuck-control" aria-label="Your folder">
                    <div className="folder-heading">
                      <span className="folder-icon">
                        <FolderOpen size={23} />
                      </span>
                      <div>
                        <h2>{nameOf(state.preferences.root)}</h2>
                        <p className="workspace-path">
                          {state.preferences.root}
                        </p>
                      </div>
                      <button
                        className="text-button"
                        disabled={setupLocked || state.running}
                        onClick={chooseFolder}
                      >
                        Change folder
                      </button>
                    </div>
                    {state.processing && (
                      <div className="reading-status" role="status">
                        <LoaderCircle size={15} className="spin" />
                        <span>
                          {state.processing.startsWith("Mapping")
                            ? "Getting to know your documents…"
                            : `Reading ${nameOf(state.processing)}…`}
                        </span>
                        {state.processing.startsWith("Mapping") && (
                          <button
                            className="text-button"
                            onClick={() => {
                              void invoke("stop_tuck").catch((e) =>
                                setError(String(e)),
                              );
                            }}
                          >
                            Cancel
                          </button>
                        )}
                      </div>
                    )}
                    {state.running || state.items.length > 0 ? (
                      <div className="session-summary">
                        <div>
                          <h2>
                            {!state.running
                              ? "Tuck is paused."
                              : state.processing || pendingCount
                                ? "Tucking things away…"
                                : state.watching
                                  ? "Watching for new documents."
                                  : attentionItems.length
                                    ? "Just a few to place."
                                    : "All caught up."}
                          </h2>
                          <p>
                            {organizedCount} organized
                            {attentionItems.length > 0
                              ? ` · ${attentionItems.length} need your help`
                              : " · Nothing needs your attention"}
                          </p>
                        </div>
                        {state.running ? (
                          <button
                            className="secondary compact"
                            disabled={disabled}
                            onClick={() =>
                              run(async () => {
                                await invoke("stop_tuck");
                              })
                            }
                          >
                            Stop Tuck
                          </button>
                        ) : (
                          <button
                            className="secondary compact"
                            disabled={disabled}
                            onClick={clearSession}
                          >
                            Start fresh
                          </button>
                        )}
                      </div>
                    ) : categoryProposal ? (
                      <div className="proposal-notice">
                        <div>
                          <h3>
                            {categoryProposal.skipped
                              ? "Partial folder suggestions"
                              : "Your folder suggestions are ready"}
                          </h3>
                          <p>
                            {categoryProposal.skipped
                              ? `Based on ${categoryProposal.sampled} of ${categoryProposal.sampled + categoryProposal.skipped} documents. ${categoryProposal.skipped} still need review.`
                              : "Review the draft before organizing."}
                          </p>
                        </div>
                        <button
                          className="secondary"
                          onClick={() => setPage("Folders")}
                        >
                          Review folders <ArrowRight size={14} />
                        </button>
                      </div>
                    ) : (
                      <>
                        <div className="setup-categories">
                          <div className="category-label">
                            <div>
                              <h3>
                                {activeRules.length
                                  ? `${activeRules.length} folders, ready to use`
                                  : "Set up your folders"}
                              </h3>
                              <p>
                                {activeRules.length
                                  ? "Keep this structure or make it your own."
                                  : "Create a structure before organizing documents."}
                              </p>
                            </div>
                            <button
                              className="text-button"
                              onClick={() => setPage("Folders")}
                            >
                              Edit folders <ArrowRight size={14} />
                            </button>
                          </div>
                          <div className="category-overview">
                            {activeRules.slice(0, 6).map((rule) => (
                              <span key={rule.folder}>
                                <Folder size={14} />
                                {rule.folder}
                              </span>
                            ))}
                            {activeRules.length > 6 && (
                              <span>+{activeRules.length - 6} more</span>
                            )}
                          </div>
                          {!activeRules.length && (
                            <button
                              className="secondary"
                              disabled={setupLocked || !state.preferences.useAi}
                              onClick={suggestCategories}
                            >
                              <Sparkles size={15} />
                              Suggest categories
                            </button>
                          )}
                          {!!state.collection?.existingRules.length &&
                            !sameExisting && (
                              <button
                                className="text-button"
                                disabled={setupLocked}
                                onClick={useExisting}
                              >
                                Use existing folders
                              </button>
                            )}
                        </div>
                        <div className="setup-footer">
                          <label className="watch-option">
                            <input
                              type="checkbox"
                              checked={watchFolder}
                              disabled={disabled || !!state.processing}
                              onChange={(e) => setWatchFolder(e.target.checked)}
                            />
                            Keep watching this folder
                          </label>
                          <button
                            className="primary"
                            disabled={
                              setupLocked ||
                              !activeRules.length ||
                              suggestingCategories
                            }
                            onClick={reviewExisting}
                          >
                            Start Tuck <ArrowRight size={16} />
                          </button>
                        </div>
                      </>
                    )}
                    {(state.running || state.items.length > 0) && (
                      <button
                        className="text-button category-shortcut"
                        onClick={() => setPage("Folders")}
                      >
                        Manage folders <ArrowRight size={14} />
                      </button>
                    )}
                    <details className="quiet-details folder-details">
                      <summary>Folder details</summary>
                      <p className="folder-location">
                        {state.preferences.root}
                      </p>
                      {state.collection && (
                        <p>
                          {state.collection.documents.length} documents ·{" "}
                          {state.collection.duplicates} duplicate copies ·{" "}
                          {state.collection.unreadable} unreadable
                        </p>
                      )}
                      <p>
                        Includes subfolders. Watching runs while Tuck is open.
                      </p>
                      {state.collection?.documents
                        .filter((d) => d.duplicateOf || d.error)
                        .map((d) => (
                          <p key={d.path}>
                            {nameOf(d.path)}:{" "}
                            {d.error ||
                              `Identical to ${nameOf(d.duplicateOf!)}`}
                          </p>
                        ))}
                      <button
                        className="text-button"
                        disabled={setupLocked || state.running}
                        onClick={mapFolder}
                      >
                        Refresh folder
                      </button>
                    </details>
                  </section>
                )}
                {state.running && (
                  <section
                    className="add-documents"
                    aria-label="Drop documents"
                  >
                    <Plus size={18} />
                    <p>Drop more documents here</p>
                    <button
                      className="text-button"
                      disabled={disabled || dirty}
                      onClick={browse}
                    >
                      Choose files
                    </button>
                  </section>
                )}
                {dragging && (
                  <div className="drop-overlay">
                    <Inbox size={35} />
                    Drop files to organize
                  </div>
                )}
                {state.items.length > 0 && (
                  <>
                    <div className="document-toolbar">
                      <div
                        className="document-tabs"
                        role="tablist"
                        aria-label="Organize"
                        onKeyDown={(event) => {
                          if (
                            ![
                              "ArrowLeft",
                              "ArrowRight",
                              "Home",
                              "End",
                            ].includes(event.key)
                          )
                            return;
                          event.preventDefault();
                          const next =
                            event.key === "Home"
                              ? "attention"
                              : event.key === "End"
                                ? "activity"
                                : documentView === "attention"
                                  ? "activity"
                                  : "attention";
                          setDocumentView(next);
                          event.currentTarget
                            .querySelectorAll<HTMLButtonElement>("button")
                            [next === "attention" ? 0 : 1]?.focus();
                        }}
                      >
                        <button
                          role="tab"
                          id="attention-tab"
                          aria-controls="document-list"
                          tabIndex={documentView === "attention" ? 0 : -1}
                          aria-selected={documentView === "attention"}
                          onClick={() => setDocumentView("attention")}
                        >
                          Needs attention <span>{attentionItems.length}</span>
                        </button>
                        <button
                          role="tab"
                          id="activity-tab"
                          aria-controls="document-list"
                          tabIndex={documentView === "activity" ? 0 : -1}
                          aria-selected={documentView === "activity"}
                          onClick={() => setDocumentView("activity")}
                        >
                          Activity <span>{activityItems.length}</span>
                        </button>
                      </div>
                      <div className="mvp-actions">
                        {documentView === "attention" && chosen.length > 0 && (
                          <button
                            className="secondary compact"
                            disabled={
                              disabled ||
                              dirty ||
                              !chosen.length ||
                              !!state.error
                            }
                            onClick={() => organize(chosen)}
                          >
                            Organize selected ({chosen.length})
                          </button>
                        )}
                        <button
                          className="text-button"
                          disabled={disabled || !!state.processing}
                          onClick={clearSession}
                        >
                          Clear batch
                        </button>
                      </div>
                    </div>
                    {documentView === "attention" && !attentionItems.length && (
                      <div className="all-clear">
                        <Check size={18} />
                        <span>Nothing needs your attention.</span>
                      </div>
                    )}
                    {documentView === "activity" && !activityItems.length && (
                      <p className="activity-empty">
                        Organized files will appear here.
                      </p>
                    )}
                  </>
                )}
                <div
                  className="review-items"
                  id="document-list"
                  role={state.items.length ? "tabpanel" : undefined}
                  aria-labelledby={
                    state.items.length ? `${documentView}-tab` : undefined
                  }
                >
                  {visibleItems.map((item) => {
                    const values = edits[item.id] || item;
                    if (!["review", "failed"].includes(item.status))
                      return (
                        <div className="activity-row" key={item.id}>
                          {item.status === "organized" ? (
                            <Check size={18} />
                          ) : (
                            <FileText size={18} />
                          )}
                          <div>
                            <button
                              className="document-preview"
                              aria-label={`Preview ${nameOf(item.destination || item.original)}`}
                              onClick={() =>
                                run(async () => {
                                  await invoke("preview_document", {
                                    id: item.id,
                                  });
                                })
                              }
                            >
                              {nameOf(item.destination || item.original)}
                            </button>
                            <p>
                              {item.status === "organized"
                                ? item.folder
                                : nameOf(item.original)}
                            </p>
                          </div>
                          <span className="status">{item.status}</span>
                          {item.status === "organized" && (
                            <button
                              className="text-button"
                              onClick={() =>
                                run(async () => {
                                  await revealItemInDir(item.destination);
                                })
                              }
                            >
                              Show in Finder
                            </button>
                          )}
                          {item.status === "skipped" && (
                            <button
                              className="text-button"
                              disabled={disabled || dirty}
                              onClick={() => retry(item)}
                            >
                              Analyze again
                            </button>
                          )}
                          {corrected.includes(item.id) &&
                            item.status === "organized" && (
                              <button
                                className="text-button"
                                disabled={
                                  disabled || dirty || !enabled(item.folder)
                                }
                                onClick={() => {
                                  setDocumentView("activity");
                                  setExampleDraft({
                                    id: item.id,
                                    folder: item.folder,
                                    title: item.title,
                                    baseline: JSON.stringify(state.preferences),
                                  });
                                }}
                              >
                                Remember this example
                              </button>
                            )}
                          {exampleDraft?.id === item.id && (
                            <ExampleDraft
                              key={item.id}
                              folder={exampleDraft.folder}
                              title={exampleDraft.title}
                              disabled={disabled}
                              onAdd={addExample}
                              onCancel={() => setExampleDraft(null)}
                            />
                          )}
                        </div>
                      );
                    return (
                      <DocumentDecision
                        key={`${state.preferences.root}:${item.id}`}
                        item={item}
                        onPreview={() =>
                          run(async () => {
                            await invoke("preview_document", { id: item.id });
                          })
                        }
                        rules={state.preferences.rules}
                        values={values}
                        disabled={disabled || dirty}
                        canFile={
                          !disabled &&
                          !dirty &&
                          !state.error &&
                          item.status === "review" &&
                          enabled(values.folder) &&
                          !!values.title.trim()
                        }
                        selectable={attentionItems.length > 1 && ready(item)}
                        selected={selected.includes(item.id) && ready(item)}
                        canRemember={
                          corrected.includes(item.id) || edited(item)
                        }
                        enabled={enabled}
                        onSelect={(checked) =>
                          setSelected((ids) =>
                            checked
                              ? [...ids, item.id]
                              : ids.filter((id) => id !== item.id),
                          )
                        }
                        onEdit={(patch) => edit(item, patch)}
                        onFile={() => resolveDecision(item)}
                        onSkip={() =>
                          run(async () => {
                            await invoke("skip_document", { id: item.id });
                          })
                        }
                        onRetry={() => retry(item)}
                        onRemember={() => resolveDecision(item, true)}
                      >
                        {exampleDraft?.id === item.id && (
                          <ExampleDraft
                            key={item.id}
                            folder={exampleDraft.folder}
                            title={exampleDraft.title}
                            disabled={disabled}
                            onAdd={addExample}
                            onCancel={() => setExampleDraft(null)}
                          />
                        )}
                      </DocumentDecision>
                    );
                  })}
                </div>
              </>
            )}
            {page === "Folders" && (
              <>
                <div className="page-heading">
                  <div>
                    <div className="page-kicker">YOUR FILING SYSTEM</div>
                    <h1>Folders</h1>
                    <p>
                      A folder for each subject. A short rule for what belongs.
                    </p>
                  </div>
                </div>
                {dirty && (
                  <div className="tree-save-bar notice-with-actions">
                    <span>
                      {state.running
                        ? "Saving rules pauses Tuck."
                        : "You have unsaved changes."}
                    </span>
                    <div className="notice-actions">
                      <button
                        className="text-button"
                        disabled={disabled}
                        onClick={reload}
                      >
                        Discard changes
                      </button>
                      <button
                        className="primary compact"
                        disabled={disabled || !!state.processing || !draft.root}
                        onClick={save}
                      >
                        <Check size={14} /> Save rules
                      </button>
                    </div>
                  </div>
                )}
                {categoryProposal ? (
                  categoryPreview
                ) : (
                  <div className="folder-actions">
                    <button
                      className="secondary"
                      disabled={setupLocked || !draft.useAi || !draft.root}
                      onClick={suggestCategories}
                    >
                      {suggestingCategories ? (
                        <LoaderCircle className="spin" size={15} />
                      ) : (
                        <Sparkles size={15} />
                      )}{" "}
                      {suggestingCategories
                        ? "Finding categories…"
                        : "Suggest categories"}
                    </button>
                    <button
                      className="text-button"
                      onClick={() => setPage("Settings")}
                    >
                      Adjust folder nesting <ArrowRight size={14} />
                    </button>
                  </div>
                )}
                {!categoryProposal && (
                  <>
                    {folderTree}
                    <div className="rule-check-toolbar">
                      <p>Find folders with matching descriptions.</p>
                      <button
                        className="secondary compact"
                        disabled={
                          disabled ||
                          !!state.processing ||
                          !draft.root ||
                          draft.rules.filter((r) => r.enabled).length < 2
                        }
                        onClick={checkRules}
                      >
                        {checkingRules ? (
                          <LoaderCircle size={15} className="spin" />
                        ) : (
                          <Check size={15} />
                        )}
                        {checkingRules ? "Checking rules…" : "Check rules"}
                      </button>
                    </div>
                    {ruleCheck && (
                      <RuleCheckResults
                        key={ruleCheck.key}
                        issues={ruleCheck.issues}
                        stale={
                          JSON.stringify(draft) !== ruleCheck.baseline ||
                          draft.instructionsRevision !==
                            state.preferences.instructionsRevision ||
                          draft.settingsRevision !==
                            state.preferences.settingsRevision
                        }
                        disabled={disabled}
                        onAdd={addClarification}
                        onDismiss={() => setRuleCheck(null)}
                      />
                    )}
                  </>
                )}
              </>
            )}
            {page === "Settings" && (
              <Settings
                preferences={draft}
                onChange={setPreferences}
                apiKey={apiKey}
                onKey={setApiKey}
                disabled={disabled || !!state.processing}
                dirty={dirty}
                onSave={save}
                onDiscard={reload}
              />
            )}
          </div>
        </div>
      </main>
      {welcome && <Welcome onClose={closeWelcome} />}
    </div>
  );
}
export default App;
