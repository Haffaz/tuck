import { useEffect, useLayoutEffect, useRef, useState } from "react";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  FolderOpen,
  Settings2,
  ShieldCheck,
  X,
} from "lucide-react";

export const welcomeKey = "tuck.welcome.v2";
const steps = [
  {
    label: "A PLACE TO START",
    title: "A home for your documents.",
    icon: FolderOpen,
    description: "Choose your structure. Tuck handles the filing.",
    items: [
      [
        "Choose a destination",
        "Pick one document folder. Tuck maps its documents first; choose an existing or suggested structure before clicking Start Tuck.",
      ],
      [
        "Drop in a batch",
        "Add PDFs, scans or text files. Your chosen model suggests folders and names. Apple Intelligence is the default.",
      ],
      [
        "Help only when needed",
        "Clear matches are organized automatically after Start Tuck. Uncertain documents stay put so you can choose where they belong.",
      ],
    ],
  },
  {
    label: "MAKE IT YOURS",
    title: "Your folders. Your names.",
    icon: Settings2,
    description: "A simple structure that follows the way you think.",
    items: [
      [
        "Describe each folder",
        "Write what belongs there in your own words. Root rules apply everywhere; child folders add their own guidance.",
      ],
      [
        "Turn destinations on or off",
        "Disabled folders are excluded from suggestions. Disabling a parent also disables its child destinations.",
      ],
      [
        "Settings that stay with your files",
        "Preferences live in .tuck/settings.json. Folder rules live in .tuck/instructions.md. You can edit them in Tuck or a text editor.",
      ],
    ],
  },
  {
    label: "YOU’RE IN CONTROL",
    title: "Set it up. Let Tuck help.",
    icon: ShieldCheck,
    description:
      "Apple Intelligence runs on your Mac. Other connections use the service you choose.",
    items: [
      [
        "You choose what happens",
        "Start Tuck enables automatic moves inside your folder. Files dropped from outside are copied, keeping their originals.",
      ],
      [
        "Uncertainty stays in review",
        "If Tuck cannot decide, choose a folder yourself. Changed documents and occupied destinations are checked again before organizing.",
      ],
      [
        "One session at a time",
        "Optionally watch for new and changed files while Tuck is open. Closing the app stops watching and clears the session. There is no saved undo history.",
      ],
    ],
  },
];

export default function Welcome({ onClose }: { onClose: () => void }) {
  const [step, setStep] = useState(0);
  const dialog = useRef<HTMLDialogElement>(null);
  const heading = useRef<HTMLHeadingElement>(null);
  const content = steps[step];
  const Icon = content.icon;
  useLayoutEffect(() => {
    const element = dialog.current;
    const previousFocus = document.activeElement;
    element?.showModal();
    return () => {
      element?.close();
      if (previousFocus instanceof HTMLElement) previousFocus.focus();
    };
  }, []);
  useEffect(() => {
    heading.current?.focus();
  }, [step]);
  return (
    <dialog
      ref={dialog}
      className="welcome-dialog"
      aria-labelledby="welcome-title"
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
    >
      <div className="welcome-top">
        <span>TUCK · A QUICK INTRODUCTION</span>
        <button
          className="icon-button"
          aria-label="Close introduction"
          onClick={onClose}
        >
          <X size={19} />
        </button>
      </div>
      <div className="welcome-content">
        <span className="welcome-icon">
          <Icon size={30} strokeWidth={1.5} />
        </span>
        <div className="eyebrow">{content.label}</div>
        <h1 id="welcome-title" ref={heading} tabIndex={-1}>
          {content.title}
        </h1>
        <p className="welcome-description">{content.description}</p>
        <div className="welcome-points">
          {content.items.map(([title, text], index) => (
            <div key={title}>
              <span>{index + 1}</span>
              <div>
                <h3>{title}</h3>
                <p>{text}</p>
              </div>
            </div>
          ))}
        </div>
      </div>
      <footer className="welcome-footer">
        <div
          className="welcome-progress"
          aria-label={`Introduction step ${step + 1} of ${steps.length}`}
        >
          {steps.map((_, index) => (
            <span key={index} className={index === step ? "active" : ""} />
          ))}
        </div>
        <div>
          {step > 0 ? (
            <button className="secondary" onClick={() => setStep(step - 1)}>
              <ArrowLeft size={14} /> Back
            </button>
          ) : (
            <button className="text-button muted" onClick={onClose}>
              Skip introduction
            </button>
          )}
          <button
            className="primary"
            onClick={() =>
              step === steps.length - 1 ? onClose() : setStep(step + 1)
            }
          >
            {step === steps.length - 1 ? (
              <>
                Start using Tuck <Check size={15} />
              </>
            ) : (
              <>
                Next <ArrowRight size={15} />
              </>
            )}
          </button>
        </div>
      </footer>
    </dialog>
  );
}
