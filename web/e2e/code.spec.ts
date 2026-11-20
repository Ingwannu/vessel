// The code on the page (milestone 11, track B-page), against the fleet's
// real repositories (e2e/repos.ts): where a session runs, the approval
// of a project's commands (and what changed since the set approved
// last) and setup's output, the Changes and Files tabs,
// git's refusals in words, an edit that meets a change on disk, a shown
// file that follows the disk, a selection sent to the draft, a worktree
// that would lose work, and projects added and cloned from Settings.

import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { expect, type Page, test } from "@playwright/test";

import { loadEnv } from "./fleet";
import { choose, newSessionPage } from "./kit";
import { openUi } from "./open";
import { openPanelTab, panelTab as tab, startInShop } from "./shop";

test("a session starts in a new worktree once its commands are approved, setup shown as it runs", async ({
  page,
}) => {
  test.setTimeout(120_000);
  await openUi(page);
  await startInShop(page, "New worktree", "Work in a worktree");
  // Nothing starts before a person approves the set shown.
  const approval = page.getByRole("dialog", {
    name: "Run this project's commands?",
  });
  await expect(approval).toBeVisible();
  await expect(approval.getByRole("region", { name: "Setup" })).toContainText(
    "echo installing packages",
  );
  await expect(approval.getByText(/^Digest [0-9a-f]{64}$/)).toBeVisible();
  await approval.getByRole("button", { name: "Approve and run" }).click();
  // Setup's output as it comes, then the session in its worktree.
  await expect(
    page.getByRole("region", { name: "Worktree setup" }),
  ).toContainText("installing packages");
  await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
  const chip = page.getByRole("button", {
    name: /^Branch vessel\/[0-9a-f]+: show changes$/,
  });
  await expect(chip).toBeVisible();
  const branch = await chip.getAttribute("aria-label");
  await chip.click();
  await expect(tab(page, "Changes")).toHaveAttribute("aria-selected", "true");
  await expect(page.getByText("No changes against base.")).toBeVisible();
  const prior = page.url();
  await page.getByRole("button", { name: "New tab", exact: true }).click();
  await expect(
    page
      .getByRole("menuitem", { name: "Agent", exact: true })
      .locator(".menu-shortcut"),
  ).toHaveText("Alt+Shift+A");
  await page.keyboard.press("Escape");
  await page.keyboard.press("Alt+Shift+A");
  await page
    .getByRole("textbox", { name: "Prompt" })
    .fill("Review the same worktree");
  await page.getByRole("button", { name: "Send", exact: true }).click();
  await expect(page).not.toHaveURL(prior);
  await expect(
    page.getByRole("button", {
      name: branch ?? /^Branch vessel\/[0-9a-f]+: show changes$/,
    }),
  ).toBeVisible();
  await expect(tab(page, "Review the same worktree")).toHaveAttribute(
    "aria-selected",
    "true",
  );
  await archiveWorktreeAgent(page);

  // The other agent is archived too, so no agent in the worktree is
  // unarchived; the worktree still takes a shell (ADR 0121: the worktree's
  // own place, which needs no agent).
  const first = "Work in a worktree";
  await tab(page, first).click();
  await expect(page).toHaveURL(prior);
  await killWorkspaceAgent(page);
  await expect(
    page.getByRole("region", { name: "Session ended" }),
  ).toBeVisible();
  await openPanelTab(page, "Files");
  await tab(page, first).hover();
  await page.getByRole("button", { name: `Close · ${first}` }).click();
  await expect(tab(page, first)).toHaveCount(0);
  const before = liveWorktreeShells();
  await openPanelTab(page, "Terminal");
  await expect(page.locator(".workspace-terminal .xterm-host")).toBeVisible();
  await expect.poll(() => liveWorktreeShells()).toBe(before + 1);
  await closeWorkspaceTerminal(page);
});

/** How many live shells the node lists in a worktree's own place. */
function liveWorktreeShells(): number {
  return cli(["shell", "list"])
    .split("\n")
    .filter((line) => /\brunning\s+in worktree\b/.test(line)).length;
}

