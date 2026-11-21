import { expect, test } from "@playwright/test";

import { openUi } from "./open";
import { panelTab, startInShop } from "./shop";

test("terminal Find holds the bottom viewport as output streams", async ({
  page,
}) => {
  await openUi(page);
  await startInShop(page, "Local", "Inspect streaming terminal output");
  await page.getByRole("button", { name: "New tab", exact: true }).click();
  await page.getByRole("menuitem", { name: "Terminal", exact: true }).click();
  const terminal = page.locator(".workspace-terminal .xterm-host");
  try {
    await terminal.click();
    // read keeps the shell ready to stream only after Find has a match.
    await page.keyboard.insertText(
      [
        "seq 1 120",
        "printf 'inspected-%s\\n' anchor",
        "read -r release",
        "for batch in 1 2 3; do seq 1 45; sleep 0.05; done",
        "printf 'streamed-%s\\n' anchor",
      ].join("; "),
    );
    await page.keyboard.press("Enter");
    await expect(terminal).toContainText("inspected-anchor");
    await page.keyboard.press("Control+f");
    const find = page.getByRole("search", { name: "Find in terminal" });
    await find.getByRole("textbox").fill("-anchor");
    await expect(find.getByRole("status")).toHaveText("1 of 1");

    await terminal.click({ position: { x: 12, y: 14 } });
    await page.keyboard.press("Enter");
    // The new match proves the later writes reached the same buffer.
    await expect(find.getByRole("status")).toHaveText(/^(?:[12] of )?2$/);
    await expect(terminal).toContainText("inspected-anchor");
    await expect(terminal).not.toContainText("streamed-anchor");
  } finally {
    await page.getByRole("button", { name: "Close · Terminal" }).click();
    await page
      .getByRole("dialog", { name: "Close terminal?" })
      .getByRole("button", { name: "Close", exact: true })
      .last()
      .click();
    await expect(panelTab(page, "Terminal")).toHaveCount(0);
  }
});
