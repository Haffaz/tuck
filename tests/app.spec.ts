import { test, expect, type Page } from "@playwright/test";
import { initial as defaults } from "../src/types";

const ROOT = "/Users/test/Documents";
const initial = structuredClone(defaults);
initial.preferences.rules = [];

/** Mocks the Tauri bridge with a small in-memory backend. `saved` starts with folders. */
async function bridge(
  page: Page,
  options: { saved?: boolean; root?: string } = {},
) {
  await page.addInitScript(
    ({ initial, options, ROOT }) => {
      Object.defineProperty(window, "isTauri", { value: true });
      const state = structuredClone(initial);
      if (options.root !== undefined) state.preferences.root = options.root;
      if (options.saved) {
        state.preferences.root = ROOT;
        state.preferences.rules = [
          { folder: "Finance", instruction: "Bank statements.", enabled: true },
          { folder: "Home", instruction: "Rent and bills.", enabled: true },
        ];
      }
      const w = window as any;
      w.calls = [];
      w.state = state;
      let next = 1;
      Object.defineProperty(window, "__TAURI_INTERNALS__", {
        value: {
          metadata: {
            currentWindow: { label: "main" },
            currentWebview: { label: "main" },
          },
          transformCallback: () => next++,
          unregisterCallback: () => {},
          invoke: async (command: string, args: any = {}) => {
            if (command === "get_state") return structuredClone(state);
            w.calls.push({ command, args });
            switch (command) {
              case "plugin:event|listen":
                return next++;
              case "plugin:event|unlisten":
              case "plugin:opener|reveal_item_in_dir":
              case "preview_document":
                return;
              case "plugin:dialog|open":
                return ROOT;
              case "choose_folder":
                state.preferences.root = args.root;
                state.preferences.rules = [];
                return;
              case "map_folder":
                state.collection = {
                  documents: [],
                  folders: [{ path: "Finance", documents: 2 }],
                  existingRules: [],
                  unreadable: 0,
                  duplicates: 0,
                };
                return;
              case "suggest_categories":
                if (w.failSuggest) throw "Apple Intelligence is turned off.";
                if (w.holdSuggest)
                  await new Promise((resolve) => (w.releaseSuggest = resolve));
                return {
                  rules: [
                    {
                      folder: "Finance",
                      instruction: "Documents such as bank statement.",
                      enabled: true,
                      documentTypes: ["bank statement"],
                    },
                    {
                      folder: "Finance/Payslips",
                      instruction: "Documents such as payslip.",
                      enabled: true,
                    },
                    {
                      folder: "Home",
                      instruction: "Documents such as rental agreement.",
                      enabled: true,
                    },
                  ],
                  files: [
                    {
                      path: `${ROOT}/chase.pdf`,
                      folder: "Finance",
                      title: "Bank Statement - Chase",
                    },
                    {
                      path: `${ROOT}/nordea.pdf`,
                      folder: "Finance",
                      title: "Bank Statement - Nordea",
                    },
                    {
                      path: `${ROOT}/acme.pdf`,
                      folder: "Finance/Payslips",
                      title: "Payslip - Acme",
                    },
                    {
                      path: `${ROOT}/lease.pdf`,
                      folder: "Home",
                      title: "Rental Agreement",
                    },
                  ],
                  sampled: 4,
                  skipped: 1,
                  reviewFiles: [
                    {
                      path: `${ROOT}/blank.png`,
                      reason: "No readable text found.",
                    },
                  ],
                  limited: false,
                  preferences: structuredClone(state.preferences),
                  ...(w.proposalOverride || {}),
                };
              case "save_preferences":
                state.preferences = structuredClone(args.preferences);
                return;
              case "organize_proposal":
                state.running = true;
                state.watching = args.watch;
                state.items = [
                  ...args.files.map((f: any, i: number) => ({
                    id: `p${i}`,
                    original: f.path,
                    destination: `${ROOT}/${f.folder}/${f.title}.pdf`,
                    folder: f.folder,
                    title: f.title,
                    reason: "",
                    status: "organized",
                    copied: false,
                    confident: true,
                  })),
                  {
                    id: "blank",
                    original: `${ROOT}/blank.png`,
                    destination: "",
                    folder: "",
                    title: "blank",
                    reason: "No readable text found.",
                    status: "review",
                    copied: false,
                    confident: false,
                  },
                ];
                return args.files.length;
              case "start_tuck":
                state.running = true;
                state.watching = args.watch;
                state.items = [
                  {
                    id: "n1",
                    original: `${ROOT}/scan.pdf`,
                    destination: "",
                    folder: w.itemFolder || "",
                    title: "scan",
                    reason:
                      "No folder fits this letter. Choose a folder for it.",
                    status: "review",
                    copied: false,
                    confident: false,
                  },
                ];
                return 1;
              case "update_review": {
                const item = state.items.find((i: any) => i.id === args.id);
                item.folder = args.folder;
                item.destination = `${ROOT}/${args.folder}/${args.title}.pdf`;
                return;
              }
              case "organize_documents":
                for (const a of args.approvals)
                  state.items.find((i: any) => i.id === a.id).status =
                    "organized";
                return;
              case "skip_document":
                state.items.find((i: any) => i.id === args.id).status =
                  "skipped";
                return;
              case "stop_tuck":
                state.running = false;
                state.watching = false;
                return;
              case "clear_batch":
                state.items = [];
                state.running = false;
                return;
              case "check_model":
                return "Apple Intelligence is ready — on-device";
              case "check_connection":
                return "Connection ready. The selected model responded.";
            }
            throw new Error(`Unhandled test command ${command}`);
          },
        },
      });
      Object.defineProperty(window, "__TAURI_EVENT_PLUGIN_INTERNALS__", {
        value: { unregisterListener: () => {} },
      });
    },
    { initial, options, ROOT },
  );
  await page.goto("/");
}
const calls = (page: Page, command: string) =>
  page.evaluate(
    (c) => (window as any).calls.filter((x: any) => x.command === c),
    command,
  );