async function killWorkspaceAgent(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Session actions" }).click();
  await page.getByRole("menuitem", { name: "Kill", exact: true }).click();
  await page
    .getByRole("dialog", { name: /^Kill / })
    .getByRole("button", { name: "Kill", exact: true })
    .click();
}

/** A worktree's shells belong to the worktree (ADR 0121, protocol 40):
 * they survive an agent's Kill and its Archive, and none is counted by the
 * archive's confirmation. */
async function archiveWorktreeAgent(page: Page): Promise<void> {
  const route = page.url();
  const agent = "Review the same worktree";
  await openPanelTab(page, "Terminal");
  const shell = page.locator(".workspace-terminal .xterm-host");
  await expect(shell).toBeVisible();
  // The node's own count of live shells in worktree places, this one in.
  const opened = liveWorktreeShells();
  expect(opened).toBeGreaterThan(0);
  await killWorkspaceAgent(page);
  await tab(page, agent).click();
  await expect(
    page.getByRole("region", { name: "Session ended" }),
  ).toBeVisible();

  // Archiving the agents beside the kept terminal tab ends no terminal.
  const terminalTab = tab(page, "Terminal");
  await terminalTab.click({ button: "right" });
  await page.getByRole("menuitem", { name: "Close other tabs" }).click();
  const bulk = page.getByRole("dialog", { name: "Close other tabs?" });
  await expect(bulk).toContainText("archive 2 agent(s)");
  await expect(bulk).not.toContainText("terminal(s)");
  await bulk.getByRole("button", { name: "Cancel" }).click();

  // The ended agent's tab closes (archived) with no word about terminals,
  // and the worktree's shell stays.
  await openPanelTab(page, "Files");
  await tab(page, agent).hover();
  await page.getByRole("button", { name: `Close · ${agent}` }).click();
  await expect(tab(page, agent)).toHaveCount(0);
  await expect(terminalTab).toHaveCount(1);
  // Still running, in the worktree's own place: under the old rule the
  // archive ended it (it sat beside the agent).
  await expect.poll(() => liveWorktreeShells()).toBe(opened);
  await expect(page).toHaveURL(route);
  await closeWorkspaceTerminal(page);
  await expect.poll(() => liveWorktreeShells()).toBe(opened - 1);

  // The address still names the archived agent; the worktree takes a new
  // shell without any agent hosting it.
  await openPanelTab(page, "Terminal");
  await expect(shell).toBeVisible();
  await closeWorkspaceTerminal(page);
  await expect(page).toHaveURL(route);
  await page.reload();
  await expect(tab(page, agent)).toHaveAttribute("aria-selected", "true");
}

async function closeWorkspaceTerminal(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Close · Terminal" }).click();
  await page
    .getByRole("dialog", { name: "Close terminal?" })
    .getByRole("button", { name: "Close", exact: true })
    .last()
    .click();
  await expect(tab(page, "Terminal")).toHaveCount(0);
}

/** `vessel` as the UI's own user runs it: the same book and data. */
function cli(args: string[]): string {
  const env = loadEnv();
  return execFileSync(env.vessel, args, {
    encoding: "utf8",
    env: {
      PATH: process.env["PATH"],
      HOME: env.root,
      USERPROFILE: env.root,
      VESSEL_CONFIG_DIR: join(env.root, "config"),
      VESSEL_DATA_DIR: join(env.root, "ui-data"),
    },
  });
}

