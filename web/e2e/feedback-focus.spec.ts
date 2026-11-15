// The real lazy page mounts after an already open modal owns focus.
// Hold only its actual asset; the bridge, modal and browser remain real.
import { expect, test, type Locator, type Page } from "@playwright/test";
import { openUi } from "./open";

function latch(): { reached: Promise<void>; release: () => void } {
  let release: () => void = () => undefined;
  const reached = new Promise<void>((resolve) => {
    release = resolve;
  });
  return { reached, release };
}

// Both are painted layers: a late lazy route must not steal a menu's
// keyboard navigation any more than a dialog's text input.
async function mountPromptBehind(
  page: Page,
  openOverlay: () => Promise<Locator>,
): Promise<void> {
  const requested = latch();
  const resume = latch();
  await page.route("**/assets/new-session-*.js", async (route) => {
    requested.release();
    await resume.reached;
    await route.continue();
  });
  try {
    await openUi(page);
    await requested.reached;
    const overlay = await openOverlay();
    await expect
      .poll(() =>
        overlay.evaluate((element) => element.contains(document.activeElement)),
      )
      .toBe(true);
    resume.release();
    await expect(
      page.getByRole("form", { name: "Prompt", includeHidden: true }),
    ).toBeAttached();
    await page.evaluate(
      () =>
        new Promise<void>((resolve) =>
          requestAnimationFrame(() => {
            requestAnimationFrame(() => {
              resolve();
            });
          }),
        ),
    );
    await expect(overlay).toBeVisible();
    await expect
      .poll(() =>
        overlay.evaluate((element) => element.contains(document.activeElement)),
      )
      .toBe(true);
    await page.keyboard.press("Escape");
    await expect(overlay).toBeHidden();
  } finally {
    resume.release();
    await page.unrouteAll({ behavior: "wait" });
  }
}

test("a new prompt mounted behind a dialog leaves focus in the dialog", async ({
  page,
}) => {
  await mountPromptBehind(page, async () => {
    await page
      .getByRole("navigation", { name: "Sidebar" })
      .getByRole("button", { name: "Add machine" })
      .click();
    return page.getByRole("dialog", { name: "Add a machine" });
  });
});

test("a new prompt mounted behind Display leaves focus in its menu", async ({
  page,
}) => {
  await mountPromptBehind(page, async () => {
    await page
      .getByRole("navigation", { name: "Sidebar" })
      .getByRole("button", { name: "Display" })
      .click();
    await page.keyboard.press("ArrowDown");
    return page.getByRole("menu");
  });
});

// Hold the initial-focus animation frame after the palette has focus.
// Escape must still reach the new modal through the actual keyboard path.
test("Escape dismisses a new dialog before its queued initial focus", async ({
  page,
}) => {
  await openUi(page);
  await expect(page.getByRole("form", { name: "Prompt" })).toBeVisible();
  await page.keyboard.press("Control+k");
  const palette = page.getByRole("dialog", { name: "Command palette" });
  await expect(palette.getByRole("combobox", { name: "Search" })).toBeFocused();
  await page.clock.install();
  await page.clock.pauseAt(new Date(Date.now() + 1000));
  await page
    .getByRole("option", { name: /Add a machine/ })
    .click({ force: true });
  const dialog = page.getByRole("dialog", { name: "Add a machine" });
  await expect(dialog).toBeVisible();
  expect(
    await dialog.evaluate((element) =>
      element.contains(document.activeElement),
    ),
  ).toBe(false);
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
});

// A page recorder must not consume a modal's keys while focus is queued.
test("a background key recorder leaves early Escape and composition to the dialog", async ({
  page,
}) => {
  const origin = await openUi(page);
  await page.goto(`${origin}/settings/shortcuts`);
  await page
    .getByRole("button", { name: "Change Find in the session" })
    .click();
  const recorder = page.getByRole("group", {
    name: "Record shortcut for Find in the session",
  });
  await expect(recorder).toBeVisible();
  await page.clock.install();
  await page.clock.pauseAt(new Date(Date.now() + 1000));
  await page
    .getByRole("navigation", { name: "Sidebar" })
    .getByRole("button", { name: /^Search/ })
    .click({ force: true });
  const dialog = page.getByRole("dialog", { name: "Command palette" });
  await expect(dialog).toBeVisible();
  expect(
    await dialog.evaluate((element) =>
      element.contains(document.activeElement),
    ),
  ).toBe(false);
  await dialog.evaluate(() => {
    const key = new KeyboardEvent("keydown", {
      key: "Escape",
      code: "Escape",
      bubbles: true,
      cancelable: true,
      isComposing: true,
    });
    (document.activeElement ?? document).dispatchEvent(key);
  });
  await expect(dialog).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
  await page.clock.resume();
  await expect(recorder).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(recorder).toBeHidden();
});
