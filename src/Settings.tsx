import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Check,
  ChevronDown,
  Cpu,
  Folder,
  LoaderCircle,
  Plus,
  Sparkles,
} from "lucide-react";
import type { Preferences, ModelConnection } from "./types";

const nestingLevels = [
  {
    name: "One level",
    hint: "Keep folders at the top level.",
    example: ["Work"],
  },
  {
    name: "Up to 2 levels",
    hint: "Add a subfolder when it helps you find documents.",
    example: ["Work", "Project Orion"],
  },
];
const presets: Record<string, string> = {
  ollama: "http://127.0.0.1:11434",
  openai: "https://api.openai.com/v1",
  anthropic: "https://api.anthropic.com",
};
export default function Settings({
  preferences: p,
  onChange,
  apiKey,
  onKey,
  disabled,
  dirty,
  onSave,
  onDiscard,
}: {
  preferences: Preferences;
  onChange: (p: Preferences) => void;
  apiKey: string;
  onKey: (key: string) => void;
  disabled: boolean;
  dirty: boolean;
  onSave: () => void;
  onDiscard: () => void;
}) {
  const [editing, setEditing] = useState(p.aiModel !== "apple");
  const [status, setStatus] = useState("");
  const [checking, setChecking] = useState(false);
  const [models, setModels] = useState<string[]>([]);
  const nesting = nestingLevels[p.folderNesting - 1] || nestingLevels[1];
  useEffect(() => {
    setStatus("");
    setModels([]);
  }, [
    p.aiModel,
    p.connection.kind,
    p.connection.baseUrl,
    p.connection.model,
    apiKey,
  ]);
  const connection = (patch: Partial<ModelConnection>) => {
    if (patch.baseUrl !== undefined || patch.kind !== undefined) onKey("");
    onChange({
      ...p,
      useAi: true,
      aiModel: "custom",
      connection: { ...p.connection, ...patch },
    });
  };
  const addConnection = () => {
    setEditing(true);
    onChange({
      ...p,
      useAi: true,
      aiModel: "custom",
      connection:
        p.aiModel === "gemmaE2b" || p.aiModel === "gemmaE4b"
          ? {
              kind: "ollama",
              baseUrl: presets.ollama,
              model:
                p.aiModel === "gemmaE2b"
                  ? "gemma4:e2b-it-q4_K_M"
                  : "gemma4:e4b-it-q4_K_M",
            }
          : p.connection,
    });
  };
  const check = async (listModels = false) => {
    setChecking(true);
    setStatus("");
    try {
      const response =
        p.aiModel === "custom"
          ? await invoke<string>("check_connection", {
              connection: {
                ...p.connection,
                model: p.connection.model || (listModels ? "discovery" : ""),
              },
              apiKey: apiKey || null,
              listModels,
            })
          : await invoke<string>("check_model", { model: p.aiModel });
      if (listModels) {
        const ids = JSON.parse(response) as string[];
        setModels(ids);
        setStatus(
          ids.length
            ? `${ids.length} models found.`
            : "No models found. Enter a model ID.",
        );
      } else setStatus(response);
    } catch (e) {
      setStatus(String(e));
    } finally {
      setChecking(false);
    }
  };
  const discard = () => {
    setEditing(false);
    onDiscard();
  };
  return (
    <>
      <div className="page-heading settings-heading">
        <div>
          <div className="page-kicker">MAKE IT YOURS</div>
          <h1>Settings</h1>
          <p>How Tuck organizes your documents.</p>
        </div>
        <div className="settings-actions">
          <button
            className="text-button"
            disabled={disabled || !dirty || checking}
            onClick={discard}
          >
            Discard changes
          </button>
          <button
            className="primary"
            disabled={
              disabled ||
              !dirty ||
              !p.root ||
              checking ||
              (p.aiModel === "custom" && !p.connection.model.trim())
            }
            onClick={onSave}
          >
            <Check size={15} />
            Save settings
          </button>
        </div>
      </div>
      {!p.root && (
        <p className="notice">
          Choose a folder in Organize to save its settings.
        </p>
      )}
      <fieldset
        className="settings-fields settings-sheet"
        disabled={disabled || checking}
      >
        <section className="preference-section" aria-labelledby="ai-heading">
          <div className="preference-caption">
            <h2 id="ai-heading">AI connection</h2>
            <p>Choose the model Tuck uses.</p>
          </div>
          <div className="preference-body">
            <div className="provider-row">
              <label>
                <input
                  type="radio"
                  name="provider"
                  checked={p.useAi && p.aiModel === "apple"}
                  onChange={() => {
                    onKey("");
                    setEditing(false);
                    onChange({ ...p, useAi: true, aiModel: "apple" });
                  }}
                />
                <span className="provider-symbol">
                  <Sparkles size={17} />
                </span>
                <span>Apple Intelligence</span>
                <small>Default</small>
              </label>
              {p.aiModel === "apple" && (
                <button
                  type="button"
                  className="text-button"
                  onClick={() => void check()}
                >
                  Test connection
                </button>
              )}
            </div>
            {(editing || p.connection.model || p.aiModel !== "apple") && (
              <div className="provider-row custom-provider">
                <label>
                  <input
                    type="radio"
                    name="provider"
                    checked={p.useAi && p.aiModel !== "apple"}
                    onChange={() => {
                      onChange({ ...p, useAi: true, aiModel: "custom" });
                    }}
                  />
                  <span className="provider-symbol">
                    <Cpu size={17} />
                  </span>
                  <span>{p.connection.model || "Custom connection"}</span>
                </label>
                <button
                  type="button"
                  className="text-button"
                  aria-expanded={editing}
                  onClick={() => setEditing(!editing)}
                >
                  Edit <ChevronDown size={13} />
                </button>
              </div>
            )}
            {!editing && (
              <button
                type="button"
                className="add-connection"
                onClick={addConnection}
              >
                <Plus size={15} />
                {p.connection.model ? "Edit connection" : "Add connection"}
              </button>
            )}
            {editing && (
              <div className="connection-editor">
                <label>
                  Provider
                  <select
                    aria-label="Connection type"
                    value={p.connection.kind}
                    onChange={(e) =>
                      connection({
                        kind: e.target.value as ModelConnection["kind"],
                        baseUrl: presets[e.target.value],
                        model: "",
                      })
                    }
                  >
                    <option value="ollama">Ollama</option>
                    <option value="openai">OpenAI-compatible API</option>
                    <option value="anthropic">Anthropic</option>
                  </select>
                </label>
                <label>
                  API base URL
                  <input
                    aria-label="API base URL"
                    type="url"
                    value={p.connection.baseUrl}
                    onChange={(e) => connection({ baseUrl: e.target.value })}
                  />
                </label>
                <label>
                  Model ID
                  <input
                    aria-label="Model ID"
                    list="connection-models"
                    value={p.connection.model}
                    placeholder="Enter a model ID"
                    onChange={(e) => connection({ model: e.target.value })}
                  />
                </label>
                <datalist id="connection-models">
                  {models.map((id) => (
                    <option key={id} value={id} />
                  ))}
                </datalist>
                <label>
                  API key{" "}
                  <span className="optional">
                    {p.connection.kind === "ollama"
                      ? "Optional"
                      : "If required"}
                  </span>
                  <input
                    aria-label="API key"
                    type="password"
                    autoComplete="new-password"
                    spellCheck={false}
                    value={apiKey}
                    onChange={(e) => onKey(e.target.value)}
                    placeholder="Leave blank to keep the saved key"
                  />
                </label>
                <div className="connection-editor-footer">
                  <div className="connection-buttons">
                    <button
                      type="button"
                      className="secondary compact"
                      disabled={!p.connection.model.trim()}
                      onClick={() => void check()}
                    >
                      Test connection
                    </button>
                    <button
                      type="button"
                      className="text-button"
                      onClick={() => void check(true)}
                    >
                      Find models
                    </button>
                  </div>
                  <p className="setting-note">
                    Keys are saved in macOS Keychain.
                  </p>
                </div>
              </div>
            )}
            {p.aiModel !== "apple" && (
              <p className="connection-destination">
                Document text is sent to <strong>{p.connection.baseUrl}</strong>
                .
              </p>
            )}
            {(checking || status) && (
              <p role="status" className="connection-status">
                {checking ? (
                  <>
                    <LoaderCircle size={13} className="spin" />
                    Checking connection…
                  </>
                ) : (
                  status
                )}
              </p>
            )}
            {!p.useAi && (
              <p className="setting-note">
                AI suggestions are paused. Select a connection to enable them.
              </p>
            )}
          </div>
        </section>
        <section
          className="preference-section"
          aria-labelledby="nesting-heading"
        >
          <div className="preference-caption">
            <h2 id="nesting-heading">Folder nesting</h2>
            <p>How deep new folders can go.</p>
          </div>
          <div className="preference-body">
            <div
              className="nesting-options"
              role="radiogroup"
              aria-label="Folder nesting"
            >
              {nestingLevels.map((level, index) => (
                <label
                  className={`nesting-option ${p.folderNesting === index + 1 ? "selected" : ""}`}
                  key={level.name}
                >
                  <input
                    type="radio"
                    name="folder-nesting"
                    value={index + 1}
                    checked={p.folderNesting === index + 1}
                    onChange={() =>
                      onChange({ ...p, folderNesting: index + 1 })
                    }
                  />
                  <span>{level.name}</span>
                </label>
              ))}
            </div>
            <p className="nesting-hint">
              {nesting.hint} Fewer levels are always allowed.
            </p>
            {!!p.legacyFolders?.length && (
              <p className="setting-note">
                {p.legacyFolders.length} saved folders are deeper than two
                levels. Their files and rules stay in place; choose a folder
                within two levels to organize new documents.
              </p>
            )}
            <div
              className="nesting-example"
              aria-label="Example folder structure"
            >
              {nesting.example.map((name, index) => (
                <div key={name} style={{ paddingLeft: index * 20 }}>
                  {index > 0 && (
                    <span className="nesting-branch" aria-hidden="true">
                      └
                    </span>
                  )}
                  <Folder size={14} aria-hidden="true" /> <span>{name}</span>
                </div>
              ))}
            </div>
            <label
              className="creation-instructions"
              htmlFor="category-instructions"
            >
              Folder creation preferences{" "}
              <span className="optional">Optional</span>
              <textarea
                id="category-instructions"
                aria-label="Folder creation preferences"
                rows={2}
                maxLength={1200}
                value={p.categoryInstructions}
                placeholder="For example: Group reports by project and invoices by supplier."
                onChange={(e) =>
                  onChange({ ...p, categoryInstructions: e.target.value })
                }
              />
            </label>
            <p className="setting-note">
              Describe what new folders should contain. Nesting stays within the
              selected limit. Review suggestions before changing your folders.
            </p>
          </div>
        </section>
        <section
          className="preference-section"
          aria-labelledby="instructions-heading"
        >
          <div className="preference-caption">
            <h2 id="instructions-heading">Instructions</h2>
            <p>Apply to every document.</p>
          </div>
          <div className="preference-body instruction-fields">
            <label htmlFor="instructions">
              Document instructions
              <textarea
                id="instructions"
                aria-label="Document instructions"
                rows={3}
                value={p.instructions}
                placeholder="For example: Use short English names. Keep project documents together."
                onChange={(e) =>
                  onChange({ ...p, instructions: e.target.value })
                }
              />
            </label>
          </div>
        </section>
      </fieldset>
    </>
  );
}