test("a set changed since its approval says what changed, its scripts shown with it", async ({
  page,
}) => {
  // main's set approved, as a person would from the CLI (again, if the
  // first test approved it already: the newest time is kept).
  const digest = /^digest: ([0-9a-f]{64})$/m.exec(
    cli(["project", "commands", "shop"]),
  )?.[1];
  expect(digest).toBeDefined();
  cli(["project", "approve", "shop", digest ?? ""]);
  await openUi(page);
  const box = await newSessionPage(page, "laptop", "Structured");
  await choose(
    page,
    page.getByRole("combobox", { name: "Project" }),
    /^shop\b/,
  );
  await choose(
    page,
    page.getByRole("combobox", { name: "Where it runs" }),
    /^New worktree/,
  );
  await choose(
    page,
    page.getByRole("combobox", { name: "Base branch" }),
    /^From next\/tooling/,
  );
  await box.getByRole("textbox", { name: "Prompt" }).fill("Run the tests");
  await box.getByRole("button", { name: "Send" }).click();
  const approval = page.getByRole("dialog", {
    name: "Run this project's commands?",
  });
  await expect(approval).toBeVisible();
  await expect(approval).toContainText(
    /Since the set approved .*: setup commands: 1 changed; script test added; service web added\./,
  );
  const setup = approval.getByRole("region", { name: "Setup" });
  await expect(setup).toContainText("echo setup finished");
  await expect(setup).toContainText("was echo setup done");
  // Every script the digest covers, with what it runs and where.
  const scripts = approval.getByRole("region", {
    name: "Scripts and services",
  });
  await expect(scripts).toContainText("npm test");
  await expect(scripts).toContainText("service in src/");
  await expect(scripts).toContainText("npm run dev");
  await approval.getByRole("button", { name: "Cancel" }).click();
  await expect(approval).toHaveCount(0);
});

test("the Changes tab lists the work, stages it, and says a refusal plainly", async ({
  page,
}) => {
  await openUi(page);
  await startInShop(page, "Local", "Look at the work");
  await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
  await openPanelTab(page, "Diff");
  const list = page.getByRole("region", { name: "Changed files" });
  for (const name of ["cart.ts", "discount.ts", "prices.ts", "todo.md"]) {
    await expect(list.getByText(name, { exact: true })).toBeVisible();
  }
  // The diff is drawn a window at a time, with numbers on each line.
  await expect(list.locator(".d-line.add").first()).toBeVisible();
  // Stage one file, then see it among the staged.
  await list.getByRole("button", { name: "Stage src/cart.ts" }).click();
  await page.getByRole("button", { name: /^Uncommitted/ }).click();
  await page.getByRole("menuitemradio", { name: /^Staged/ }).click();
  await expect(list.getByText("cart.ts", { exact: true })).toBeVisible();
  await expect(list.getByText("prices.ts", { exact: true })).toHaveCount(0);
  // A branch renamed to one that exists: git's refusal, in words.
  await page.getByRole("button", { name: "Git action options" }).click();
  await page.getByRole("menuitem", { name: "Rename branch…" }).click();
  const rename = page.getByRole("dialog", { name: "Rename branch" });
  await rename
    .getByRole("textbox", { name: "New name" })
    .fill("feature/discounts");
  await rename.getByRole("button", { name: "Rename" }).click();
  await expect(rename.getByRole("alert")).toContainText(
    "A branch with that name exists.",
  );
  await page.keyboard.press("Escape");
});

test("an edit is saved, and one saved after the file changed on disk shows both sides and overwrites nothing", async ({
  page,
}) => {
  const env = loadEnv();
  await openUi(page);
  await startInShop(page, "Local", "Edit a file");
  await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
  await openPanelTab(page, "Files");
  await page.getByRole("searchbox", { name: "Go to file" }).fill("tax");
  await page
    .getByRole("list", { name: "Files found" })
    .getByRole("button", { name: /tax\.ts/ })
    .click();
  await expect(page.locator(".f-line").first()).toContainText("RATE");
  const file = join(env.root, "repos", "shop", "src", "tax.ts");
  const editor = page.getByRole("textbox", { name: "Edit src/tax.ts" });
  // A file nobody else touched is written: the guard is the content's
  // hash the edit read, which the node compares itself.
  await page.getByRole("button", { name: "Edit", exact: true }).click();
  await editor.fill("export const RATE = 0.21;\n");
  await page.getByRole("button", { name: "Save" }).click();
  await expect(editor).toHaveCount(0);
  await expect(page.locator(".f-line").first()).toContainText("0.21");
  expect(readFileSync(file, "utf8")).toBe("export const RATE = 0.21;\n");
  await page.getByRole("button", { name: "Edit", exact: true }).click();
  await editor.fill("export const RATE = 0.2;\n");
  // Meanwhile, someone else changes it on disk.
  writeFileSync(file, "export const RATE = 0.25; // from elsewhere\n");
  await page.getByRole("button", { name: "Save" }).click();
  // The save's conflict (the watch's alert may have said "Changed on
  // disk" first; the conflict replaces it).
  const conflict = page
    .getByRole("alert")
    .filter({ hasText: "File changed on disk." });
  await expect(conflict).toContainText("File changed on disk.");
  // Both states: what the edit read, and what is there now.
  await expect(conflict).toContainText("Read: 26 B");
  await expect(conflict).toContainText("On disk now: 44 B");
  expect(readFileSync(file, "utf8")).toContain("from elsewhere");
  await expect(editor).toHaveValue("export const RATE = 0.2;\n");
  await page.getByRole("button", { name: "Reload from disk" }).click();
  await expect(page.locator(".f-line").first()).toContainText("from elsewhere");
});

