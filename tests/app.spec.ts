import { test, expect, type Page } from "@playwright/test";
import { initial as defaults } from "../src/types";
// Saved hierarchical settings remain supported independently of starter categories.
const initial = structuredClone(defaults);
initial.preferences.rules = [
  {
    enabled: true,
    folder: "Migration/AGENCY",
    instruction:
      "Only documents associated with AGENCY belong here. Look for AGENCY in the text, filename or source folder names. Keep its supporting attachments together.",
  },
  {
    enabled: true,
    folder: "Migration",
    instruction:
      "Keep my migration documents here, including residence permits, immigration applications and letters from Migrationsverket.",
  },
  {
    enabled: true,
    folder: "Finances",
    instruction:
      "Keep banking and tax documents here, including account statements and tax returns.",
  },
  {
    enabled: true,
    folder: "Work",
    instruction:
      "Keep employment documents here, including payslips and employment contracts.",
  },
  {
    enabled: true,
    folder: "Home",
    instruction:
      "Keep housing documents here, including rental agreements and household utility bills. Insurance policies belong in Insurance.",
  },
  {
    enabled: true,
    folder: "Insurance",
    instruction:
      "Keep insurance documents here, including policies and certificates of cover for my home, belongings or other risks.",
  },
  {
    enabled: true,
    folder: "Health",
    instruction:
      "Keep health documents here, including medical reports and prescriptions.",
  },
];
async function ruleField(page: Page, index: number) {
  const path = initial.preferences.rules[index - 1].folder;
  await page
    .getByRole("button", { name: `Rules for ${path}`, exact: true })
    .click();
  return page.getByRole("textbox", { name: `Rule for ${path}`, exact: true });
}
async function bridge(page: Page, welcome = false) {
  await page.addInitScript(
    ({ initial, welcome }) => {
      if (!welcome) localStorage.setItem("tuck.welcome.v2", "seen");
      Object.defineProperty(window, "isTauri", { value: true });
      const state = structuredClone(initial);
      (window as any).calls = [];
      (window as any).testState = state;
      let next = 1;
      window.addEventListener("test-disk-change", () => {
        state.preferences.rename = false;
        state.preferences.settingsRevision = "disk-edited";
      });
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
            (window as any).calls.push({ command, args });
            if (command === "plugin:event|listen") return next++;
            if (command === "plugin:event|unlisten") return;
            if (command === "plugin:dialog|open")
              return args.options.directory
                ? "/Users/test/Documents"
                : [
                    "/Users/test/Downloads/statement.txt",
                    "/Users/test/Downloads/scan.txt",
                  ];
            if (command === "choose_folder") {
              state.preferences.root = args.root;
              return;
            }
            if (command === "map_folder") {
              if ((window as any).delayMap)
                await new Promise((resolve) => {
                  (window as any).resumeMap = resolve;
                });
              if ((window as any).failMap)
                throw new Error("Could not read this folder.");
              state.collection = {
                documents: [],
                folders: [{ path: "Finances", documents: 0 }],
                existingRules: structuredClone(state.preferences.rules),
                unreadable: 0,
                duplicates: 0,
              };
              return;
            }
            if (command === "start_tuck") {
              state.running = true;
              state.watching = args.watch;
              return 0;
            }
            if (command === "stop_tuck") {
              state.running = false;
              state.watching = false;
              return;
            }
            if (command === "suggest_folder_rule")
              return "Keep immigration applications and residence permits here.";
            if (command === "check_folder_rules") {
              if ((window as any).delayRuleCheck)
                await new Promise((resolve) => setTimeout(resolve, 500));
              const rules = args.preferences.rules.filter(
                (r: any) => r.enabled,
              );
              const issues = [];
              for (let i = 0; i < rules.length; i++)
                for (let j = i + 1; j < rules.length; j++) {
                  if (
                    rules[i].instruction.toLowerCase() ===
                    rules[j].instruction.toLowerCase()
                  )
                    issues.push({
                      folder: rules[i].folder,
                      otherFolder: rules[j].folder,
                      issue: "These folders have the same description.",
                      clarification: "",
                    });
                }
              return issues;
            }
            if (command === "suggest_categories") {
              return {
                rules: [
                  {
                    folder: "Finances",
                    documentTypes: ["Bank statement"],
                    instruction: "Keep bank statements here.",
                    enabled: true,
                  },
                  {
                    folder: "Home",
                    instruction: "Keep rental agreements here.",
                    enabled: true,
                  },
                ],
                sampled: 3,
                skipped: 0,
                limited: false,
                preferences: structuredClone(state.preferences),
                ...((window as any).proposalOverride || {}),
              };
            }
            if (command === "save_preferences") {
              state.preferences = structuredClone(args.preferences);
              return;
            }
            if (
              command === "import_documents" ||
              command === "review_existing_documents"
            ) {
              const paths =
                command === "review_existing_documents"
                  ? [`${state.preferences.root}/statement.txt`]
                  : args.paths;
              state.items = paths.map((path: string, index: number) => ({
                id: `item-${index}`,
                original: path,
                destination:
                  index === 0
                    ? "/Users/test/Documents/Finances/Bank statement.txt"
                    : "",
                folder: index === 0 ? "Finances" : "",
                title: index === 0 ? "Bank statement" : "scan",
                reason:
                  index === 0
                    ? "Bank account statement"
                    : "Insufficient evidence. Choose a folder.",
                status: "review",
                copied: true,
                confident: index === 0,
              }));
              return state.items.length;
            }
            if (command === "update_review") {
              if ((window as any).failUpdate)
                throw new Error("Destination validation failed.");
              if ((window as any).changeRootOnUpdate)
                state.preferences.root = "/Users/test/Other";
              const i = state.items.find((i) => i.id === args.id)!;
              i.folder = args.folder;
              i.title = args.title;
              i.destination = `${state.preferences.root}/${i.folder}/${i.title}.txt`;
              return;
            }
            if (command === "organize_documents") {
              for (const a of args.approvals)
                state.items.find((i) => i.id === a.id)!.status = "organized";
              return;
            }
            if (command === "skip_document") {
              state.items.find((i) => i.id === args.id)!.status = "skipped";
              return;
            }
            if (command === "retry_document") {
              state.items.find((i) => i.id === args.id)!.status = "review";
              return;
            }
            if (command === "clear_batch") {
              state.running = false;
              state.watching = false;
              state.items = [];
              return;
            }
            if (command === "check_connection") {
              if ((window as any).failModelCheck)
                throw new Error("Open Ollama, then check again.");
              return args.listModels
                ? JSON.stringify(["my-local-model"])
                : "Connection ready. The selected model responded.";
            }
            if (command === "check_model") {
              if ((window as any).failModelCheck)
                throw "Open Ollama, then check again.";
              return `${args.model} is ready — on this Mac`;
            }
            if (command === "plugin:opener|reveal_item_in_dir") return;
            throw new Error(`Unhandled test command ${command}`);
          },
        },
      });
      Object.defineProperty(window, "__TAURI_EVENT_PLUGIN_INTERNALS__", {
        value: { unregisterListener: () => {} },
      });
    },
    { initial, welcome },
  );
}
async function start(page: Page) {
  await bridge(page);
  await page.goto("/");
  await page
    .getByRole("button", { name: "Choose folder", exact: true })
    .click();
  await page.getByRole("button", { name: "Start Tuck", exact: true }).click();
  await page.getByRole("button", { name: "Choose files", exact: true }).click();
  await expect(page.getByRole("article")).toHaveCount(2);
}
test("fallback suggestions can still be organized individually", async ({
  page,
}) => {
  await start(page);
  await expect(
    page.getByRole("button", { name: "Organize selected (0)" }),
  ).toHaveCount(0);
  expect(
    await page.evaluate(() =>
      (window as any).calls.some(
        (c: any) => c.command === "organize_documents",
      ),
    ),
  ).toBe(false);
  await page.getByRole("checkbox", { name: "Select statement.txt" }).check();
  await page.getByRole("button", { name: "Organize selected (1)" }).click();
  await expect(page.getByRole("article")).toHaveCount(1);
  await expect(
    page
      .getByRole("article")
      .getByText("Where should this go?", { exact: true }),
  ).toBeVisible();
  await page.getByRole("tab", { name: "Activity" }).click();
  await expect(
    page.locator(".activity-row").getByText("organized", { exact: true }),
  ).toBeVisible();
  const call = await page.evaluate(() =>
    (window as any).calls.find((c: any) => c.command === "organize_documents"),
  );
  expect(call.args.approvals).toEqual([
    {
      id: "item-0",
      destination: "/Users/test/Documents/Finances/Bank statement.txt",
    },
  ]);
});
test("an uncertain file is validated and filed with one action", async ({
  page,
}) => {
  await start(page);
  const card = page.getByRole("article").nth(1);
  await expect(
    card.getByRole("button", { name: "File document" }),
  ).toBeDisabled();
  await card.getByLabel("Category for scan.txt", { exact: true }).click();
  await card
    .getByRole("button", { name: "Migration/AGENCY", exact: true })
    .click();
  await card.getByText("More options", { exact: true }).click();
  await card.getByRole("textbox").fill("Residence permit card");
  await expect(
    card.getByRole("button", { name: "Update preview" }),
  ).toHaveCount(0);
  await card.getByRole("button", { name: "File document" }).click();
  await expect(page.getByRole("article")).toHaveCount(1);
  const calls = await page.evaluate(() =>
    (window as any).calls.filter((c: any) =>
      ["update_review", "organize_documents"].includes(c.command),
    ),
  );
  expect(calls).toEqual([
    {
      command: "update_review",
      args: {
        id: "item-1",
        folder: "Migration/AGENCY",
        title: "Residence permit card",
      },
    },
    {
      command: "organize_documents",
      args: {
        approvals: [
          {
            id: "item-1",
            destination:
              "/Users/test/Documents/Migration/AGENCY/Residence permit card.txt",
          },
        ],
      },
    },
  ]);
});
test("editing clears bulk selection but can be filed directly", async ({
  page,
}) => {
  await start(page);
  const card = page.getByRole("article").first();
  await card.getByRole("checkbox").check();
  await card.getByText("More options", { exact: true }).click();
  await card.getByRole("textbox").fill("Edited name");
  await expect(card.getByRole("checkbox")).toHaveCount(0);
  await expect(
    card.getByRole("button", { name: "File document" }),
  ).toBeEnabled();
  await expect(
    page.getByRole("button", { name: /Organize selected/ }),
  ).toHaveCount(0);
});
for (const failure of ["failUpdate", "changeRootOnUpdate"]) {
  test(`filing stops when ${failure}`, async ({ page }) => {
    await start(page);
    const card = page.getByRole("article").nth(1);
    await card.getByLabel("Category for scan.txt", { exact: true }).click();
    await card.getByRole("button", { name: "Migration", exact: true }).click();
    await page.evaluate((flag) => {
      (window as any)[flag] = true;
    }, failure);
    await card.getByRole("button", { name: "File document" }).click();
    await expect(
      page.getByText(
        failure === "failUpdate"
          ? /Destination validation failed/
          : /document or folder settings changed/,
      ),
    ).toBeVisible();
    expect(
      await page.evaluate(() =>
        (window as any).calls.some(
          (c: any) => c.command === "organize_documents",
        ),
      ),
    ).toBe(false);
  });
}
test("category picker closes with Escape and outside clicks", async ({
  page,
}) => {
  await start(page);
  const card = page.getByRole("article").nth(1);
  const trigger = card.getByLabel("Category for scan.txt", { exact: true });
  await trigger.focus();
  await page.keyboard.press("Enter");
  await expect(
    card.getByRole("button", { name: "Finances", exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(trigger).toBeFocused();
  await expect(
    card.getByRole("button", { name: "Finances", exact: true }),
  ).toBeHidden();
  await trigger.click();
  await page.locator(".topbar").click();
  await expect(
    card.getByRole("button", { name: "Finances", exact: true }),
  ).toBeHidden();
});
test("skip and clear never invoke organize and unlock destination picker", async ({
  page,
}) => {
  await start(page);
  await expect(
    page.getByRole("button", { name: "Change folder" }),
  ).toBeDisabled();
  await page
    .getByRole("article")
    .first()
    .getByRole("button", { name: "Leave for now", exact: true })
    .click();
  await page.getByRole("tab", { name: "Activity" }).click();
  await expect(
    page.locator(".activity-row").getByText("skipped", { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Clear batch" }).click();
  await expect(page.getByRole("article")).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Change folder" }),
  ).toBeEnabled();
  expect(
    await page.evaluate(() =>
      (window as any).calls.some(
        (c: any) => c.command === "organize_documents",
      ),
    ),
  ).toBe(false);
});
test("folder prompts and switches remain editable", async ({ page }) => {
  await start(page);
  await page.getByRole("button", { name: "Folders", exact: true }).click();
  await (
    await ruleField(page, 1)
  ).fill("AGENCY application documents belong here.");
  await page
    .getByRole("button", { name: "Rules for Migration", exact: true })
    .click();
  await page
    .getByRole("checkbox", { name: "Enable category Migration", exact: true })
    .click();
  await page.getByRole("button", { name: "Save rules", exact: true }).click();
  await expect(
    page.getByRole("checkbox", {
      name: "Enable category Migration",
      exact: true,
    }),
  ).not.toBeChecked();
  await page
    .locator("nav")
    .getByRole("button", { name: /^Organize/ })
    .click();
  const card = page.getByRole("article").nth(1);
  await card.getByLabel("Category for scan.txt", { exact: true }).click();
  await expect(
    card.getByRole("button", {
      name: "Migration/AGENCY (disabled)",
      exact: true,
    }),
  ).toBeDisabled();
});
test("disk preferences refresh without replacing unsaved edits", async ({
  page,
}) => {
  await start(page);
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByLabel("Document instructions").fill("My unsaved prompt");
  await page.evaluate(() =>
    window.dispatchEvent(new Event("test-disk-change")),
  );
  await expect
    .poll(() =>
      page.evaluate(
        () => (window as any).testState.preferences.settingsRevision,
      ),
    )
    .toBe("disk-edited");
  await expect(page.getByLabel("Document instructions")).toHaveValue(
    "My unsaved prompt",
  );
  await page
    .getByRole("button", { name: "Discard changes", exact: true })
    .first()
    .click();
  await expect(
    page.getByRole("checkbox", { name: "Rename documents" }),
  ).toHaveCount(0);
  await expect(page.getByLabel("Document instructions")).not.toHaveValue(
    "My unsaved prompt",
  );
});
test("introduction explains automatic filing and session lifetime", async ({
  page,
}) => {
  await bridge(page, true);
  await page.goto("/");
  await expect(page.getByRole("dialog")).toBeVisible();
  await expect(
    page.getByText("Help only when needed", { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Next", exact: true }).click();
  await page.getByRole("button", { name: "Next", exact: true }).click();
  await expect(
    page.getByText("One session at a time", { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Start using Tuck" }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
});
test("content scrolls while the sidebar and header stay fixed", async ({
  page,
}) => {
  await start(page);
  await page.evaluate(() => {
    const s = (window as any).testState;
    s.items = Array.from({ length: 15 }, (_, i) => ({
      ...s.items[0],
      id: `long-${i}`,
    }));
  });
  await expect(page.getByRole("article")).toHaveCount(15);
  const before = await page.locator(".topbar").boundingBox();
  await page.locator(".content-scroll").evaluate((el) => {
    el.scrollTop = el.scrollHeight;
  });
  expect(await page.locator(".topbar").boundingBox()).toEqual(before);
  expect(await page.evaluate(() => document.documentElement.scrollTop)).toBe(0);
  await page.screenshot({
    animations: "disabled",
    path: "test-results/review-scroll.png",
  });
});
test("review UI screenshot", async ({ page }) => {
  await start(page);
  await page.screenshot({
    animations: "disabled",
    path: "test-results/review-mvp.png",
  });
  await page.setViewportSize({ width: 900, height: 640 });
  const card = page.getByRole("article").nth(1);
  await card.getByLabel("Category for scan.txt", { exact: true }).click();
  await expect(
    card.getByRole("button", { name: "Migration/AGENCY", exact: true }),
  ).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await expect(
    card.getByRole("button", { name: "Health", exact: true }),
  ).toBeInViewport();
  await page.screenshot({
    animations: "disabled",
    path: "test-results/review-narrow.png",
  });
});

test("privacy is visible and stale language suggestions are hidden", async ({
  page,
}) => {
  await start(page);
  await expect(page.locator(".topbar")).toContainText("On-device AI");
  await expect(
    page.getByText("You approve every file", { exact: true }),
  ).toHaveCount(0);
  await expect(page.locator(".sidebar-bottom .local-card")).toHaveCount(0);
  await page.evaluate(() => {
    (window as any).testState.items[1].language = {
      name: "Polish",
      code: "pl",
      status: "download",
      message:
        "Download Polish and English in System Settings → General → Language & Region → Translation Languages, then choose Analyze again.",
    };
  });
  await page
    .getByRole("article")
    .nth(1)
    .getByText("More options", { exact: true })
    .click();
  await expect(
    page.getByText("Polish · Language download available", { exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByText(/Download Polish and English in System Settings/),
  ).toHaveCount(0);
  await expect(
    page
      .getByRole("article")
      .nth(1)
      .getByRole("button", { name: "File document", exact: true }),
  ).toBeDisabled();
  await page.screenshot({
    animations: "disabled",
    path: "test-results/language-support.png",
  });
});

async function emptyFolder(page: Page) {
  await bridge(page);
  await page.goto("/");
  await page.evaluate(() => {
    (window as any).testState.preferences.rules = [];
  });
  await page
    .getByRole("button", { name: "Choose folder", exact: true })
    .click();
}
test("category suggestions are editable and create folders only on approval", async ({
  page,
}) => {
  await emptyFolder(page);
  await page
    .getByRole("button", { name: "Suggest categories", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Your suggested categories" }),
  ).toBeVisible();
  expect(
    await page.evaluate(() =>
      (window as any).calls.some((c: any) => c.command === "save_preferences"),
    ),
  ).toBe(false);
  await expect(
    page.getByText("Document types (1)", { exact: true }),
  ).toBeVisible();
  await page.getByLabel("Suggested folder 1").fill("Money");
  await page
    .getByLabel("Suggested rule 1")
    .fill("Keep my financial paperwork here.");
  await page
    .getByRole("button", { name: "Rules for Home", exact: true })
    .click();
  await page.getByRole("button", { name: "Remove suggestion 2" }).click();
  await page.screenshot({
    animations: "disabled",
    path: "/private/tmp/tuck-category-preview.png",
  });
  await page
    .getByRole("button", { name: "Use these categories", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Your suggested categories" }),
  ).toHaveCount(0);
  const calls = await page.evaluate(() => (window as any).calls);
  expect(
    calls.find((c: any) => c.command === "save_preferences").args.preferences
      .rules,
  ).toEqual([
    {
      folder: "Money",
      documentTypes: [],
      instruction: "Keep my financial paperwork here.",
      enabled: true,
    },
  ]);
  expect(calls.some((c: any) => c.command === "organize_documents")).toBe(
    false,
  );
});
test("discarding suggested categories leaves configuration untouched", async ({
  page,
}) => {
  await emptyFolder(page);
  await page
    .getByRole("button", { name: "Suggest categories", exact: true })
    .click();
  await page.getByRole("button", { name: "Discard suggestions" }).click();
  await expect(
    page.getByRole("button", { name: "Suggest categories", exact: true }),
  ).toBeVisible();
  expect(
    await page.evaluate(() =>
      (window as any).calls.some((c: any) => c.command === "save_preferences"),
    ),
  ).toBe(false);
});

test("folder selection maps first and waits for Start Tuck with optional watching", async ({
  page,
}) => {
  await bridge(page);
  await page.goto("/");
  await page
    .getByRole("button", { name: "Choose folder", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "7 folders, ready to use", exact: true }),
  ).toBeVisible();
  await expect(page.locator(".category-overview")).toContainText("Finances");
  await expect(
    page.getByText("0 documents · 0 duplicate copies · 0 unreadable"),
  ).not.toBeVisible();
  await expect(
    page.getByRole("button", { name: "Edit folders", exact: true }),
  ).toBeVisible();
  const before = await page.evaluate(() => (window as any).calls);
  expect(before.some((c: any) => c.command === "map_folder")).toBe(true);
  expect(before.some((c: any) => c.command === "start_tuck")).toBe(false);
  await page
    .getByRole("checkbox", { name: "Keep watching this folder" })
    .check();
  await page.screenshot({
    animations: "disabled",
    path: "/private/tmp/tuck-map-setup.png",
  });
  await page.getByRole("button", { name: "Start Tuck", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Watching for new documents." }),
  ).toBeVisible();
  const after = await page.evaluate(() => (window as any).calls);
  expect(after.find((c: any) => c.command === "start_tuck").args).toEqual({
    watch: true,
  });
  await page.getByRole("button", { name: "Stop Tuck" }).click();
  await expect(
    page.getByRole("heading", {
      name: "7 folders, ready to use",
      exact: true,
    }),
  ).toBeVisible();
});

test("AI folder descriptions remain editable drafts until Save rules", async ({
  page,
}) => {
  await emptyFolder(page);
  await page.getByRole("button", { name: "Folders", exact: true }).click();
  await page.getByRole("button", { name: "Add folder", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Suggest description" }),
  ).toBeDisabled();
  await page
    .getByRole("textbox", { name: "Category path", exact: true })
    .fill("Migration");
  await page.getByRole("button", { name: "Suggest description" }).click();
  await expect(
    page.getByRole("textbox", { name: "Rule for Migration", exact: true }),
  ).toHaveValue("Keep immigration applications and residence permits here.");
  expect(
    await page.evaluate(() =>
      (window as any).calls.some((c: any) => c.command === "save_preferences"),
    ),
  ).toBe(false);
  await page
    .getByRole("textbox", { name: "Rule for Migration", exact: true })
    .fill("Keep my migration documents and AGENCY attachments here.");
  await page.screenshot({
    animations: "disabled",
    path: "/private/tmp/tuck-rule-draft.png",
  });
  await page.getByRole("button", { name: "Save rules", exact: true }).click();
  const saved = await page.evaluate(() =>
    (window as any).calls.find((c: any) => c.command === "save_preferences"),
  );
  expect(saved.args.preferences.rules[0].instruction).toContain(
    "AGENCY attachments",
  );
});

test("remembered examples are editable rule drafts and saving is explicit", async ({
  page,
}) => {
  await start(page);
  const card = page.getByRole("article").first();
  await expect(
    card.getByRole("button", { name: "Remember this example" }),
  ).toHaveCount(0);
  await card.getByLabel("Category for statement.txt", { exact: true }).click();
  await card
    .getByRole("button", { name: "Migration/AGENCY", exact: true })
    .click();

  await card.getByText("More options", { exact: true }).click();
  await card.getByRole("button", { name: "Remember this example" }).click();
  await page
    .getByRole("textbox", { name: "Example that belongs here", exact: true })
    .fill("A bank statement included in a AGENCY application.");
  await page
    .getByRole("textbox", {
      name: "Example that does not belong here",
      exact: true,
    })
    .fill("An unrelated monthly bank statement.");
  await page.screenshot({
    animations: "disabled",
    path: "/private/tmp/tuck-example-draft.png",
  });
  await page.getByRole("button", { name: "Add to rule draft" }).click();
  await expect(await ruleField(page, 1)).toHaveValue(/## Filing example/);
  expect(
    await page.evaluate(() =>
      (window as any).calls.some(
        (c: any) =>
          c.command === "save_preferences" ||
          c.command === "organize_documents",
      ),
    ),
  ).toBe(false);
  await page.getByRole("button", { name: "Save rules", exact: true }).click();
  const saved = await page.evaluate(() =>
    (window as any).calls.find((c: any) => c.command === "save_preferences"),
  );
  expect(saved.args.preferences.rules[0].instruction).toContain(
    "Does not belong here: An unrelated monthly bank statement.",
  );
  expect(saved.args.preferences.rules[0].instruction).not.toContain(
    "/Users/test",
  );
});

test("canceling an example and dismissing a rule check do not save", async ({
  page,
}) => {
  await start(page);
  const card = page.getByRole("article").first();
  await card.getByText("More options", { exact: true }).click();
  await card.getByRole("textbox").fill("Monthly Statement");

  await card.getByRole("button", { name: "Remember this example" }).click();
  await page.getByRole("button", { name: "Cancel example" }).click();
  await expect(
    page.getByRole("region", { name: "Remember a filing example" }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Folders", exact: true }).click();
  await page.getByRole("button", { name: "Check rules", exact: true }).click();
  await expect(
    page.getByRole("region", { name: "Rule check results" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Dismiss check" }).click();
  expect(
    await page.evaluate(() =>
      (window as any).calls.some((c: any) => c.command === "save_preferences"),
    ),
  ).toBe(false);
});

test("rule checks add editable clarifications and invalidate stale suggestions", async ({
  page,
}) => {
  await start(page);
  await page.getByRole("button", { name: "Folders", exact: true }).click();
  await (await ruleField(page, 5)).fill("Keep insurance documents here.");
  await (await ruleField(page, 6)).fill("Keep insurance documents here.");
  await page.getByRole("button", { name: "Check rules", exact: true }).click();
  await page
    .getByRole("textbox", { name: "Clarification 1" })
    .fill("Put home insurance policies in Insurance.");
  await page.screenshot({
    animations: "disabled",
    path: "/private/tmp/tuck-rule-check.png",
  });
  await page
    .getByRole("button", { name: "Add clarification to draft" })
    .click();
  await expect(await ruleField(page, 5)).toHaveValue(
    /Put home insurance policies in Insurance/,
  );
  await expect(
    page.getByRole("button", { name: "Add clarification to draft" }),
  ).toBeDisabled();
  expect(
    await page.evaluate(() =>
      (window as any).calls.some((c: any) => c.command === "save_preferences"),
    ),
  ).toBe(false);
  await page.getByRole("button", { name: "Save rules", exact: true }).click();
  expect(
    await page.evaluate(
      () =>
        (window as any).calls.filter(
          (c: any) => c.command === "save_preferences",
        ).length,
    ),
  ).toBe(1);
});

test("rule editing waits for an active check and later edits invalidate its result", async ({
  page,
}) => {
  await start(page);
  await page.getByRole("button", { name: "Folders", exact: true }).click();
  await (await ruleField(page, 5)).fill("Keep insurance documents here.");
  await (await ruleField(page, 6)).fill("Keep insurance documents here.");
  await page.evaluate(() => {
    (window as any).delayRuleCheck = true;
  });
  await page
    .getByRole("button", { name: "Rules for Home", exact: true })
    .click();
  await page.getByRole("button", { name: "Check rules", exact: true }).click();
  await expect(
    page.getByRole("textbox", { name: "Rule for Home", exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByRole("region", { name: "Rule check results" }),
  ).toBeVisible();
  await (await ruleField(page, 5)).fill("My updated housing rule.");
  await expect(
    page.getByRole("button", { name: "Add clarification to draft" }),
  ).toBeDisabled();
  await expect(
    page.getByRole("region", { name: "Rule check results" }),
  ).toContainText("Your rules changed");
});

test("Overview starts with one clear folder action", async ({ page }) => {
  await bridge(page);
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Choose folder", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Start Tuck", exact: true }),
  ).toHaveCount(0);
  await expect(page.getByRole("tablist")).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Choose files", exact: true }),
  ).toHaveCount(0);
  await page.screenshot({
    animations: "disabled",
    path: "/private/tmp/tuck-clean-empty.png",
  });
});

test("a saved folder starts with one click and maps before filing", async ({
  page,
}) => {
  await bridge(page);
  await page.goto("/");
  await page.evaluate(() => {
    (window as any).testState.preferences.root = "/Users/test/Documents";
  });
  const start = page.getByRole("button", { name: "Start Tuck", exact: true });
  await expect(start).toBeEnabled();
  expect(
    await page.evaluate(() =>
      (window as any).calls.some((c: any) => c.command === "map_folder"),
    ),
  ).toBe(false);
  await page.screenshot({
    animations: "disabled",
    path: "/private/tmp/tuck-clean-setup.png",
  });
  await start.click();
  const calls = await page.evaluate(() =>
    (window as any).calls.map((c: any) => c.command),
  );
  expect(calls.indexOf("map_folder")).toBeLessThan(calls.indexOf("start_tuck"));
  await expect(page.getByRole("button", { name: "Stop Tuck" })).toBeVisible();
});

test("successful documents stay in activity while exceptions stay actionable", async ({
  page,
}) => {
  await start(page);
  await page.evaluate(() => {
    const s = (window as any).testState;
    s.items[0].status = "organized";
  });
  await expect(page.getByRole("article")).toHaveCount(1);
  await expect(page.getByRole("article")).toContainText("scan.txt");
  await expect(
    page.getByRole("tab", { name: "Needs attention" }),
  ).toHaveAttribute("aria-selected", "true");
  await expect(
    page.getByRole("button", { name: "Show in Finder" }),
  ).toHaveCount(0);
  await page.screenshot({
    animations: "disabled",
    path: "/private/tmp/tuck-clean-attention.png",
  });
  await page.getByRole("tab", { name: "Activity" }).click();
  await expect(page.getByRole("article")).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Show in Finder" }),
  ).toBeVisible();
  await page.screenshot({
    animations: "disabled",
    path: "/private/tmp/tuck-clean-activity.png",
  });
});

test("a failed startup read never starts automatic filing", async ({
  page,
}) => {
  await bridge(page);
  await page.goto("/");
  await page.evaluate(() => {
    (window as any).testState.preferences.root = "/Users/test/Documents";
    (window as any).failMap = true;
  });
  await page.getByRole("button", { name: "Start Tuck", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText(
    "Could not read this folder",
  );
  expect(
    await page.evaluate(() =>
      (window as any).calls.some((c: any) => c.command === "start_tuck"),
    ),
  ).toBe(false);
});

test("category edits wait for startup reading", async ({ page }) => {
  await bridge(page);
  await page.goto("/");
  await page.evaluate(() => {
    (window as any).testState.preferences.root = "/Users/test/Documents";
    (window as any).delayMap = true;
  });
  await page.getByRole("button", { name: "Start Tuck", exact: true }).click();
  await expect
    .poll(() => page.evaluate(() => typeof (window as any).resumeMap))
    .toBe("function");
  await page.getByRole("button", { name: "Folders", exact: true }).click();
  await page
    .getByRole("button", { name: "Rules for Migration", exact: true })
    .click();
  await expect(
    page.getByRole("checkbox", {
      name: "Enable category Migration",
      exact: true,
    }),
  ).toBeDisabled();
  await page.evaluate(() => (window as any).resumeMap());
  await expect
    .poll(() =>
      page.evaluate(() =>
        (window as any).calls.some((c: any) => c.command === "start_tuck"),
      ),
    )
    .toBe(true);
});

async function setupTree(page: Page) {
  await bridge(page);
  await page.goto("/");
  await page
    .getByRole("button", { name: "Choose folder", exact: true })
    .click();
  await page.getByRole("button", { name: "Folders", exact: true }).click();
  return page.getByRole("region", { name: "Folder tree", exact: true });
}
test("Overview tree edits nested rules without changing precedence or filing", async ({
  page,
}) => {
  const tree = await setupTree(page);
  await tree
    .getByRole("button", { name: "Collapse Migration", exact: true })
    .click();
  await expect(
    tree.getByRole("button", {
      name: "Rules for Migration/AGENCY",
      exact: true,
    }),
  ).toHaveCount(0);
  await tree
    .getByRole("button", { name: "Expand Migration", exact: true })
    .click();
  await tree
    .getByRole("button", { name: "Rules for Migration/AGENCY", exact: true })
    .click();
  await tree
    .getByRole("textbox", { name: "Rule for Migration/AGENCY", exact: true })
    .fill("All documents in my AGENCY migration application belong here.");
  await expect(
    page.getByRole("button", { name: "Save rules", exact: true }),
  ).toBeEnabled();
  expect(
    await page.evaluate(() =>
      (window as any).calls.some((c: any) => c.command === "save_preferences"),
    ),
  ).toBe(false);
  await page.screenshot({
    animations: "disabled",
    path: "/private/tmp/tuck-folder-tree.png",
  });
  await page.setViewportSize({ width: 900, height: 650 });
  await tree.scrollIntoViewIfNeeded();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    animations: "disabled",
    path: "/private/tmp/tuck-folder-tree-narrow.png",
  });
  await page.getByRole("button", { name: "Save rules", exact: true }).click();
  const calls = await page.evaluate(() => (window as any).calls);
  const rules = calls.find((c: any) => c.command === "save_preferences").args
    .preferences.rules;
  expect(rules.map((r: any) => r.folder)).toEqual(
    initial.preferences.rules.map((r) => r.folder),
  );
  expect(rules[0].instruction).toContain("my AGENCY migration");
  expect(
    calls.some((c: any) =>
      ["start_tuck", "organize_documents"].includes(c.command),
    ),
  ).toBe(false);
});
test("tree shows inherited disabled state and can discard rule changes", async ({
  page,
}) => {
  const tree = await setupTree(page);
  await tree
    .getByRole("button", { name: "Rules for Migration", exact: true })
    .click();
  await tree
    .getByRole("checkbox", { name: "Enable category Migration", exact: true })
    .uncheck();
  await expect(
    tree.getByRole("button", {
      name: "Rules for Migration/AGENCY",
      exact: true,
    }),
  ).toContainText("Off");
  await tree
    .getByRole("button", { name: "Rules for Migration/AGENCY", exact: true })
    .click();
  await expect(
    tree.getByText("Turn on Migration to use this child folder."),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Discard changes", exact: true })
    .click();
  await expect(
    tree.getByRole("button", {
      name: "Rules for Migration/AGENCY",
      exact: true,
    }),
  ).not.toContainText("Off");
  expect(
    await page.evaluate(() =>
      (window as any).calls.some((c: any) => c.command === "save_preferences"),
    ),
  ).toBe(false);
});
test("root rules and new categories are editable directly in Overview", async ({
  page,
}) => {
  const tree = await setupTree(page);
  await tree
    .getByRole("button", { name: "Rules for all documents", exact: true })
    .click();
  await tree
    .getByRole("textbox", {
      name: "Document instructions",
      exact: true,
    })
    .fill("Keep names short and in English.");
  await tree.getByRole("button", { name: "Add folder", exact: true }).click();
  await tree
    .getByRole("textbox", { name: "Category path", exact: true })
    .fill("Travel/Tickets");
  await tree
    .getByRole("textbox", { name: "Rule for Travel/Tickets", exact: true })
    .fill("Keep flight and train tickets here.");
  await expect(
    tree.getByRole("button", { name: "Rules for Travel", exact: true }),
  ).toBeVisible();
  await tree
    .getByRole("button", { name: "Rules for Travel", exact: true })
    .click();
  await tree
    .getByRole("button", { name: "Add a rule here", exact: true })
    .click();
  await tree
    .getByRole("textbox", { name: "Rule for Travel", exact: true })
    .fill("Keep travel plans here.");
  await page.getByRole("button", { name: "Save rules", exact: true }).click();
  const saved = await page.evaluate(
    () =>
      (window as any).calls.find((c: any) => c.command === "save_preferences")
        .args.preferences,
  );
  expect(saved.instructions).toBe("Keep names short and in English.");
  expect(saved.rules.slice(-2).map((r: any) => r.folder)).toEqual([
    "Travel/Tickets",
    "Travel",
  ]);
});
test("running Overview exposes the same folder tree on demand", async ({
  page,
}) => {
  await start(page);
  await expect(
    page.getByRole("region", { name: "Folder tree", exact: true }),
  ).toBeHidden();
  await page
    .getByRole("button", { name: "Manage folders", exact: true })
    .click();
  const tree = page.getByRole("region", { name: "Folder tree", exact: true });
  await tree
    .getByRole("button", { name: "Rules for Finances", exact: true })
    .click();
  await tree
    .getByRole("textbox", { name: "Rule for Finances", exact: true })
    .fill("Bank and tax documents.");
  await expect(
    page.getByText("Saving rules pauses Tuck.", { exact: true }),
  ).toBeVisible();
});

test("document name opens native preview without filing", async ({ page }) => {
  await start(page);
  await page.getByRole("button", { name: "Choose files", exact: true }).click();
  await page
    .getByRole("button", { name: "Preview statement.txt", exact: true })
    .click();
  const calls = await page.evaluate(() => (window as any).calls);
  expect(
    calls.filter((c: any) => c.command === "preview_document").at(-1).args,
  ).toEqual({ id: "item-0" });
  expect(calls.some((c: any) => c.command === "organize_documents")).toBe(
    false,
  );
});

test("custom connection uses any model ID and keeps its API key out of preferences", async ({
  page,
}) => {
  await start(page);
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await expect(
    page.getByRole("radio", { name: /Apple Intelligence/ }),
  ).toBeChecked();
  await page
    .getByRole("button", { name: "Add connection", exact: true })
    .click();
  await page.getByLabel("Connection type").selectOption("openai");
  await page.getByLabel("API base URL").fill("https://example.test/v1");
  await page
    .getByLabel("Model ID", { exact: true })
    .fill("arbitrary-text-model");
  await page
    .getByLabel("API key", { exact: true })
    .fill("test-key-never-persist-in-json");
  expect(
    await page.evaluate(() => (window as any).testState.preferences.aiModel),
  ).toBe("apple");
  await page
    .getByRole("button", { name: "Test connection", exact: true })
    .click();
  await expect(
    page.getByText("Connection ready. The selected model responded."),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Save settings", exact: true })
    .click();
  const args = await page.evaluate(
    () =>
      (window as any).calls
        .filter((c: any) => c.command === "save_preferences")
        .at(-1).args,
  );
  expect(args.preferences.aiModel).toBe("custom");
  expect(args.preferences.connection.model).toBe("arbitrary-text-model");
  expect(JSON.stringify(args.preferences)).not.toContain("test-key");
  expect(args.apiKey).toBe("test-key-never-persist-in-json");
  await expect(page.getByLabel("API key", { exact: true })).toHaveValue("");
  await page.getByRole("radio", { name: /Apple Intelligence/ }).check();
  await page
    .getByRole("button", { name: "Discard changes", exact: true })
    .click();
  await expect(
    page.locator(".custom-provider").getByRole("radio"),
  ).toBeChecked();
});
test("unavailable custom provider stays selected without changing saved settings", async ({
  page,
}) => {
  await start(page);
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page
    .getByRole("button", { name: "Add connection", exact: true })
    .click();
  await page.getByLabel("Model ID", { exact: true }).fill("my-local-model");
  await page.evaluate(() => {
    (window as any).failModelCheck = true;
  });
  await page
    .getByRole("button", { name: "Test connection", exact: true })
    .click();
  await expect(page.getByText("Open Ollama, then check again.")).toBeVisible();
  await expect(
    page.locator(".custom-provider").getByRole("radio"),
  ).toBeChecked();
  expect(
    await page.evaluate(() => (window as any).testState.preferences.aiModel),
  ).toBe("apple");
});
test("settings own nesting and visible instructions, with no naming toggle", async ({
  page,
}) => {
  await setupTree(page);
  await expect(
    page.getByRole("radiogroup", { name: "Folder nesting" }),
  ).toHaveCount(0);
  await page.keyboard.press("Meta+,");
  await expect(
    page.getByRole("heading", { name: "Settings", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("checkbox", { name: "Rename documents" }),
  ).toHaveCount(0);
  await expect(
    page.getByText("Advanced instructions", { exact: true }),
  ).toHaveCount(0);
  await page.getByRole("radio", { name: "One level", exact: true }).check();
  await expect(
    page.getByRole("radio", { name: "Up to 3 levels", exact: true }),
  ).toHaveCount(0);
  await page
    .getByLabel("Folder creation preferences")
    .fill("Keep income from each employer separate.");
  expect(
    await page.evaluate(
      () => (window as any).testState.preferences.folderNesting,
    ),
  ).toBe(2);
  await page
    .getByRole("button", { name: "Save settings", exact: true })
    .click();
  expect(
    await page.evaluate(
      () => (window as any).testState.preferences.folderNesting,
    ),
  ).toBe(1);
  expect(
    await page.evaluate(() => (window as any).testState.preferences.rename),
  ).toBe(true);
});
test("nesting is an explicit draft and lowering it preserves existing folder rules", async ({
  page,
}) => {
  await setupTree(page);
  await page.keyboard.press("Meta+,");
  await page.getByRole("radio", { name: "One level", exact: true }).check();
  await expect(
    page.getByRole("radio", { name: "One level", exact: true }),
  ).toBeChecked();
  await expect(
    page.getByText("Fewer levels are always allowed.", { exact: false }),
  ).toBeVisible();
  expect(
    await page.evaluate(
      () => (window as any).testState.preferences.folderNesting,
    ),
  ).toBe(2);
  await page
    .getByRole("button", { name: "Discard changes", exact: true })
    .click();
  await expect(
    page.getByRole("radio", { name: "Up to 2 levels", exact: true }),
  ).toBeChecked();
  await page.getByRole("radio", { name: "One level", exact: true }).check();
  await page
    .getByRole("button", { name: "Save settings", exact: true })
    .click();
  expect(
    await page.evaluate(
      () => (window as any).testState.preferences.folderNesting,
    ),
  ).toBe(1);
  expect(
    await page.evaluate(
      () => (window as any).testState.preferences.rules[0].folder,
    ),
  ).toBe("Migration/AGENCY");
});
test("macOS layout is clear at standard and minimum sizes and respects reduced motion", async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await setupTree(page);
  await page.keyboard.press("Meta+1");
  await expect(
    page.getByRole("heading", { name: "Organize", exact: true }),
  ).toBeVisible();
  await page.screenshot({
    animations: "disabled",
    path: "/private/tmp/tuck-fresh-organize.png",
  });
  await page.keyboard.press("Meta+2");
  await page
    .getByRole("button", { name: "Rules for Migration/AGENCY", exact: true })
    .click();
  await page.screenshot({
    animations: "disabled",
    path: "/private/tmp/tuck-fresh-folders.png",
  });
  await page.keyboard.press("Meta+,");
  await expect(
    page.getByRole("button", { name: "Settings", exact: true }),
  ).toHaveAttribute("aria-current", "page");
  await page.screenshot({
    animations: "disabled",
    path: "/private/tmp/tuck-fresh-settings.png",
  });
  await page
    .getByRole("button", { name: "Add connection", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Save settings", exact: true }),
  ).toBeDisabled();
  await page.getByLabel("Model ID", { exact: true }).fill("my-model");
  await page.screenshot({
    animations: "disabled",
    path: "/private/tmp/tuck-fresh-connection.png",
  });
  await page.setViewportSize({ width: 900, height: 650 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page
    .getByRole("button", { name: "Discard changes", exact: true })
    .click();
  await page.screenshot({
    animations: "disabled",
    path: "/private/tmp/tuck-fresh-minimum.png",
  });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.screenshot({
    animations: "disabled",
    path: "/private/tmp/tuck-fresh-dark.png",
  });
  expect(
    await page
      .locator(".settings-sheet")
      .evaluate((e) => getComputedStyle(e).animationName),
  ).toBe("none");
});

test("partial suggestions show coverage and file reasons before approval", async ({
  page,
}) => {
  await emptyFolder(page);
  await page.evaluate(() => {
    (window as any).proposalOverride = {
      sampled: 4,
      skipped: 21,
      reviewFiles: Array.from({ length: 21 }, (_, i) => ({
        path: `/Users/test/Documents/scan-${i + 1}.pdf`,
        reason: "The document type could not be confirmed.",
      })),
    };
  });
  await page
    .getByRole("button", { name: "Suggest categories", exact: true })
    .click();
  await expect(page.getByRole("status")).toContainText(
    "Based on 4 of 25 documents",
  );
  await expect(page.getByRole("status")).toContainText(
    "21 files still need review",
  );
  await page.getByText("Files needing review (21)", { exact: true }).click();
  await expect(page.getByText("scan-1.pdf", { exact: true })).toBeVisible();
  for (const colorScheme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme });
    await page.setViewportSize({ width: 860, height: 640 });
    await expect(page.locator(".proposal-coverage")).toBeVisible();
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    ).toBe(true);
    await page.screenshot({
      path: `/private/tmp/tuck-partial-${colorScheme}.png`,
      animations: "disabled",
    });
  }
  await page.getByRole("button", { name: "Organize", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Partial folder suggestions" }),
  ).toBeVisible();
  expect(
    await page.evaluate(() =>
      (window as any).calls.some(
        (c: any) =>
          c.command === "save_preferences" || c.command === "start_tuck",
      ),
    ),
  ).toBe(false);
});
