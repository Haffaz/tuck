import { useRef, useState } from "react";
import { nameOf } from "../folders";

/** Inline rename: Enter or leaving the field saves a valid name; Escape cancels. */
export default function RenameField({
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