test("a shown file follows the disk; an open edit keeps its text under Paseo's alert", async ({
  page,
}) => {
  const env = loadEnv();
  const file = join(env.root, "repos", "shop", "notes", "todo.md");
  const before = readFileSync(file, "utf8");
  try {
    await openUi(page);
    await startInShop(page, "Local", "Watch a file");
    await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
    await openPanelTab(page, "Files");
    await page.getByRole("searchbox", { name: "Go to file" }).fill("todo");
    await page
      .getByRole("list", { name: "Files found" })
      .getByRole("button", { name: /todo\.md/ })
      .click();
    // A Markdown file opens on its Preview (Paseo's built-in panel).
    const preview = page.getByTestId("markdown-preview");
    await expect(preview).toContainText("pay with a card");
    // Changed elsewhere while shown: the view follows, nothing pressed.
    writeFileSync(file, "- pay by bank transfer\n");
    await expect(preview).toContainText("pay by bank transfer");
    // An open edit (on the Source side) keeps its text; the alert offers
    // both ways out.
    await page.getByRole("button", { name: "Edit", exact: true }).click();
    const first = page.locator(".f-line").first();
    const editor = page.getByRole("textbox", { name: "Edit notes/todo.md" });
    await editor.fill("- mine\n");
    writeFileSync(file, "- pay in cash\n");
    const alert = page
      .getByRole("alert")
      .filter({ hasText: "Changed on disk" });
    await expect(alert).toContainText(
      "The local buffer was preserved. Choose which version to keep.",
    );
    const overwrite = alert.getByRole("button", { name: "Overwrite" });
    await expect(overwrite).toBeVisible();
    // Its choices sit inside it (a global class once pulled them away).
    const inside = await alert.evaluate((box) => {
      const outer = box.getBoundingClientRect();
      return [...box.querySelectorAll("button")].every((button) => {
        const inner = button.getBoundingClientRect();
        return (
          inner.left >= outer.left &&
          inner.right <= outer.right &&
          inner.top >= outer.top &&
          inner.bottom <= outer.bottom
        );
      });
    });
    expect(inside).toBe(true);
    await expect(editor).toHaveValue("- mine\n");
    expect(readFileSync(file, "utf8")).toBe("- pay in cash\n");
    // Reload asks first, since the edit would be thrown away.
    await alert.getByRole("button", { name: "Reload", exact: true }).click();
    const ask = page.getByRole("dialog", { name: "Reload from disk?" });
    await expect(ask).toContainText("Your local changes will be lost.");
    await ask.getByRole("button", { name: "Cancel", exact: true }).click();
    await expect(editor).toHaveValue("- mine\n");
    await alert.getByRole("button", { name: "Reload", exact: true }).click();
    await ask.getByRole("button", { name: "Reload", exact: true }).click();
    await expect(editor).toHaveCount(0);
    await expect(first).toContainText("pay in cash");
    // Gone: the copy stays under "File deleted on disk"; put back as
    // shown, the alert goes.
    rmSync(file);
    const gone = page
      .getByRole("alert")
      .filter({ hasText: "File deleted on disk" });
    await expect(gone).toContainText("The open copy is preserved.");
    await expect(first).toContainText("pay in cash");
    writeFileSync(file, "- pay in cash\n");
    await expect(gone).toHaveCount(0);
  } finally {
    writeFileSync(file, before);
  }
});

