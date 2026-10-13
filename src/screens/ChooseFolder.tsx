import { FolderOpen, FolderTree, ScanText, Sparkles } from "lucide-react";

const steps = [
  { Icon: ScanText, title: "Reads", text: "Every document, on this Mac" },
  { Icon: Sparkles, title: "Suggests", text: "Folders that fit what you have" },
  { Icon: FolderTree, title: "Organizes", text: "Only when you say so" },
];

export function ChooseFolder({
  disabled,
  onChoose,
}: {
  disabled: boolean;
  onChoose: () => void;
}) {
  return (
    <section className="hero">
      <span className="hero-tile" aria-hidden="true">
        <FolderOpen size={34} strokeWidth={1.6} />
      </span>
      <h1>A place for every document</h1>
      <p>
        Choose a messy folder. Tuck reads it and suggests a tidy home for each
        file.
      </p>
      <button className="primary large" disabled={disabled} onClick={onChoose}>
        <FolderOpen size={16} aria-hidden="true" />
        Choose folder
      </button>
      <ol className="steps">
        {steps.map(({ Icon, title, text }) => (
          <li key={title}>
            <Icon size={18} aria-hidden="true" />
            <strong>{title}</strong>
            <span>{text}</span>
          </li>
        ))}
      </ol>
    </section>
  );
}
