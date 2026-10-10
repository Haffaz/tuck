import { useEffect, useRef, useState, type ReactNode } from "react";
import { ChevronDown, FileText, Folder } from "lucide-react";
import type { ReviewItem, Rule } from "./types";

type Props = {
  item: ReviewItem;
  rules: Rule[];
  values: { folder: string; title: string };
  disabled: boolean;
  canFile: boolean;
  selectable: boolean;
  selected: boolean;
  canRemember: boolean;
  enabled: (folder: string) => boolean;
  onPreview: () => void;
  onSelect: (selected: boolean) => void;
  onEdit: (patch: Partial<{ folder: string; title: string }>) => void;
  onFile: () => void;
  onSkip: () => void;
  onRetry: () => void;
  onRemember: () => void;
  children?: ReactNode;
};

// The parent supplies one folder's documents and rules; disclosures never own app state.
export default function DocumentDecision(props: Props) {
  const { item, values, disabled } = props;
  const name = item.original.split("/").pop() || item.original;
  const picker = useRef<HTMLDetailsElement>(null);
  const [opensUp, setOpensUp] = useState(false);
  useEffect(() => {
    const close = (event: PointerEvent) => {
      if (picker.current && !picker.current.contains(event.target as Node))
        picker.current.open = false;
    };
    document.addEventListener("pointerdown", close);
    return () => document.removeEventListener("pointerdown", close);
  }, []);
  return (
    <article
      className="review-card decision-card"
      aria-label={`Review ${name}`}
    >
      <div className="review-card-heading">
        {props.selectable && (
          <input
            type="checkbox"
            aria-label={`Select ${name}`}
            checked={props.selected}
            disabled={disabled}
            onChange={(e) => props.onSelect(e.target.checked)}
          />
        )}
        <FileText size={20} />
        <div>
          <button
            className="document-preview"
            aria-label={`Preview ${name}`}
            onClick={props.onPreview}
          >
            {name}
          </button>
          <p>
            {item.status === "failed"
              ? "Couldn’t finish this document"
              : "Where should this go?"}
          </p>
        </div>
      </div>
      {item.status === "review" ? (
        <div className="decision-actions">
          <details
            className={`category-picker ${opensUp ? "opens-up" : ""}`}
            ref={picker}
            onToggle={(event) => {
              if (!event.currentTarget.open) return;
              const rect = event.currentTarget.getBoundingClientRect();
              const menu =
                event.currentTarget.querySelector<HTMLElement>(
                  ".category-choices",
                );
              const height = Math.min(menu?.scrollHeight || 240, 240) + 12;
              setOpensUp(
                window.innerHeight - rect.bottom < height &&
                  rect.top > height + 64,
              );
            }}
            onKeyDown={(e) => {
              if (e.key === "Escape" && picker.current) {
                picker.current.open = false;
                picker.current.querySelector("summary")?.focus();
              }
            }}
          >
            <summary
              aria-label={`Category for ${name}`}
              aria-disabled={disabled}
              onClick={(e) => {
                if (disabled) e.preventDefault();
              }}
            >
              <Folder size={16} />
              <span>{values.folder || "Choose category"}</span>
              <ChevronDown size={14} />
            </summary>
            <div
              className="category-choices"
              aria-label={`Categories for ${name}`}
            >
              {props.rules.map((rule) => (
                <button
                  type="button"
                  key={rule.folder}
                  disabled={disabled || !props.enabled(rule.folder)}
                  aria-pressed={values.folder === rule.folder}
                  onClick={() => {
                    props.onEdit({ folder: rule.folder });
                    if (picker.current) {
                      picker.current.open = false;
                      picker.current.querySelector("summary")?.focus();
                    }
                  }}
                >
                  {rule.folder}
                  {!props.enabled(rule.folder) && " (disabled)"}
                </button>
              ))}
              {!props.rules.length && (
                <p>Add a category in Folders to file this document.</p>
              )}
            </div>
          </details>
          <button
            className="primary compact"
            disabled={!props.canFile}
            onClick={props.onFile}
          >
            File document
          </button>
        </div>
      ) : (
        <div className="decision-actions">
          <button
            className="secondary compact"
            disabled={disabled}
            onClick={props.onRetry}
          >
            Try again
          </button>
        </div>
      )}
      <div className="decision-secondary">
        <details className="quiet-details decision-options">
          <summary>More options</summary>
          {item.status === "review" && (
            <label className="decision-filename">
              Filename <small>Extension stays the same</small>
              <input
                aria-label={`Filename for ${name}`}
                value={values.title}
                disabled={disabled}
                onChange={(e) => props.onEdit({ title: e.target.value })}
              />
            </label>
          )}
          <div className="decision-extra-actions">
            {item.status === "review" && (
              <button
                className="text-button"
                disabled={disabled}
                onClick={props.onRetry}
              >
                Analyze again
              </button>
            )}
            {props.canRemember && (
              <button
                className="text-button"
                disabled={
                  disabled ||
                  !values.title.trim() ||
                  !props.enabled(values.folder)
                }
                onClick={props.onRemember}
              >
                Remember this example
              </button>
            )}
          </div>
          <details className="quiet-details document-details">
            <summary>Details</summary>
            <p className="review-reason">{item.reason}</p>
            <p className="folder-location">{item.original}</p>
            {item.destination && (
              <p className="review-path">
                <strong>Last validated destination</strong> {item.destination}
              </p>
            )}
          </details>
        </details>
        <button
          className="text-button decision-skip"
          disabled={disabled}
          onClick={props.onSkip}
        >
          Leave for now
        </button>
      </div>
      {props.children}
    </article>
  );
}
