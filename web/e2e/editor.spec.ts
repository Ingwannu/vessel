import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { expect, type Page, test } from "@playwright/test";

import { loadEnv } from "./fleet";
import { acknowledgePreference, openUi } from "./open";
import { expectEditorText, openPanelTab, startInShop } from "./shop";

async function openTaxSource(page: Page, prompt: string): Promise<void> {
  await startInShop(page, "Local", prompt);
  await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
  await openPanelTab(page, "Files");
  await page.getByRole("searchbox", { name: "Go to file" }).fill("tax");
  await page
    .getByRole("list", { name: "Files found" })
    .getByRole("button", { name: /tax\.ts/ })
    .click();
}

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
  const code = 'const tax = "flat";\r\n/* rate\r\ncomment */\r\n';
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
    writeFileSync(file, `first\r\n${code}`);
    await openTaxSource(page, "Edit a source file with Vim");
    await expect(page.locator(".f-line").first()).toContainText("first");
    expect(editorRequests).toHaveLength(0);

    await page.getByRole("button", { name: "Edit", exact: true }).click();
    const editor = page.getByRole("textbox", { name: "Edit src/tax.ts" });
    const toolbar = page.locator(".file-head");
    const normal = toolbar.getByRole("status", { name: "Vim mode NORMAL" });
    const unchanged = `first\r\n${code}`.replaceAll("\r\n", "\n");
    await expect(normal).toHaveText("NORMAL");
    await expect(toolbar).toContainText("Ln 1, Col 1");
    await expectEditorText(editor, unchanged);
    await expect(editor.locator(".syn-keyword")).toHaveText("const");
    await expect(editor.locator(".syn-string")).toHaveText('"flat"');
    await expect(editor.locator(".syn-comment")).toHaveText([
      "/* rate",
      "comment */",
    ]);
    expect(editorRequests).toHaveLength(1);

    await editor.press("Enter");
    await expectEditorText(editor, unchanged);
    await expect(toolbar.getByRole("button", { name: "Save" })).toBeDisabled();
    await editor.press("l");
    await editor.press("Backspace");
    await expectEditorText(editor, unchanged);
    await expect(toolbar.getByRole("button", { name: "Save" })).toBeDisabled();
    await editor.press("v");
    await editor.press("l");
    await expect(
      toolbar.getByRole("status", { name: "Vim mode VISUAL" }),
    ).toHaveText("VISUAL");
    await editor.press("Escape");
    await expect(normal).toHaveText("NORMAL");
    await expectEditorText(editor, unchanged);

    await editor.press("g");
    await editor.press("g");
    await editor.press("0");
    await editor.press("x");
    await editor.press("i");
    await expect(
      toolbar.getByRole("status", { name: "Vim mode INSERT" }),
    ).toHaveText("INSERT");
    await page.keyboard.type("F");
    await editor.press("Escape");
    await editor.press("$");
    await editor.press("a");
    await editor.press("Enter");
    await page.keyboard.type("added");
    await expect(toolbar).toContainText("Ln 2, Col 6");
    await editor.press("Escape");
    await expect(normal).toHaveText("NORMAL");
    await expectEditorText(
      editor,
      `First\r\nadded\r\n${code}`.replaceAll("\r\n", "\n"),
    );
    await expect(editor.locator(".syn-keyword")).toHaveText("const");
    await editor.press("Control+s");
    await expect(editor).toHaveCount(0);
    expect(readFileSync(file, "utf8")).toBe(`First\r\nadded\r\n${code}`);

    await toolbar.getByRole("button", { name: "Edit", exact: true }).click();
    await expect(normal).toHaveText("NORMAL");
    await editor.press("v");
    await editor.press("l");
    await editor.press("Escape");
    await editor.press("Tab");
    const leftAfterVisual = await editor.evaluate(
      (field) => !field.contains(document.activeElement),
    );
    await editor.focus();
    await editor.press("Escape");
    await editor.press("Tab");
    const leftFromNormal = await editor.evaluate(
      (field) => !field.contains(document.activeElement),
    );
    console.log(
      `Vim Escape then Tab left the editor (from Visual: ${String(leftAfterVisual)}, from Normal: ${String(leftFromNormal)}).`,
    );
    await toolbar.getByRole("button", { name: "Cancel" }).click();
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

test("standard editing indents with Tab and lets Escape then Tab move focus out", async ({
  page,
}) => {
  const origin = await openUi(page);
  await page.goto(`${origin}/settings/editor`);
  await expect(
    page.getByRole("switch", { name: "Vim keybindings" }),
  ).not.toBeChecked();
  await openTaxSource(page, "Indent a source file with the standard editor");
  await page.getByRole("button", { name: "Edit", exact: true }).click();
  const editor = page.getByRole("textbox", { name: "Edit src/tax.ts" });
  const source = readFileSync(
    join(loadEnv().root, "repos", "shop", "src", "tax.ts"),
    "utf8",
  );
  await expectEditorText(editor, source);

  await editor.press("Tab");
  await expectEditorText(editor, `  ${source}`);
  await expect(editor).toBeFocused();
  await editor.press("Escape");
  await editor.press("Tab");
  await expect(editor).not.toBeFocused();
  await expectEditorText(editor, `  ${source}`);
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
});