async function suggested(page: Page) {
  await bridge(page);
  await page.getByRole("button", { name: "Choose folder" }).click();
  await expect(
    page.getByRole("region", { name: "Suggested folders" }),
  ).toBeVisible();
}

test("a new folder is read and folders are suggested without extra steps", async ({
  page,
}) => {
  await bridge(page);
  await expect(
    page.getByRole("heading", { name: "A place for every document" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Choose folder" }).click();
  const list = page.getByRole("region", { name: "Suggested folders" });
  await expect(list.getByText("Payslips")).toBeVisible();
  await expect(list.getByText("5 files")).toBeVisible();
  // Finance counts its own two files plus the payslip in its subfolder.
  await expect(
    list.locator("li", { hasText: "Finance" }).first().locator(".count"),
  ).toHaveText("3");
  await expect(page.getByText("1 file will wait for you")).toBeVisible();
  expect((await calls(page, "map_folder")).length).toBe(1);
  expect((await calls(page, "save_preferences")).length).toBe(0);
});

test("folders can be renamed, removed and files moved before organizing", async ({
  page,
}) => {
  await suggested(page);
  await page
    .getByRole("button", { name: "Rename Finance", exact: true })
    .click();
  const name = page.getByRole("textbox", { name: "Name for Finance" });
  await name.fill("Money");
  await name.press("Enter");
  await expect(page.getByText("Money", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Show files in Home" }).click();
  await page
    .getByRole("combobox", { name: "Folder for lease.pdf" })
    .selectOption("Money");
  await page.getByRole("button", { name: "Remove Money/Payslips" }).click();
  await page
    .getByRole("checkbox", { name: "Keep watching for new files" })
    .check();
  await page.getByRole("button", { name: /^Organize 4 files$/ }).click();

  const saved = (await calls(page, "save_preferences"))[0].args.preferences;
  expect(saved.rules.map((r: any) => r.folder)).toEqual(["Money", "Home"]);
  const organized = (await calls(page, "organize_proposal"))[0].args;
  expect(organized.watch).toBe(true);
  expect(organized.files.map((f: any) => f.folder)).toEqual([
    "Money",
    "Money",
    "Money",
    "Money",
  ]);
  await expect(
    page.getByRole("heading", { name: "4 files organized" }),
  ).toBeVisible();
  await expect(page.getByText("Watching for new files.")).toBeVisible();
});

test("removing a top folder sends its files to the waiting list", async ({
  page,
}) => {
  await suggested(page);
  await page.getByRole("button", { name: "Remove Home", exact: true }).click();
  await expect(page.getByText("2 files will wait for you")).toBeVisible();
  await expect(
    page.getByRole("button", { name: /^Organize 3 files$/ }),
  ).toBeVisible();
});

test("files that need a decision are filed or skipped in place", async ({
  page,
}) => {
  await suggested(page);
  await page.getByRole("button", { name: /^Organize 4 files$/ }).click();
  await expect(page.getByText("1 file needs you.")).toBeVisible();
  await page.screenshot({ path: "test-results/results.png" });
  await page
    .getByRole("combobox", { name: "Folder for blank.png" })
    .selectOption("Home");
  await page.getByRole("button", { name: "File", exact: true }).click();
  expect((await calls(page, "update_review"))[0].args).toMatchObject({
    id: "blank",
    folder: "Home",
  });
  expect(
    (await calls(page, "organize_documents"))[0].args.approvals[0].id,
  ).toBe("blank");
  await expect(
    page.getByRole("heading", { name: "5 files organized" }),
  ).toBeVisible();
  await expect(page.getByText("All done.")).toBeVisible();
  await page.getByRole("button", { name: "Done" }).click();
  expect((await calls(page, "clear_batch")).length).toBe(1);
});

test("a folder with saved folders organizes new files and can suggest again", async ({
  page,
}) => {
  await bridge(page, { saved: true });
  const list = page.getByRole("region", { name: "Your folders" });
  await expect(list.getByText("Finance")).toBeVisible();
  await page.getByRole("button", { name: "Organize new files" }).click();
  expect((await calls(page, "start_tuck"))[0].args.watch).toBe(false);
  await expect(page.getByText("1 file needs you.")).toBeVisible();
  await page.getByRole("button", { name: "Skip" }).click();
  expect((await calls(page, "skip_document"))[0].args.id).toBe("n1");
});

test("a failed suggestion explains itself and can be retried", async ({
  page,
}) => {
  await bridge(page);
  await page.evaluate(() => ((window as any).failSuggest = true));
  await page.getByRole("button", { name: "Choose folder" }).click();
  await expect(page.getByRole("alert")).toContainText(
    "Apple Intelligence is turned off.",
  );
  await page.evaluate(() => ((window as any).failSuggest = false));
  await page.getByRole("button", { name: "Suggest folders" }).click();
  await expect(
    page.getByRole("region", { name: "Suggested folders" }),
  ).toBeVisible();
});

test("settings keep advanced options folded and save explicitly", async ({
  page,
}) => {
  await bridge(page, { saved: true });
  await page.getByRole("button", { name: "Settings" }).click();
  await expect(
    page.getByRole("textbox", { name: "Document instructions" }),
  ).toBeHidden();
  await page.getByText("One level", { exact: true }).click();
  await expect(page.getByRole("radio", { name: "One level" })).toBeChecked();
  await page.getByRole("switch").uncheck();
  await page.getByText("Advanced").click();
  await page.screenshot({ path: "test-results/settings.png" });
  await page
    .getByRole("textbox", { name: "Document instructions" })
    .fill("Short names.");
  await page.getByRole("button", { name: "Save" }).click();
  const saved = (await calls(page, "save_preferences"))[0].args.preferences;
  expect(saved).toMatchObject({
    folderNesting: 1,
    rename: false,
    instructions: "Short names.",
  });
  await expect(
    page.getByRole("region", { name: "Your folders" }),
  ).toBeVisible();
});

test("a custom connection keeps its API key out of preferences", async ({
  page,
}) => {
  await bridge(page, { saved: true });
  await page.getByRole("button", { name: "Settings" }).click();
  await page.getByText("Other", { exact: true }).click();
  await page.getByLabel("Model ID").fill("my-model");
  await page.getByLabel("API key").fill("secret-key");
  await page.getByRole("button", { name: "Save" }).click();
  const { preferences, apiKey } = (await calls(page, "save_preferences"))[0]
    .args;
  expect(apiKey).toBe("secret-key");
  expect(JSON.stringify(preferences)).not.toContain("secret-key");
  expect(preferences.connection.model).toBe("my-model");
});

test("layout fits narrow windows in light and dark mode", async ({ page }) => {
  await suggested(page);
  for (const colorScheme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme });
    for (const width of [900, 420]) {
      await page.setViewportSize({ width, height: 700 });
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= innerWidth,
        ),
      ).toBe(true);
      await page.screenshot({
        path: `test-results/home-${colorScheme}-${width}.png`,
      });
    }
  }
});

test("invalid or colliding folder names are refused and Escape cancels", async ({
  page,
}) => {
  await suggested(page);
  await page.getByRole("button", { name: "Rename Home", exact: true }).click();
  const name = page.getByRole("textbox", { name: "Name for Home" });
  for (const bad of ["..", ".hidden", "a:b", "finance"]) {
    await name.fill(bad);
    await expect(name).toHaveAttribute("aria-invalid", "true");
  }
  await name.press("Enter");
  await expect(name).toBeVisible();
  await name.fill("Housing");
  await name.press("Escape");
  await expect(page.getByText("Home", { exact: true })).toBeVisible();
  await expect(page.getByText("Housing")).toHaveCount(0);
  await expect(
    page.getByRole("region", { name: "Suggested folders" }),
  ).toBeVisible();
});

test("renaming a parent carries its subfolders, files and descriptions", async ({
  page,
}) => {
  await suggested(page);
  await page
    .getByRole("button", { name: "Rename Finance", exact: true })
    .click();
  await page.getByRole("textbox", { name: "Name for Finance" }).fill("Money");
  await page.getByRole("textbox", { name: "Name for Finance" }).press("Enter");
  await page.getByRole("button", { name: /^Organize 4 files$/ }).click();
  const { rules } = (await calls(page, "save_preferences"))[0].args.preferences;
  expect(rules.map((r: any) => r.folder)).toEqual([
    "Money",
    "Money/Payslips",
    "Home",
  ]);
  expect(rules[1].instruction).toBe("Documents such as payslip.");
  const { files } = (await calls(page, "organize_proposal"))[0].args;
  expect(files.map((f: any) => f.folder)).toEqual([
    "Money",
    "Money",
    "Money/Payslips",
    "Home",
  ]);
});

test("a subfolder without a parent folder sends its files to wait when removed", async ({
  page,
}) => {
  await bridge(page);
  await page.evaluate(() => {
    (window as any).proposalOverride = {
      rules: [
        { folder: "Work/Contracts", instruction: "Contracts.", enabled: true },
        { folder: "Home", instruction: "Rent.", enabled: true },
      ],
      files: [
        {
          path: "/Users/test/Documents/c.pdf",
          folder: "Work/Contracts",
          title: "Contract",
        },
        { path: "/Users/test/Documents/r.pdf", folder: "Home", title: "Rent" },
      ],
      reviewFiles: [],
    };
  });
  await page.getByRole("button", { name: "Choose folder" }).click();
  await page.getByRole("button", { name: "Remove Work/Contracts" }).click();
  await expect(page.getByText("1 file will wait for you")).toBeVisible();
  await page.getByRole("button", { name: /^Organize 1 file$/ }).click();
  const { files } = (await calls(page, "organize_proposal"))[0].args;
  expect(files).toEqual([
    { path: "/Users/test/Documents/r.pdf", folder: "Home", title: "Rent" },
  ]);
});

test("settings changed after suggesting are kept when organizing", async ({
  page,
}) => {
  await suggested(page);
  await page.getByRole("button", { name: "Settings" }).click();
  await page.getByRole("switch").uncheck();
  await expect(page.getByText("Unsaved changes")).toBeVisible();
  await page.getByRole("button", { name: "Save" }).click();
  await expect(
    page.getByRole("region", { name: "Suggested folders" }),
  ).toBeVisible();
  await page.getByRole("button", { name: /^Organize 4 files$/ }).click();
  const saves = await calls(page, "save_preferences");
  expect(saves.at(-1).args.preferences.rename).toBe(false);
  expect(saves.at(-1).args.preferences.rules).toHaveLength(3);
});

test("a waiting file with an unknown folder must have a folder chosen", async ({
  page,
}) => {
  await bridge(page, { saved: true });
  await page.evaluate(() => ((window as any).itemFolder = "Old/Removed"));
  await page.getByRole("button", { name: "Organize new files" }).click();
  await expect(
    page.getByRole("combobox", { name: "Folder for scan.pdf" }),
  ).toHaveValue("");
  await expect(
    page.getByRole("button", { name: "File", exact: true }),
  ).toBeDisabled();
});

test("each screen renders for visual review", async ({ page }) => {
  await page.setViewportSize({ width: 1000, height: 760 });
  await bridge(page);
  await page.screenshot({ path: "test-results/screen-1-hero.png" });
  await page.evaluate(() => {
    const w = window as any;
    w.holdSuggest = true;
    w.state.processing = "Reading document 12 of 35";
  });
  await page.getByRole("button", { name: "Choose folder" }).click();
  await expect(page.getByRole("status")).toContainText(
    "Reading document 12 of 35",
  );
  await page.screenshot({ path: "test-results/screen-2-working.png" });
  await page.evaluate(() => {
    const w = window as any;
    w.state.processing = null;
    w.releaseSuggest();
  });
  await expect(
    page.getByRole("region", { name: "Suggested folders" }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Show files in Finance", exact: true })
    .click();
  await page.screenshot({ path: "test-results/screen-3-proposal.png" });
  await page.getByRole("button", { name: /^Organize 4 files$/ }).click();
  await expect(page.getByText("1 file needs you.")).toBeVisible();
  await page.screenshot({ path: "test-results/screen-4-results.png" });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.screenshot({ path: "test-results/screen-5-results-dark.png" });
});

test("dark mode screens render for visual review", async ({ page }) => {
  await page.setViewportSize({ width: 1000, height: 760 });
  await page.emulateMedia({ colorScheme: "dark" });
  await bridge(page);
  await page.screenshot({ path: "test-results/screen-6-hero-dark.png" });
  await page.getByRole("button", { name: "Choose folder" }).click();
  await page
    .getByRole("button", { name: "Show files in Finance", exact: true })
    .click();
  await page.screenshot({ path: "test-results/screen-7-proposal-dark.png" });
});
