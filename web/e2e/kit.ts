// Driving the kit's controls (src/ui/kit) as a person does: a select is a
// button that opens a list of options, a segmented control is a group of
// radios. Shared by the specs and the screens.

import { expect, type Locator, type Page } from "@playwright/test";

/** Open the select named `field` and choose the option whose name
 * matches `option` (the label, then any detail after it). */
export async function choose(
  page: Page,
  field: Locator,
  option: RegExp,
): Promise<void> {
  await field.click();
  await page.getByRole("option", { name: option }).click();
  await expect(page.getByRole("listbox")).toBeHidden();
}

/** The option `option` in the open list of `field`, after opening it. */
export async function optionOf(
  page: Page,
  field: Locator,
  option: RegExp,
): Promise<Locator> {
  await field.click();
  return page.getByRole("option", { name: option });
}

/** The new-session page (the index's, else the title bar's New) on
 * `node` with Claude Code, in no project (the home folder); opened as a
 * terminal for `"Terminal"`. Nothing is sent yet: the prompt box is
 * returned, its text empty. */
export async function newSessionPage(
  page: Page,
  node: string,
  mode: "Structured" | "Terminal",
): Promise<Locator> {
  await openNew(page);
  await choose(
    page,
    page.getByRole("combobox", { name: "Machine" }),
    new RegExp(`^${node}\\b`),
  );
  await chooseAgent(page, "Claude Code");
  // Where the tests' sessions run: the machine's home folder, as before
  // the page started a new session in the project worked in last (the
  // new chat, 2026-11-13); a test that wants a project chooses it.
  const project = page.getByRole("combobox", { name: "Project" });
  if (
    (await project.isVisible()) &&
    !(await project.innerText()).includes("No project")
  ) {
    await choose(page, project, /^No project/);
  }
  if (mode === "Terminal") {
    await page.getByRole("button", { name: "Add", exact: true }).click();
    await page.getByRole("switch", { name: "Open as terminal" }).click();
    await page.keyboard.press("Escape");
  }
  return page.getByRole("form", { name: "Prompt" });
}

/** The new-session page: the index opens it (the first screen is a
 * composer, as GOAT's), any other page by its title bar's New. */
export async function openNew(page: Page): Promise<void> {
  const heading = page.getByRole("heading", {
    name: "What do you want to work on?",
  });
  const button = page.getByRole("button", { name: "New", exact: true });
  // The index draws the shell (with a New) before it goes on to the new
  // chat: a New pressed then is taken away with it.
  await page.waitForURL((url) => url.pathname !== "/");
  await expect(heading.or(button)).toBeVisible();
  if (!(await heading.isVisible())) {
    await button.click();
  }
  await expect(heading).toBeVisible();
}

/** Close the open menu with Escape, once it holds the focus: a menu
 * draws its items before it takes the focus, and an Escape pressed in
 * between goes to the button that opened it and is lost; the menu then
 * stays open, and the next press on that button closes it instead of
 * opening it (marks.spec.ts, one run in about twelve, 2026-11-13). */
export async function closeMenu(page: Page): Promise<void> {
  await expect
    .poll(() =>
      page.evaluate(
        () => document.activeElement?.closest('[role="menu"]') != null,
      ),
    )
    .toBe(true);
  await page.keyboard.press("Escape");
  await expect(page.getByRole("menu")).toHaveCount(0);
}

/** A session's timeline: its entries, apart from the live region that
 * repeats the newest one's text for screen readers. */
export function timeline(page: Page): Locator {
  return page.getByRole("region", { name: "Timeline" });
}

/** Choose `agent` in the agent, model and effort menu. The item is chosen
 * with Enter, the menu's own keyboard path: a pointer's click waits for
 * the submenu's box to hold still, and under load (the machine at 15 to
 * 47) it was seen "not stable", then "detached", until the test's time
 * ran out (app.spec.ts's terminal test, 2026-11-12). Three attempts to
 * make the page do it on purpose (its processor slowed ten and twelve
 * times, the bridge's answers held back, the agent's options answered
 * while the submenu was open) did not. */
export async function chooseAgent(page: Page, agent: string): Promise<void> {
  await page.getByRole("button", { name: /^Agent, model and effort/ }).click();
  await page.getByRole("menuitem", { name: /^Agent/ }).click();
  await page.getByRole("menuitemradio", { name: agent }).press("Enter");
  await expect(page.getByRole("menu")).toHaveCount(0);
}

/** Start a session from the new-session page with `prompt`, and wait for
 * its page: the address changes before the session's route has drawn
 * (it loads a chunk), and text typed in between lands in the new-session
 * page's box, so a structured session waits for its timeline. */
export async function startSession(
  page: Page,
  node: string,
  mode: "Structured" | "Terminal",
  prompt = "",
): Promise<void> {
  const box = await newSessionPage(page, node, mode);
  if (prompt !== "") {
    await box.getByRole("textbox", { name: "Prompt" }).fill(prompt);
  }
  await box
    .getByRole("button", {
      name: mode === "Terminal" ? "Open the terminal" : "Send",
    })
    .click();
  await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
  if (mode === "Structured") {
    await expect(timeline(page)).toBeVisible();
  }
}
