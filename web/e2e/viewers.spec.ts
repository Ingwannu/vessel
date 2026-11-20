// The Files tab's viewers (Paseo's built-in file panels; F-F38, F-F39,
// F-F40; docs/plans/wp-file-viewers.md), against files written into the
// fleet's `shop` repository for the test and removed after it: a Markdown
// file opens on its Preview (front matter table, an image read through
// the node, a relative link that opens the file), Source beside it, and
// on Source when opened at a line; an HTML file shows in a sandboxed
// frame whose scripts do not run; an image is drawn, zooms, fits again,
// and follows the file on disk.

import { mkdirSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { expect, type Page, test } from "@playwright/test";

import { loadEnv } from "./fleet";
import { openUi } from "./open";
import { openPanelTab, startInShop } from "./shop";

/** A 4×2 and a 2×4 PNG, to tell the image's two versions apart. */
const WIDE = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAQAAAACCAIAAADwyuo0AAAAEElEQVR4nGOQizoBRwzIHAB8ygoBb1TmcAAAAABJRU5ErkJggg==",
  "base64",
);
const TALL = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAIAAAAECAIAAAArjXluAAAAEElEQVR4nGM4YSMHRAzYKACETAkRDQzFEgAAAABJRU5ErkJggg==",
  "base64",
);

const GUIDE = `---
name: release-guide
allowed-tools:
  - Bash
  - Read
---
# Release guide

See [the notes](../notes/todo.md) first.

![wide dot](wide.png)
`;

const PAGE = `<!doctype html><html><body>
<h1 id="title">Static page</h1>
<script>document.getElementById("title").textContent = "Scripts ran";</script>
</body></html>
`;

/** Open `name` from the Files tab's tree (its folder already open). */
async function openInTree(page: Page, name: string) {
  await page
    .getByRole("region", { name: "Files" })
    .getByRole("button", { name, exact: true })
    .click();
}

/** The natural width of the first image `selector` finds; -1 while there
 * is none or it has not loaded. */
function naturalWidth(page: Page, selector: string): Promise<number> {
  return page.evaluate((query) => {
    const image = document.querySelector(query);
    return image instanceof HTMLImageElement && image.complete
      ? image.naturalWidth
      : -1;
  }, selector);
}

test("Markdown, HTML and images open in Paseo's panels, and an image follows the disk", async ({
  page,
}) => {
  const env = loadEnv();
  const shop = realpathSync(join(env.root, "repos", "shop"));
  const viewers = join(shop, "viewers");
  mkdirSync(viewers, { recursive: true });
  writeFileSync(join(viewers, "guide.md"), GUIDE);
  writeFileSync(join(viewers, "wide.png"), WIDE);
  writeFileSync(join(viewers, "page.html"), PAGE);
  try {
    await openUi(page);
    await startInShop(page, "Local", "Look at the viewers");
    await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
    await openPanelTab(page, "Files");
    await page
      .getByRole("region", { name: "Files" })
      .getByRole("button", { name: "viewers", exact: true })
      .click();

    // Markdown: Preview first, its front matter as Paseo's table, the
    // image read through the node, the body as the conversation draws it.
    await openInTree(page, "guide.md");
    const side = page.getByRole("radiogroup", { name: "File view" });
    await expect(side.getByRole("radio", { name: "Preview" })).toBeChecked();
    const preview = page
      .getByRole("tabpanel", { name: "guide.md" })
      .getByTestId("markdown-preview");
    const front = page.getByTestId("markdown-front-matter");
    await expect(front.locator("dt")).toHaveText(["name", "allowed-tools"]);
    await expect(front.locator("dd")).toHaveText([
      "release-guide",
      "Bash, Read",
    ]);
    await expect(
      preview.getByRole("heading", { name: "Release guide" }),
    ).toBeVisible();
    const dot = preview.getByRole("img", { name: "wide dot" });
    await expect(dot).toBeVisible();
    await expect
      .poll(() => naturalWidth(page, '[data-testid="markdown-preview"] img'))
      .toBe(4);
    // Source: the file's lines, front matter included.
    await side.getByRole("radio", { name: "Source" }).click();
    await expect(page.locator(".f-line").first()).toContainText("---");
    await side.getByRole("radio", { name: "Preview" }).click();
    // A relative link opens that file here, on its own Preview.
    await preview.getByRole("button", { name: "the notes" }).click();
    await expect(
      page
        .getByRole("tabpanel", { name: "todo.md" })
        .getByTestId("markdown-preview"),
    ).toContainText("pay with a card");
    await page.getByRole("button", { name: "All files" }).click();

    // Opened at a line (a content search's hit): Source.
    await page.getByRole("button", { name: "Search in files" }).click();
    await page
      .getByRole("searchbox", { name: "Search in files" })
      .fill("Release guide");
    await page
      .getByRole("list", { name: "Lines found" })
      .getByRole("button", { name: /# Release guide/ })
      .click();
    await expect(
      page
        .getByRole("radiogroup", { name: "File view" })
        .getByRole("radio", { name: "Source" }),
    ).toBeChecked();
    await page.getByRole("button", { name: "All files" }).click();
    await page.getByRole("button", { name: "Search in files" }).click();
    await page.getByRole("searchbox", { name: "Go to file" }).fill("");

    // HTML: the page in a sandboxed frame; its script does not run.
    await openInTree(page, "page.html");
    const frame = page.locator('iframe[aria-label="HTML preview"]');
    await expect(frame).toHaveAttribute("sandbox", "");
    await expect(
      page.frameLocator('iframe[aria-label="HTML preview"]').locator("h1"),
    ).toHaveText("Static page");
    await page.getByRole("button", { name: "All files" }).click();

    // An image: fitted, zoomed from its corner buttons, fitted again by a
    // double click; a new image on disk is read and drawn.
    await openInTree(page, "wide.png");
    const image = page.getByRole("img", { name: "wide.png" });
    await expect(image).toBeVisible();
    const reset = page.getByRole("button", { name: "Reset view" });
    await expect(reset).toBeDisabled();
    await image.hover();
    await page.getByRole("button", { name: "Zoom in" }).click();
    await expect(reset).toBeEnabled();
    await image.dblclick();
    await expect(reset).toBeDisabled();
    await expect
      .poll(() => naturalWidth(page, ".zoomable-content img"))
      .toBe(4);
    writeFileSync(join(viewers, "wide.png"), TALL);
    await expect
      .poll(() => naturalWidth(page, ".zoomable-content img"))
      .toBe(2);
  } finally {
    rmSync(viewers, { recursive: true, force: true });
  }
});