test("lines selected in a diff go into the session's draft as a reference", async ({
  page,
}) => {
  await openUi(page);
  await startInShop(page, "Local", "Read the diff");
  await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
  await openPanelTab(page, "Diff");
  const list = page.getByRole("region", { name: "Changed files" });
  const lines = list.locator('.d-line[data-path="src/cart.ts"]');
  await expect(lines.first()).toBeVisible();
  await lines.first().evaluate((first) => {
    const last = first.parentElement?.parentElement?.querySelectorAll(
      '.d-line[data-path="src/cart.ts"]',
    );
    const end = last?.item(Math.min(3, last.length - 1)) ?? first;
    const range = document.createRange();
    range.setStart(first, 0);
    range.setEnd(end, end.childNodes.length);
    const selection = window.getSelection();
    selection?.removeAllRanges();
    selection?.addRange(range);
    first.dispatchEvent(new MouseEvent("mouseup", { bubbles: true }));
  });
  await page.getByRole("button", { name: "Add to chat" }).click();
  const prompt = page.getByRole("textbox", { name: "Prompt" });
  await expect(prompt).toHaveValue(/User-supplied captured code/);
  await expect(prompt).toHaveValue(/"src\/cart\.ts"#L\d+-L\d+ \(new side\)/);
});

test("projects are added from Paseo's sheet, and a failed clone says why", async ({
  page,
}) => {
  const env = loadEnv();
  const folder = join(env.root, "repos", "added");
  mkdirSync(folder, { recursive: true });
  writeFileSync(join(folder, "README.md"), "# Added\n");
  const parent = join(env.root, "clones");
  mkdirSync(parent, { recursive: true });
  const origin = await openUi(page);
  await page.goto(`${origin}/settings/projects?machine=laptop`);
  await expect(
    page.getByRole("heading", { name: "Projects on laptop" }),
  ).toBeVisible();
  // A folder: Settings' Add project opens the sheet on this machine's
  // methods; a typed path is the first row ("Open this path").
  await page.getByRole("button", { name: "Add project" }).last().click();
  const sheet = page.getByRole("dialog", { name: /^Add project: / });
  await expect(sheet).toHaveAccessibleName("Add project: method");
  await sheet.getByRole("button", { name: /^Search for directory/ }).click();
  await expect(sheet).toHaveAccessibleName("Add project: directory-search");
  // The field, filled: keys typed before the page's field takes focus
  // would go nowhere.
  const field = sheet.getByRole("textbox");
  await field.fill(folder);
  await expect(
    sheet.getByRole("button", { name: /Open this path/ }),
  ).toBeVisible();
  await page.keyboard.press("Enter");
  // Added: the sheet closes on a new session in the project.
  await expect(sheet).toBeHidden();
  await expect(page.getByRole("combobox", { name: "Project" })).toHaveText(
    /added/,
  );
  // A clone of nothing, by its URL: the destination is typed, the folder
  // the clone makes is the row; it starts, then fails, and Settings says
  // why.
  await page.goto(`${origin}/settings/projects?machine=laptop`);
  await page.getByRole("button", { name: "Add project" }).last().click();
  await sheet.getByRole("button", { name: /^Clone from GitHub/ }).click();
  await expect(sheet).toHaveAccessibleName("Add project: github-search");
  await field.fill("http://127.0.0.1:9/nothing.git");
  await sheet
    .getByRole("button", { name: /Clone this repository URL/ })
    .click();
  await expect(sheet).toHaveAccessibleName("Add project: github-location");
  await field.fill(parent);
  await sheet
    .getByRole("button", { name: new RegExp(`Parent directory: ${parent}$`) })
    .first()
    .click();
  await expect(sheet).toBeHidden();
  const rows = page.getByRole("list", { name: "Projects" });
  await expect(rows.getByText("Clone failed", { exact: true })).toBeVisible({
    timeout: 60_000,
  });
  await expect(page.getByText(/^nothing: /)).toBeVisible();
  // The sidebar's "+" on a project starts the next session there, also
  // when the new-session page is already open.
  // (It opens in the project laptop's latest session ran in, shop.)
  await page.goto(`${origin}/new`);
  await expect(page.getByRole("combobox", { name: "Project" })).toHaveText(
    /shop/,
  );
  await page.getByRole("button", { name: "New session in notes" }).click();
  await expect(page.getByRole("combobox", { name: "Project" })).toHaveText(
    /notes/,
  );
});
