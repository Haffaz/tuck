import { useCallback, useEffect, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { ArrowLeft, FolderOpen, Settings2, X } from "lucide-react";
import {
  initial,
  type CategoryProposal,
  type Preferences,
  type ReviewItem,
  type Snapshot,
} from "./types";
import Settings from "./Settings";
import {
  ChooseFolder,
  Proposal,
  Results,
  SavedFolders,
  Working,
  nameOf,
} from "./Home";
import "./tuck.css";

const desktop = isTauri();

function Mark() {
  return (
    <span className="mark" aria-hidden="true">
      <svg viewBox="0 0 32 32" fill="none">
        <path
          d="M7 11.5 16 7l9 4.5v10L16 26l-9-4.5v-10Z"
          stroke="currentColor"
          strokeWidth="1.8"
          strokeLinejoin="round"
        />
        <path
          d="m7 12 9 4.5 9-4.5M16 17v9"
          stroke="currentColor"
          strokeWidth="1.8"
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
  const [settings, setSettings] = useState(false);
  const [busy, setBusy] = useState(false);
  const busyRef = useRef(false);
  const [working, setWorking] = useState("");
  const [loaded, setLoaded] = useState(false);
  const [error, setError] = useState("");
  const [proposal, setProposal] = useState<CategoryProposal | null>(null);
  const [watch, setWatch] = useState(false);
  const [apiKey, setApiKey] = useState("");
  const dirty =
    !!apiKey || JSON.stringify(draft) !== JSON.stringify(state.preferences);
  const disabled = busy || !loaded || !desktop;

  const setPreferences = (p: Preferences) => {
    draftRef.current = p;
    setDraft(p);
  };
  const requests = useRef(0);
  const refresh = useCallback(async () => {
    // Polls and actions overlap; only the newest response may update the screen.
    const request = ++requests.current;
    const next = await invoke<Snapshot>("get_state");
    if (request !== requests.current) return next;
    next.preferences = { ...initial.preferences, ...next.preferences };
    // Keep unsaved settings edits; otherwise follow what is on disk.
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
        setWorking("");
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
      try {
        if (!disposed) await refresh();
      } catch (e) {
        if (!disposed) setError(String(e));
      }
    };
    void poll();
    const timer = window.setInterval(() => void poll(), 1000);
    return () => {
      disposed = true;
      clearInterval(timer);
    };
  }, [refresh]);
  useEffect(() => {
    const keys = (event: KeyboardEvent) => {
      if (event.metaKey && event.key === ",") {
        event.preventDefault();
        setSettings(true);
      }
      if (event.key === "Escape") setSettings(false);
    };
    window.addEventListener("keydown", keys);
    return () => window.removeEventListener("keydown", keys);
  }, []);

  const suggest = async () => {
    setProposal(null);
    setWorking("Reading your documents…");
    const current = await invoke<Snapshot>("get_state");
    if (!current.collection) await invoke("map_folder");
    setWorking("Suggesting folders…");
    setProposal(await invoke<CategoryProposal>("suggest_categories"));
  };
  const chooseFolder = () =>
    run(async () => {
      const root = await open({
        directory: true,
        multiple: false,
        title: "Choose a folder to organize",
      });
      if (typeof root !== "string") return;
      await invoke("choose_folder", { root });
      setProposal(null);
      setWatch(false);
      setSettings(false);
      setWorking("Reading your documents…");
      await invoke("map_folder");
      const next = await refresh();
      if (!next.preferences.rules.length) await suggest();
    });
  const organizeProposal = () =>
    run(async () => {
      if (!proposal) return;
      setWorking("Moving files…");
      // Current settings, not the copy captured when suggesting; only files whose folder
      // still exists move, the rest wait for a decision.
      const current = await invoke<Snapshot>("get_state");
      const folders = new Set(proposal.rules.map((r) => r.folder));
      await invoke("save_preferences", {
        preferences: { ...current.preferences, rules: proposal.rules },
      });
      await invoke("organize_proposal", {
        files: proposal.files.filter((f) => folders.has(f.folder)),
        watch,
      });
      setProposal(null);
    });
  const organizeSaved = () =>
    run(async () => {
      if (!state.collection) {
        setWorking("Reading your documents…");
        await invoke("map_folder");
      }
      await invoke("start_tuck", { watch });
    });
  const fileItem = (item: ReviewItem, folder: string) =>
    run(async () => {
      await invoke("update_review", { id: item.id, folder, title: item.title });
      const fresh = await invoke<Snapshot>("get_state");
      const updated = fresh.items.find((i) => i.id === item.id);
      if (!updated?.destination)
        throw new Error("Choose a folder for this file.");
      await invoke("organize_documents", {
        approvals: [{ id: updated.id, destination: updated.destination }],
      });
    });
  const finish = () =>
    run(async () => {
      await invoke("clear_batch");
      setWorking("Reading your documents…");
      await invoke("map_folder");
    });
  const saveSettings = () =>
    run(async () => {
      await invoke("save_preferences", {
        preferences: draft,
        apiKey: apiKey || null,
      });
      setApiKey("");
      setPreferences((await refresh()).preferences);
      setSettings(false);
    });
  const discardSettings = () => {
    setApiKey("");
    setPreferences(state.preferences);
  };

  const importFiles = useCallback(
    (paths: string[]) =>
      run(async () => {
        await invoke("import_documents", { paths });
      }),
    [run],
  );
  useEffect(() => {
    if (!desktop) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === "drop")
          void importFiles(event.payload.paths);
      })
      .then((fn) => (disposed ? fn() : (unlisten = fn)))
      .catch((e) => setError(String(e)));
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [importFiles]);

  const root = state.preferences.root;
  const progress = working && (state.processing || working);
  let body;
  if (!root)
    body = <ChooseFolder disabled={disabled} onChoose={chooseFolder} />;
  else if (progress) body = <Working message={progress} />;
  else if (proposal)
    body = (
      <Proposal
        proposal={proposal}
        disabled={disabled}
        watch={watch}
        onWatch={setWatch}
        onChange={setProposal}
        onOrganize={organizeProposal}
        onDiscard={() => run(suggest)}
      />
    );
  else if (state.items.length)
    body = (
      <Results
        state={state}
        disabled={disabled}
        onFile={fileItem}
        onSkip={(item) => run(() => invoke("skip_document", { id: item.id }))}
        onPreview={(item) =>
          void invoke("preview_document", { id: item.id }).catch((e) =>
            setError(String(e)),
          )
        }
        onReveal={() =>
          void revealItemInDir(root).catch((e) => setError(String(e)))
        }
        onStop={() => run(() => invoke("stop_tuck"))}
        onDone={finish}
      />
    );
  else if (state.preferences.rules.length)
    body = (
      <SavedFolders
        state={state}
        disabled={disabled}
        watch={watch}
        onWatch={setWatch}
        onOrganize={organizeSaved}
        onSuggest={() => run(suggest)}
      />
    );
  else
    body = (
      <section className="empty">
        <p>Tuck has not suggested folders for this folder yet.</p>
        <button
          className="primary"
          disabled={disabled}
          onClick={() => run(suggest)}
        >
          Suggest folders
        </button>
      </section>
    );

  return (
    <div className="app">
      <header className="titlebar" data-tauri-drag-region>
        <div className="brand">
          <Mark />
          Tuck
        </div>
        {settings ? (
          <button
            className="icon"
            aria-label="Back"
            onClick={() => setSettings(false)}
          >
            <ArrowLeft size={17} />
          </button>
        ) : (
          <button
            className="icon"
            aria-label="Settings"
            onClick={() => setSettings(true)}
          >
            <Settings2 size={17} />
          </button>
        )}
      </header>
      <main className="content">
        {error && (
          <div className="alert" role="alert">
            <span>{error}</span>
            <button
              className="icon"
              aria-label="Dismiss"
              onClick={() => setError("")}
            >
              <X size={14} />
            </button>
          </div>
        )}
        {settings ? (
          <Settings
            preferences={draft}
            onChange={setPreferences}
            apiKey={apiKey}
            onKey={setApiKey}
            disabled={disabled}
            dirty={dirty}
            onSave={saveSettings}
            onDiscard={discardSettings}
          />
        ) : (
          <>
            {root && (
              <div className="folder-bar">
                <span className="folder-icon" aria-hidden="true">
                  <FolderOpen size={15} />
                </span>
                <span className="folder-title">
                  <strong title={root}>{nameOf(root)}</strong>
                  <small>{root.replace(/^\/Users\/[^/]+/, "~")}</small>
                </span>
                <button
                  className="link"
                  disabled={disabled || state.running}
                  onClick={chooseFolder}
                >
                  Change
                </button>
              </div>
            )}
            {body}
          </>
        )}
      </main>
    </div>
  );
}

export default App;
