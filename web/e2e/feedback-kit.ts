// Actual controls and geometry shared by the feedback regressions/captures.
import { expect, type Locator, type Page } from "@playwright/test";

/** Toasts said in a row by the fixtures; one stays up (Paseo's host). */
export const REPEAT_FIXTURE_COUNT = 8;
export const FOCUS_FRAME_MS = 120;
// Paseo toast-host: 16px at each side, then a boundary 92% as wide, at
// most 480; the card is as wide as its words within it. Literal results
// keep this independent of the CSS formula.
const TOAST_BOUNDARY: Readonly<Record<number, number>> = {
  390: 329.36,
  800: 480,
  1280: 480,
};
/** The host's room for the card's shadow at each side. */
const TOAST_SHADOW_ROOM = 12;

async function rectangle(element: Locator) {
  const value = await element.boundingBox();
  if (value === null) throw new Error("Expected a visible feedback boundary");
  return value;
}

export async function copyToast(
  page: Page,
  timing: "live" | "paused" = "live",
): Promise<void> {
  await page
    .getByRole("button", { name: "Session actions" })
    .click({ force: timing === "paused" });
  if (timing === "paused") await page.clock.runFor(FOCUS_FRAME_MS);
  const copy = page.getByRole("menuitem", { name: "Copy agent id" });
  if (timing === "paused") {
    // Toasts sit above menus (Paseo's planes, tokens.css): on a phone the
    // toast this fixture keeps up covers the menu's first rows, so the row
    // is chosen without a pointer aimed through the toast.
    await copy.dispatchEvent("click");
  } else {
    await copy.click();
  }
}

export async function clearToastBounds(page: Page): Promise<Locator> {
  const toast = page.locator(".toast").filter({ hasText: "Copied Agent ID" });
  await expect(toast).toBeVisible();
  const viewport = page.viewportSize();
  if (viewport === null) throw new Error("Expected a fixed fixture viewport");
  const notice = await rectangle(toast);
  const boundary = TOAST_BOUNDARY[viewport.width];
  if (boundary !== undefined) {
    const host = await rectangle(page.locator(".toast-host"));
    expect(host.width).toBeCloseTo(boundary + TOAST_SHADOW_ROOM * 2, 1);
    // A short message's card is narrower than the boundary (Paseo's card
    // takes its words' width, centred).
    expect(notice.width).toBeLessThan(boundary - 1);
  }
  // Paseo's copied card: its CheckCircle2 in front, no close button.
  await expect(toast.locator(".toast-mark .ico-circle-check")).toBeVisible();
  await expect(toast.getByRole("button")).toHaveCount(0);
  const header = await rectangle(page.getByRole("banner"));
  const prompt = await rectangle(page.getByRole("form", { name: "Prompt" }));
  const next = await rectangle(
    page
      .getByRole("region", { name: "Question from the agent" })
      .getByRole("button", { name: "Next", exact: true }),
  );
  expect(notice.y).toBeGreaterThanOrEqual(header.y + header.height);
  expect(notice.y + notice.height).toBeLessThanOrEqual(
    Math.min(prompt.y, next.y),
  );
  expect(notice.x + notice.width / 2).toBeCloseTo(viewport.width / 2, 0);
  expect(notice.x).toBeGreaterThanOrEqual(0);
  expect(notice.x + notice.width).toBeLessThanOrEqual(viewport.width);
  return toast;
}

/** Freeze the page's clock a second ahead (installing it on first use):
 * toasts leave after 2.2 s, less than a loaded run may take to measure. */
export async function pauseClock(page: Page): Promise<void> {
  await page.clock.pauseAt((await page.evaluate(() => Date.now())) + 1000);
}

/** Says the copied toast eight times on a paused clock (pauseClock). */
export async function repeatedToasts(page: Page): Promise<void> {
  for (let index = 0; index < REPEAT_FIXTURE_COUNT; index += 1) {
    await copyToast(page, "paused");
    await page.clock.runFor(FOCUS_FRAME_MS);
    // Each replaces the one up: never a second card.
    await expect(page.locator(".toast")).toHaveCount(1);
  }
  const host = page.locator(".toast-host");
  const bounds = await rectangle(host);
  const header = await rectangle(page.getByRole("banner"));
  const prompt = await rectangle(page.getByRole("form", { name: "Prompt" }));
  const next = await rectangle(
    page
      .getByRole("region", { name: "Question from the agent" })
      .getByRole("button", { name: "Next", exact: true }),
  );
  expect(bounds.y).toBeGreaterThanOrEqual(header.y + header.height);
  expect(bounds.y + bounds.height).toBeLessThanOrEqual(
    Math.min(prompt.y, next.y),
  );
  await expect(page.locator(".toast")).toBeInViewport({ ratio: 1 });
}
