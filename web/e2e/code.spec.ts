// The code on the page (milestone 11, track B-page), against the fleet's
// real repositories (e2e/repos.ts): where a session runs, the approval
// of a project's commands (and what changed since the set approved
// last) and setup's output, the Changes and Files tabs,
// git's refusals in words, an edit that meets a change on disk, a shown
// file that follows the disk, a selection sent to the draft, a worktree
// that would lose work, and projects added and cloned from Settings.

import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { expect, type Page, test } from "@playwright/test";

import { loadEnv } from "./fleet";
import { choose, newSessionPage } from "./kit";
import { openUi } from "./open";

/** The session panel's tab named `name`. */
function tab(page: Page, name: string) {
  return page
    .getByRole("tablist", { name: "Session panel" })
    .getByRole("tab", { name });
}

/** Start a session on laptop in the project `shop`, `where` it runs. */
async function startInShop(
  page: Page,
  where: "Local" | "New worktree",
  prompt: string,
): Promise<void> {
  const box = await newSessionPage(page, "laptop", "Structured");
  await choose(
    page,
    page.getByRole("combobox", { name: "Project" }),
    /^shop\b/,
  );
  if (where === "New worktree") {
    await choose(
      page,
      page.getByRole("combobox", { name: "Where it runs" }),
      /^New worktree/,
    );
  }
  await box.getByRole("textbox", { name: "Prompt" }).fill(prompt);
  await box.getByRole("button", { name: "Send" }).click();
}

test("a session starts in a new worktree once its commands are approved, setup shown as it runs", async ({
  page,
}) => {
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
  await chip.click();
  await expect(tab(page, "Changes")).toHaveAttribute("aria-selected", "true");
  await expect(page.getByText("No changes against base.")).toBeVisible();
});

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
  await tab(page, "Changes").click();
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
  await tab(page, "Files").click();
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
  const conflict = page.getByRole("alert");
  await expect(conflict).toContainText("File changed on disk.");
  // Both states: what the edit read, and what is there now.
  await expect(conflict).toContainText("Read: 26 B");
  await expect(conflict).toContainText("On disk now: 44 B");
  expect(readFileSync(file, "utf8")).toContain("from elsewhere");
  await expect(editor).toHaveValue("export const RATE = 0.2;\n");
  await page.getByRole("button", { name: "Reload from disk" }).click();
  await expect(page.locator(".f-line").first()).toContainText("from elsewhere");
});

test("a shown file follows the disk, and an open edit keeps its text under Changed on disk", async ({
  page,
}) => {
  const env = loadEnv();
  const file = join(env.root, "repos", "shop", "notes", "todo.md");
  const before = readFileSync(file, "utf8");
  try {
    await openUi(page);
    await startInShop(page, "Local", "Watch a file");
    await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
    await tab(page, "Files").click();
    await page.getByRole("searchbox", { name: "Go to file" }).fill("todo");
    await page
      .getByRole("list", { name: "Files found" })
      .getByRole("button", { name: /todo\.md/ })
      .click();
    const first = page.locator(".f-line").first();
    await expect(first).toContainText("pay with a card");
    // Changed elsewhere while shown: the view follows, nothing pressed.
    writeFileSync(file, "- pay by bank transfer\n");
    await expect(first).toContainText("pay by bank transfer");
    // An open edit keeps its text; the page says the disk moved.
    await page.getByRole("button", { name: "Edit", exact: true }).click();
    const editor = page.getByRole("textbox", { name: "Edit notes/todo.md" });
    await editor.fill("- mine\n");
    writeFileSync(file, "- pay in cash\n");
    await expect(
      page.getByRole("status").filter({ hasText: "Changed on disk" }),
    ).toBeVisible();
    await expect(editor).toHaveValue("- mine\n");
    expect(readFileSync(file, "utf8")).toBe("- pay in cash\n");
    // Leaving the edit shows what is on disk now.
    await page.getByRole("button", { name: "Cancel", exact: true }).click();
    await expect(first).toContainText("pay in cash");
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
  await tab(page, "Changes").click();
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

test("projects are added from Settings, and a failed clone says why", async ({
  page,
}) => {
  const env = loadEnv();
  const folder = join(env.root, "repos", "added");
  mkdirSync(folder, { recursive: true });
  writeFileSync(join(folder, "README.md"), "# Added\n");
  const parent = join(env.root, "clones");
  mkdirSync(parent, { recursive: true });
  const origin = await openUi(page);
  await page.getByRole("button", { name: "Add project" }).click();
  await expect(
    page.getByRole("heading", { name: "Projects on laptop" }),
  ).toBeVisible();
  // A folder.
  await page.getByRole("button", { name: "Add project" }).last().click();
  await page.getByRole("menuitem", { name: "Add a folder…" }).click();
  const add = page.getByRole("dialog", { name: "Add a folder" });
  await add.getByRole("textbox", { name: "Folder" }).fill(folder);
  await add.getByRole("button", { name: "Add" }).click();
  const rows = page.getByRole("list", { name: "Projects" });
  await expect(
    rows.getByRole("button", { name: "added", exact: true }),
  ).toBeVisible();
  // A clone of nothing: it starts, then fails, and the page says why.
  await page.getByRole("button", { name: "Add project" }).last().click();
  await page.getByRole("menuitem", { name: "Clone a repository…" }).click();
  const clone = page.getByRole("dialog", { name: "Clone a Git repository" });
  await clone
    .getByRole("textbox", { name: "Repository URL" })
    .fill("http://127.0.0.1:9/nothing.git");
  await clone.getByRole("textbox", { name: "Clone into" }).fill(parent);
  await expect(clone).toContainText(`${parent}/nothing`);
  await clone.getByRole("button", { name: "Clone" }).click();
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
