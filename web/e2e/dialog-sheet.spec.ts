// A page dialog at a compact width is Paseo's AdaptiveModalSheet (TRACKER
// 787): a sheet from the bottom at 65% of the window, raised to 90% by a
// drag up or by a field taking focus, lowered or closed by a drag down; a
// handle on top. Wider it is a centred card at most 520 wide. The context
// ring's figures open in a sheet as tall as they are (Paseo's
// ContextWindowSheet).

import { expect, type Locator, type Page, test } from "@playwright/test";

import { startSession } from "./kit";
import { openUi } from "./open";

const PHONE = { width: 390, height: 844 };

/** Within 2px: the snaps are fractions of the window, laid out in
 * fractional pixels. */
function near(actual: number, expected: number): void {
  expect(Math.abs(actual - expected)).toBeLessThan(2);
}

/** The dialog's box once its height has settled (the snap animates). */
async function settled(dialog: Locator) {
  let last = -1;
  await expect(async () => {
    const box = await dialog.boundingBox();
    expect(box).not.toBeNull();
    const height = box?.height ?? 0;
    const moved = Math.abs(height - last);
    last = height;
    expect(moved).toBeLessThan(0.5);
  }).toPass({ intervals: [120, 120, 120, 250, 500] });
  const box = await dialog.boundingBox();
  if (box === null) throw new Error("the dialog has no box");
  return box;
}

/** Drag `handle` by `dy` pixels with the mouse, in steps, then let go. */
async function dragBy(page: Page, handle: Locator, dy: number): Promise<void> {
  const box = await handle.boundingBox();
  if (box === null) throw new Error("the handle has no box");
  const x = box.x + box.width / 2;
  const y = box.y + box.height / 2;
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x, y + dy, { steps: 12 });
  // Held still before letting go: a placement, not a fling.
  await page.waitForTimeout(150);
  await page.mouse.up();
}

async function openAddMachine(page: Page): Promise<Locator> {
  await page.getByRole("button", { name: "Add machine" }).click();
  const add = page.getByRole("dialog", { name: "Add a machine" });
  await expect(add).toBeVisible();
  return add;
}

test("a compact dialog is a bottom sheet that snaps, extends and drags closed", async ({
  page,
}) => {
  const origin = await openUi(page);
  await page.setViewportSize(PHONE);
  await page.goto(`${origin}/settings/machines`);
  let add = await openAddMachine(page);
  const handle = add.locator(".dialog-handle");
  await expect(handle).toBeVisible();
  let box = await settled(add);
  near(box.width, PHONE.width);
  near(box.y + box.height, PHONE.height);
  near(box.height, PHONE.height * 0.65);
  // The sheet holds focus, not its field: no keyboard until one is pressed.
  await expect(add).toBeFocused();

  // A field taking focus raises it to 90% (Gorhom's keyboard "extend").
  await add.getByLabel("Paste the link it printed").click();
  box = await settled(add);
  near(box.height, PHONE.height * 0.9);

  // A little way down lands on 65%; far down closes it.
  await dragBy(page, handle, 120);
  box = await settled(add);
  near(box.height, PHONE.height * 0.65);
  await dragBy(page, handle, 400);
  await expect(add).toBeHidden();

  // A drag up past halfway raises it to 90%.
  add = await openAddMachine(page);
  await settled(add);
  await dragBy(page, add.locator(".dialog-handle"), -160);
  box = await settled(add);
  near(box.height, PHONE.height * 0.9);
  await page.keyboard.press("Escape");
  await expect(add).toBeHidden();

  // Wider: a centred card, 520 at most, no handle.
  await page.setViewportSize({ width: 1280, height: 800 });
  add = await openAddMachine(page);
  await expect(add.locator(".dialog-handle")).toBeHidden();
  box = await settled(add);
  near(box.width, 520);
  near(box.x + box.width / 2, 640);
});

test("on a compact window the context ring opens its figures in a sheet", async ({
  page,
}) => {
  await openUi(page);
  await startSession(page, "laptop", "Structured", "usage");
  const prompt = page.getByRole("form", { name: "Prompt" });
  await expect(
    prompt.getByRole("img", { name: "Context 1% used" }),
  ).toBeVisible();
  await page.setViewportSize(PHONE);
  const ring = prompt.getByRole("button", { name: "Context 1% used" });
  await ring.click();
  const sheet = page.getByRole("dialog", { name: "Context window" });
  await expect(sheet).toBeVisible();
  await expect(sheet.getByText("1% used", { exact: true })).toBeVisible();
  await expect(
    sheet.getByText("1k / 200k tokens", { exact: true }),
  ).toBeVisible();
  // As tall as its figures, at the window's foot.
  const box = await settled(sheet);
  expect(box.height).toBeLessThan(PHONE.height * 0.8);
  near(box.y + box.height, PHONE.height);
  await sheet.getByRole("button", { name: "Close" }).click();
  await expect(sheet).toBeHidden();
});
