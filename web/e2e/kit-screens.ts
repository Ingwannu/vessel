// `npm run screens`, the kit: the gallery at /kit (features/kit-gallery)
// in both themes, at full width and at a phone's 390px, with the popups
// that are only seen open: a select at the window's right edge holding a
// long Korean option, a menu with a submenu, a tooltip, a dialog and the
// command palette, and keyboard focus. Each popup here is reached from the
// keyboard alone, so the run also proves that the kit can be used without
// a pointer.

import { expect, type Page, test } from "@playwright/test";

import { launchUi } from "./open";
import { shot } from "./shot";

/** The gallery, at its top, in the system's `scheme` (no stored choice:
 * the page follows the system). */
async function gallery(
  page: Page,
  origin: string,
  scheme: "light" | "dark",
): Promise<void> {
  await page.emulateMedia({ colorScheme: scheme });
  await page.goto(`${origin}/kit`);
  await expect(page.getByRole("heading", { name: "Kit" })).toBeVisible();
}

/** The section headed `title`. */
function section(page: Page, title: string) {
  return page.getByRole("region", { name: title });
}

test("the kit, light and dark", async ({ page }) => {
  test.setTimeout(120_000);
  // Only the origin: the gallery needs no token.
  const { origin } = await launchUi(page);

  for (const scheme of ["light", "dark"] as const) {
    await gallery(page, origin, scheme);
    await shot(page, `20-kit-${scheme}`, { fullPage: true });

    // Keyboard only: Tab to the select at the right edge, open it with the
    // arrow key, move down to the long option.
    const edge = section(page, "Choices")
      .locator(".kit-specimens", { hasText: "window's edge" })
      .getByRole("combobox")
      .last();
    await edge.focus();
    await page.keyboard.press("ArrowDown");
    const long = page.getByRole("option", { name: /^아주 긴 이름의 모델/ });
    await expect(long).toBeVisible();
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await shot(page, `21-kit-${scheme}-select-at-edge`);
    await page.keyboard.press("Escape");
    await expect(long).toBeHidden();

    // The menu, from the keyboard, with its submenu open.
    const menu = section(page, "Popups").getByRole("button", {
      name: "Menu",
      exact: true,
    });
    await menu.focus();
    await page.keyboard.press("Enter");
    const sort = page.getByRole("menuitem", { name: "Sort" });
    await expect(sort).toBeVisible();
    await sort.focus();
    await page.keyboard.press("ArrowRight");
    await expect(
      page.getByRole("menuitemradio", { name: "Newest first" }),
    ).toBeVisible();
    await shot(page, `22-kit-${scheme}-menu`);
    await page.keyboard.press("Escape");
    await page.keyboard.press("Escape");

    // A tooltip, from focus, and the focus ring on the button it names.
    await page.keyboard.press("Shift");
    await section(page, "Popups")
      .getByRole("button", { name: "Copy the session id" })
      .focus();
    // Base UI's tooltip is drawn for the eye only (the trigger carries
    // the name), so it has no role to find it by.
    await expect(
      page.locator(".tooltip", { hasText: "Copy the session id" }),
    ).toBeVisible();
    await shot(page, `23-kit-${scheme}-tooltip-focus`);

    // The popover, opened with Enter; then the combobox's suggestions,
    // with the arrow key.
    await section(page, "Popups")
      .getByRole("button", { name: "Popover" })
      .press("Enter");
    await expect(page.locator(".popover")).toBeVisible();
    await shot(page, `23b-kit-${scheme}-popover`);
    await page.keyboard.press("Escape");
    await expect(page.locator(".popover")).toBeHidden();
    await page
      .getByLabel("Folder (suggests, takes anything)")
      .press("ArrowDown");
    await expect(page.locator(".combobox-popup")).toBeVisible();
    await page.keyboard.press("ArrowDown");
    await shot(page, `23c-kit-${scheme}-combobox`);
    await page.keyboard.press("Escape");

    // The dialog and the palette, opened with Enter.
    await section(page, "Popups")
      .getByRole("button", { name: "Dialog", exact: true })
      .press("Enter");
    const dialog = page.getByRole("dialog", { name: "Rename node" });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByLabel("Name")).toBeFocused();
    await shot(page, `24-kit-${scheme}-dialog`);
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
    await section(page, "Popups")
      .getByRole("button", { name: "Command palette" })
      .press("Enter");
    await page.keyboard.insertText("세션");
    await expect(
      page.getByRole("option", { name: /^아주 긴 세션/ }),
    ).toBeVisible();
    await shot(page, `25-kit-${scheme}-palette`);
    await page.keyboard.press("Escape");
  }

  // A phone's width: the page in one column, a select opened at the edge.
  await page.setViewportSize({ width: 390, height: 844 });
  for (const scheme of ["light", "dark"] as const) {
    await gallery(page, origin, scheme);
    await shot(page, `26-kit-390-${scheme}`, { fullPage: true });
  }
  const edge = section(page, "Choices")
    .locator(".kit-specimens", { hasText: "window's edge" })
    .getByRole("combobox")
    .last();
  await edge.scrollIntoViewIfNeeded();
  await edge.click();
  await expect(
    page.getByRole("option", { name: /^아주 긴 이름의 모델/ }),
  ).toBeVisible();
  await shot(page, "27-kit-390-dark-select");
});
