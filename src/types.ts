export type Rule = { folder: string; keywords: string };
export type Preferences = {
  root: string;
  recursive: boolean;
  rename: boolean;
  maxDepth: number;
  useAi: boolean;
  model: string;
  instructions: string;
  rules: Rule[];
};
export type Activity = {
  id: string;
  timestamp: number;
  original: string;
  destination: string;
  folder: string;
  reason: string;
  status:
    "pending" | "undo_pending" | "sorted" | "review" | "undone" | "failed";
  hash: string;
  copied: boolean;
  duplicate: boolean;
};
export type Snapshot = {
  preferences: Preferences;
  watching: boolean;
  activity: Activity[];
  pending: number;
  processing: string | null;
  error: string | null;
};
export const initial: Snapshot = {
  preferences: {
    root: "",
    recursive: true,
    rename: true,
    maxDepth: 2,
    useAi: true,
    model: "qwen3:4b",
    instructions:
      "Use clear English document names. Keep application bundles together. Never infer an expiry date or call a residence permit a visa.",
    rules: [
      { folder: "Migration/AGENCY", keywords: "agency" },
      {
        folder: "Migration",
        keywords:
          "residence permit, migrationsverket, uppehållstillstånd, immigration",
      },
      {
        folder: "Finances",
        keywords: "bank statement, tax return, kontoutdrag, skatteverket",
      },
      {
        folder: "Work",
        keywords:
          "payslip, employment contract, lönespecifikation, anställningsavtal",
      },
      { folder: "Home", keywords: "rental agreement, tenancy, hyresavtal" },
      { folder: "Insurance", keywords: "insurance policy, försäkringsbrev" },
      {
        folder: "Health",
        keywords: "medical report, prescription, läkarintyg",
      },
    ],
  },
  watching: false,
  activity: [],
  pending: 0,
  processing: null,
  error: null,
};
