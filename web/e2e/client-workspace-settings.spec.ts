import { expect, test } from "@playwright/test";

import { choose } from "./kit";
import { acknowledgePreference, openUi } from "./open";
import { openPanelTab, panelTab, startInShop } from "./shop";

test("terminal scrollback saves bounded lines and Files shows the real empty state", async ({
  page,
}) => {
  const origin = await openUi(page);
  await page.goto(`${origin}/settings/terminal`);
  const input = page.getByRole("textbox", {
    name: "Terminal scrollback lines",
  });
  await expect(input).toHaveValue("10000");
  try {
    await acknowledgePreference(
      page,
      async () => {
        await input.fill("1000001");
        await input.press("Enter");
      },
      (prefs) => prefs.terminal.scrollback === 1_000_000,
    );
    await page.reload();
    await expect(input).toHaveValue("1000000");
    await input.fill("");
    await input.press("Tab");
    await expect(input).toHaveValue("1000000");
  } finally {
    if ((await input.inputValue()) !== "10000") {
      await acknowledgePreference(
        page,
        async () => {
          await input.fill("10000");
          await input.press("Enter");
        },
        (prefs) => prefs.terminal.scrollback === 10_000,
      );
    }
  }
  await page.goto(`${origin}/settings/files`);
  await expect(
    page.getByText("File panels appear here when a plugin adds one"),
  ).toBeVisible();
});

test("explorer files open beside their tree and keep one dirty editor when the pane changes", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  const origin = await openUi(page);
  const settings = `${origin}/settings/general`;
  await page.goto(settings);
  await expect(page.getByRole("heading", { name: "Composer" })).toBeVisible();
  const placement = page.getByRole("heading", { name: "Open location" });
  await expect(placement).toHaveCount(0);
  await page.setViewportSize({ width: 1280, height: 900 });
  await expect(placement).toBeVisible();
  const source = page.getByRole("combobox", {
    name: "Opening a file from the Files tab",
  });
  try {
    await acknowledgePreference(
      page,
      () => choose(page, source, /^On the side$/),
      (prefs) => prefs.open_location.explorer_files === "side",
    );
    await startInShop(page, "Local", "Open a file beside the explorer");
    await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
    await openPanelTab(page, "Files");
    const tree = page.getByRole("region", { name: "Files" });
    await tree.getByRole("button", { name: "src", exact: true }).click();
    await tree.getByRole("button", { name: "tax.ts", exact: true }).click();

    const side = page.getByRole("tabpanel", { name: "tax.ts" });
    await expect(side).toHaveAttribute("data-placement", "side");
    await expect(panelTab(page, "Files")).toHaveAttribute(
      "aria-selected",
      "true",
    );
    await side.getByRole("button", { name: "Edit", exact: true }).click();
    const editor = page.getByRole("textbox", { name: "Edit src/tax.ts" });
    await editor.fill("export const RATE = 0.81;\n");
    await side.getByRole("button", { name: "Close", exact: true }).click();
    await page
      .getByRole("dialog", { name: "Close unsaved tab?" })
      .getByRole("button", { name: "Cancel" })
      .click();
    await expect(editor).toHaveValue("export const RATE = 0.81;\n");

    await tree.getByRole("button", { name: "cart.ts", exact: true }).click();
    await expect(
      page.getByRole("tabpanel", { name: "cart.ts" }),
    ).toHaveAttribute("data-placement", "side");
    await panelTab(page, "tax.ts").click();
    await expect(editor).toHaveValue("export const RATE = 0.81;\n");
    await expect(editor).toHaveCount(1);
    await page.getByRole("button", { name: "Close · tax.ts" }).click();
    await page
      .getByRole("dialog", { name: "Close unsaved tab?" })
      .getByRole("button", { name: "Close", exact: true })
      .last()
      .click();
    await expect(panelTab(page, "tax.ts")).toHaveCount(0);
  } finally {
    await page.goto(settings);
    await acknowledgePreference(
      page,
      () => choose(page, source, /^Main panel$/),
      (prefs) => prefs.open_location.explorer_files === "main",
    );
  }
});
