// Actual controls and geometry shared by the feedback regressions/captures.
import { expect, type Locator, type Page } from "@playwright/test";

export const STACK_FIXTURE_COUNT = 8;
export const FOCUS_FRAME_MS = 120;
// Paseo toast-host: 16px at each side, then a 92% width boundary.
// A literal measured result keeps this independent of the CSS formula.
const REFERENCE_NARROW_TOAST = { viewport: 390, card: 329.36 } as const;

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
  await page
    .getByRole("menuitem", { name: "Copy session id" })
    .click({ force: timing === "paused" });
}

export async function clearToastBounds(page: Page): Promise<Locator> {
  const toast = page
    .locator(".toast")
    .filter({ hasText: "Copied the session id" });
  await expect(toast).toBeVisible();
  const viewport = page.viewportSize();
  if (viewport === null) throw new Error("Expected a fixed fixture viewport");
  const notice = await rectangle(toast);
  if (viewport.width === REFERENCE_NARROW_TOAST.viewport) {
    expect(notice.width).toBeCloseTo(REFERENCE_NARROW_TOAST.card, 1);
  }
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

export async function stackedToasts(page: Page): Promise<void> {
  await page.clock.install();
  await page.clock.pauseAt(new Date(Date.now() + 1000));
  for (let index = 0; index < STACK_FIXTURE_COUNT; index += 1) {
    await copyToast(page, "paused");
    await page.clock.runFor(FOCUS_FRAME_MS);
    await expect(page.locator(".toast")).toHaveCount(index + 1);
  }
  const stack = page.locator(".toast-stack");
  const bounds = await rectangle(stack);
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
  expect(
    await stack.evaluate(
      (element) => element.scrollHeight > element.clientHeight,
    ),
  ).toBe(true);
  await stack.evaluate((element) => {
    element.scrollTop = element.scrollHeight;
  });
  await expect(page.locator(".toast").last()).toBeInViewport({ ratio: 1 });
  await page.clock.resume();
}
