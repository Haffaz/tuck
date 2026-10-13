import { LoaderCircle } from "lucide-react";

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
