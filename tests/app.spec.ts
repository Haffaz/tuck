import { test, expect, type Page } from "@playwright/test";
import { initial } from "../src/types";

async function desktopBridge(page: Page) {
  await page.addInitScript(
    ({ initial }) => {
      Object.defineProperty(window, "isTauri", { value: true });
      const state = structuredClone(initial);
      const callbacks = new Map<number, (value: unknown) => void>();
      let next = 1;
      Object.defineProperty(window, "__TAURI_INTERNALS__", {
        value: {
          metadata: {
            currentWindow: { label: "main" },
            currentWebview: { label: "main" },
          },
          transformCallback: (callback: (value: unknown) => void) => {
            const id = next++;
            callbacks.set(id, callback);
            return id;
          },
          unregisterCallback: (id: number) => callbacks.delete(id),
          invoke: async (command: string, args: any = {}) => {
            if (command === "get_state") return structuredClone(state);
            if (command === "plugin:dialog|open")
              return "/Users/test/Documents";
            if (command === "save_preferences") {
              state.preferences = structuredClone(args.preferences);
              return;
            }
            if (command === "set_watching") {
              state.watching = args.enabled;
              return;
            }
            if (command === "check_model") return ["qwen3:4b"];
            if (command === "organize_existing") {
              state.watching = true;
              state.activity = [
                {
                  id: "example",
                  timestamp: 1700000000,
                  original: "/Users/test/Documents/scan.txt",
                  destination: "/Users/test/Documents/Finances/scan.txt",
                  folder: "Finances",
                  reason: "Matched your rule: bank statement",
                  status: "sorted",
                  hash: "sample",
                  copied: false,
                  duplicate: false,
                },
              ];
              return 1;
            }
            if (command === "undo_action") {
              state.activity[0].status = "undone";
              return;
            }
            if (command === "plugin:event|listen") return next++;
            if (command === "plugin:event|unlisten") return;
            throw new Error(`Unhandled test command ${command}`);
          },
        },
      });
      Object.defineProperty(window, "__TAURI_EVENT_PLUGIN_INTERNALS__", {
        value: { unregisterListener: () => {} },
      });
    },
    { initial },
  );
}

test("first launch explains setup and provides real empty states", async ({
  page,
}) => {
  await desktopBridge(page);
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Everything, in its place." }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Add documents", exact: true }),
  ).toBeDisabled();
  await expect(page.getByText("A fresh start feels good.")).toBeVisible();
  await page.screenshot({
    path: "test-results/tuck-overview.png",
    fullPage: true,
  });
});

test("choose a folder, start and pause watching", async ({ page }) => {
  await desktopBridge(page);
  await page.goto("/");
  await page.getByRole("button", { name: "Choose your first folder" }).click();
  await expect(page.getByText("~/Documents", { exact: true })).toBeVisible();
  await page
    .getByRole("button", { name: "Start watching", exact: true })
    .click();
  await expect(
    page.getByText("New documents will be sorted automatically"),
  ).toBeVisible();
  await page.getByRole("button", { name: "Pause", exact: true }).click();
  await expect(page.getByText("Paused", { exact: true })).toBeVisible();
});

test("edit rules and save preferences without poll overwriting edits", async ({
  page,
}) => {
  await desktopBridge(page);
  await page.goto("/");
  await page.getByRole("button", { name: "Folder rules", exact: true }).click();
  await page
    .getByLabel("Folder 1", { exact: true })
    .fill("Migration/Applications");
  await page.waitForTimeout(1700);
  await expect(page.getByLabel("Folder 1", { exact: true })).toHaveValue(
    "Migration/Applications",
  );
  await page.getByRole("button", { name: "Save rules", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("Preferences saved");
  await page.getByRole("button", { name: "Preferences", exact: true }).click();
  await page.getByRole("switch", { name: "Rename documents" }).click();
  await page.getByRole("button", { name: "Save preferences" }).click();
  await expect(
    page.getByRole("switch", { name: "Rename documents" }),
  ).toHaveAttribute("aria-checked", "false");
  await page.getByRole("button", { name: "Check connection" }).click();
  await expect(
    page.getByText("qwen3:4b is ready", { exact: false }),
  ).toBeVisible();
});

test("existing cleanup has an explicit action and activity supports undo", async ({
  page,
}) => {
  await desktopBridge(page);
  await page.goto("/");
  await page.getByRole("button", { name: "Choose your first folder" }).click();
  await page
    .getByRole("button", { name: "Sort existing", exact: true })
    .click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await page
    .getByRole("button", { name: "Sort existing documents", exact: true })
    .click();
  await page.getByRole("button", { name: "Details for scan.txt" }).click();
  await expect(page.getByRole("dialog")).toContainText(
    "Matched your rule: bank statement",
  );
  await page.getByRole("button", { name: "Undo move", exact: true }).click();
  await expect(page.getByText("Undone", { exact: true })).toBeVisible();
});

test("browser preview clearly disables filesystem actions", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByText("Interface preview", { exact: false }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Choose your first folder" }),
  ).toBeDisabled();
});
