export type RuleIssue = {
  folder: string;
  otherFolder: string;
  issue: string;
  clarification: string;
};
export type Rule = {
  folder: string;
  instruction: string;
  enabled: boolean;
  documentTypes?: string[];
};
export type LocalModel = "apple" | "gemmaE2b" | "gemmaE4b" | "custom";
export type ModelConnection = {
  kind: "ollama" | "openai" | "anthropic";
  baseUrl: string;
  model: string;
};
export type Preferences = {
  root: string;
  recursive: boolean;
  rename: boolean;
  maxDepth: number;
  legacyFolders?: string[];
  useAi: boolean;
  aiModel: LocalModel;
  connection: ModelConnection;
  folderNesting: number;
  categoryInstructions: string;
  useAgent: boolean;
  useMemory: boolean;
  instructions: string;
  instructionsRevision: string;
  settingsRevision: string;
  rules: Rule[];
};
export type LanguageInfo = {
  name: string;
  code: string;
  status: string;
  message: string;
};
export type ReviewItem = {
  language?: LanguageInfo | null;
  id: string;
  original: string;
  destination: string;
  folder: string;
  title: string;
  reason: string;
  status: "queued" | "reading" | "review" | "organized" | "failed" | "skipped";
  copied: boolean;
  confident: boolean;
};
export type Collection = {
  documents: {
    path: string;
    folder: string;
    duplicateOf: string | null;
    error: string | null;
  }[];
  folders: { path: string; documents: number }[];
  existingRules: Rule[];
  unreadable: number;
  duplicates: number;
};
export type Snapshot = {
  collection: Collection | null;
  running: boolean;
  watching: boolean;
  preferences: Preferences;
  items: ReviewItem[];
  processing: string | null;
  error: string | null;
};
export const initial: Snapshot = {
  collection: null,
  running: false,
  watching: false,
  preferences: {
    root: "",
    recursive: true,
    rename: true,
    maxDepth: 2,
    useAi: true,
    aiModel: "apple",
    connection: {
      kind: "ollama",
      baseUrl: "http://127.0.0.1:11434",
      model: "",
    },
    folderNesting: 2,
    categoryInstructions: "",
    useAgent: true,
    useMemory: false,
    instructionsRevision: "",
    settingsRevision: "",
    instructions:
      "Use clear English document names. Describe the document accurately without inventing names or dates.",
    rules: [
      {
        enabled: true,
        folder: "Identity & Immigration",
        instruction:
          "Identity documents, residence permits, visas and immigration decisions or applications.",
      },
      {
        enabled: true,
        folder: "Banking & Taxes",
        instruction: "Bank statements, account records and tax documents.",
      },
      {
        enabled: true,
        folder: "Employment & Pay",
        instruction:
          "Employment contracts, employment certificates, payslips and other employment or pay records.",
      },
      {
        enabled: true,
        folder: "Housing & Utilities",
        instruction:
          "Rental agreements and household utility bills, including electricity bills.",
      },
      {
        enabled: true,
        folder: "Insurance",
        instruction:
          "Insurance policies and certificates of cover, including home and travel insurance.",
      },
      {
        enabled: true,
        folder: "Medical Records",
        instruction:
          "Medical certificates, reports, prescriptions and other clinical records.",
      },
      {
        enabled: true,
        folder: "Maintenance & Repairs",
        instruction:
          "Equipment inspections, maintenance reports and repair records.",
      },
    ],
  },
  items: [],
  processing: null,
  error: null,
};

/** Where a suggestion puts one document; title is empty when renaming is off. */
export type Placement = { path: string; folder: string; title: string };
export type CategoryProposal = {
  rules: Rule[];
  files: Placement[];
  sampled: number;
  skipped: number;
  reviewFiles?: { path: string; reason: string }[];
  limited: boolean;
  preferences: Preferences;
};
