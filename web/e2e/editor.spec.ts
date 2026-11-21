import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { expect, test } from "@playwright/test";

import { loadEnv } from "./fleet";
import { acknowledgePreference, openUi } from "./open";
import { expectEditorText, openPanelTab, startInShop } from "./shop";

test("the editor loads on Edit and Vim saves CRLF through the file view", async ({
  page,
}) => {
  const editorRequests: string[] = [];
  page.on("request", (request) => {
    const path = new URL(request.url()).pathname;
    if (/\/assets\/file-editor-[^/]+\.js$/u.test(path)) {
      editorRequests.push(path);
    }
  });

  const origin = await openUi(page);
  const settings = `${origin}/settings/editor`;
  const file = join(loadEnv().root, "repos", "shop", "src", "tax.ts");
  const before = readFileSync(file, "utf8");
  await page.goto(settings);
  const toggle = page.getByRole("switch", { name: "Vim keybindings" });
  await expect(toggle).not.toBeChecked();
  expect(editorRequests).toHaveLength(0);

  try {
    await acknowledgePreference(
      page,
      () => toggle.click(),
      (prefs) => prefs.editor.vim_keybindings,
    );
    await page.reload();
    await expect(toggle).toBeChecked();
    writeFileSync(file, "first\r\nsecond\r\n");
    await startInShop(page, "Local", "Edit a source file with Vim");
    await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
    await openPanelTab(page, "Files");
    await page.getByRole("searchbox", { name: "Go to file" }).fill("tax");
    await page
      .getByRole("list", { name: "Files found" })
      .getByRole("button", { name: /tax\.ts/ })
      .click();
    await expect(page.locator(".f-line").first()).toContainText("first");
    expect(editorRequests).toHaveLength(0);

    await page.getByRole("button", { name: "Edit", exact: true }).click();
    const editor = page.getByRole("textbox", { name: "Edit src/tax.ts" });
    const mode = page.getByRole("status").filter({ hasText: /^Vim /u });
    await expect(mode).toHaveText("Vim normal");
    await expectEditorText(editor, "first\nsecond\n");
    expect(editorRequests).toHaveLength(1);
    await editor.press("g");
    await editor.press("g");
    await editor.press("0");
    await editor.press("x");
    await editor.press("i");
    await expect(mode).toHaveText("Vim insert");
    await page.keyboard.type("F");
    await editor.press("Escape");
    await editor.press("Shift+a");
    await editor.press("Enter");
    await page.keyboard.type("added");
    await editor.press("Escape");
    await expect(mode).toHaveText("Vim normal");
    await expectEditorText(editor, "First\nadded\nsecond\n");
    await editor.press("Control+s");
    await expect(editor).toHaveCount(0);
    expect(readFileSync(file, "utf8")).toBe("First\r\nadded\r\nsecond\r\n");
  } finally {
    writeFileSync(file, before);
    await page.goto(settings);
    if (await toggle.isChecked()) {
      await acknowledgePreference(
        page,
        () => toggle.click(),
        (prefs) => !prefs.editor.vim_keybindings,
      );
    }
  }
});
