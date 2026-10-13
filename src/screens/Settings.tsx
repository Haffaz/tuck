import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { LoaderCircle } from "lucide-react";
import type { ModelConnection, Preferences } from "../types";

const presets: Record<ModelConnection["kind"], string> = {
  ollama: "http://127.0.0.1:11434",
  openai: "https://api.openai.com/v1",
  anthropic: "https://api.anthropic.com",
};

type Props = {
  preferences: Preferences;
  onChange: (p: Preferences) => void;
  apiKey: string;
  onKey: (key: string) => void;
  disabled: boolean;
  dirty: boolean;
  onSave: () => void;
  onDiscard: () => void;
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
}: Props) {
  const [status, setStatus] = useState("");
  const [checking, setChecking] = useState(false);
  const [models, setModels] = useState<string[]>([]);
  const custom = p.aiModel !== "apple";
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
  const check = async (listModels = false) => {
    setChecking(true);
    setStatus("");
    try {
      const response = custom
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

  return (
    <section className="settings" aria-label="Settings">
      <h1>Settings</h1>
      {!p.root && (
        <p className="muted">
          Choose a folder first; settings are saved with it.
        </p>
      )}
      <fieldset disabled={disabled || checking}>
        <div className="setting">
          <span className="setting-label" id="model-label">
            AI model
          </span>
          <div
            className="segmented"
            role="radiogroup"
            aria-labelledby="model-label"
          >
            <label className={!custom ? "selected" : ""}>
              <input
                type="radio"
                name="model"
                checked={!custom}
                onChange={() => {
                  onKey("");
                  onChange({ ...p, useAi: true, aiModel: "apple" });
                }}
              />
              Apple Intelligence
            </label>
            <label className={custom ? "selected" : ""}>
              <input
                type="radio"
                name="model"
                checked={custom}
                onChange={() =>
                  onChange({ ...p, useAi: true, aiModel: "custom" })
                }
              />
              Other
            </label>
          </div>
        </div>
        {custom && (
          <div className="connection">
            <label>
              Provider
              <select
                aria-label="Connection type"
                value={p.connection.kind}
                onChange={(e) => {
                  const kind = e.target.value as ModelConnection["kind"];
                  connection({ kind, baseUrl: presets[kind], model: "" });
                }}
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
              API key
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
            <p className="muted">
              Document text is sent to {p.connection.baseUrl}. Keys are kept in
              macOS Keychain.{" "}
              <button
                type="button"
                className="link"
                onClick={() => void check(true)}
              >
                Find models
              </button>
            </p>
          </div>
        )}
        <div className="setting-inline">
          <button type="button" className="link" onClick={() => void check()}>
            Test connection
          </button>
          {(checking || status) && (
            <span role="status" className="muted">
              {checking ? (
                <>
                  <LoaderCircle size={12} className="spin" aria-hidden="true" />{" "}
                  Checking…
                </>
              ) : (
                status
              )}
            </span>
          )}
        </div>

        <div className="setting">
          <span className="setting-label" id="nesting-label">
            Folder levels
          </span>
          <div
            className="segmented"
            role="radiogroup"
            aria-labelledby="nesting-label"
          >
            {["One level", "Up to 2 levels"].map((name, index) => (
              <label
                key={name}
                className={p.folderNesting === index + 1 ? "selected" : ""}
              >
                <input
                  type="radio"
                  name="folder-nesting"
                  checked={p.folderNesting === index + 1}
                  onChange={() => onChange({ ...p, folderNesting: index + 1 })}
                />
                {name}
              </label>
            ))}
          </div>
        </div>

        <label className="setting toggle">
          <span className="setting-label">
            Rename files
            <small>For example “Bank Statement - Nordea - 2026-03”.</small>
          </span>
          <input
            type="checkbox"
            role="switch"
            checked={p.rename}
            onChange={(e) => onChange({ ...p, rename: e.target.checked })}
          />
        </label>

        <details className="advanced">
          <summary>Advanced</summary>
          <label>
            Folder preferences
            <textarea
              aria-label="Folder creation preferences"
              rows={2}
              maxLength={1200}
              value={p.categoryInstructions}
              placeholder="For example: Keep work and personal documents apart."
              onChange={(e) =>
                onChange({ ...p, categoryInstructions: e.target.value })
              }
            />
          </label>
          <label>
            Filing instructions
            <textarea
              aria-label="Document instructions"
              rows={3}
              value={p.instructions}
              placeholder="For example: Use short English names."
              onChange={(e) => onChange({ ...p, instructions: e.target.value })}
            />
          </label>
          {!!p.legacyFolders?.length && (
            <p className="muted">
              {p.legacyFolders.length} saved folders are deeper than two levels.
              Their files stay in place.
            </p>
          )}
        </details>
      </fieldset>
      <footer className="actions">
        {dirty && <span className="muted unsaved">Unsaved changes</span>}
        <button
          className="link"
          disabled={disabled || !dirty}
          onClick={onDiscard}
        >
          Discard
        </button>
        <button
          className="primary"
          disabled={
            disabled ||
            !dirty ||
            !p.root ||
            checking ||
            (custom && !p.connection.model.trim())
          }
          onClick={onSave}
        >
          Save
        </button>
      </footer>
    </section>
  );
}
