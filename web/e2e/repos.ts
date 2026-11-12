// Real git repositories for the code panel's tests and captures, made in
// the fleet's temporary directory (never under a path of the machine that
// matters) and added as projects of laptop with the CLI, as a person
// would: `shop`, a small TypeScript repository with commits, a branch, a
// `vessel.toml` whose setup prints a few lines, and uncommitted, staged
// and untracked changes; `notes`, a folder without git.

import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

/** Run git in `cwd` with a fixed identity and no user configuration. */
function git(cwd: string, args: string[], env: NodeJS.ProcessEnv): string {
  return execFileSync("git", args, {
    cwd,
    encoding: "utf8",
    env: {
      ...env,
      HOME: cwd,
      GIT_CONFIG_NOSYSTEM: "1",
      GIT_AUTHOR_DATE: "2026-11-10T09:00:00Z",
      GIT_COMMITTER_DATE: "2026-11-10T09:00:00Z",
    },
  });
}

function write(root: string, path: string, text: string): void {
  const full = join(root, path);
  mkdirSync(join(full, ".."), { recursive: true });
  writeFileSync(full, text);
}

/** `count` lines of a module, for a diff long enough to scroll. */
function lines(count: number, word: string, factor: number): string {
  return Array.from(
    { length: count },
    (_, index) =>
      `export const ${word}${String(index)} = ${String(index * factor)}; // ${word} entry ${String(index)}`,
  ).join("\n");
}

/** Make the repositories under `root`; the paths of each. */
export function makeRepos(
  root: string,
  env: NodeJS.ProcessEnv,
): { shop: string; notes: string } {
  const shop = join(root, "repos", "shop");
  const notes = join(root, "repos", "notes");
  mkdirSync(shop, { recursive: true });
  mkdirSync(notes, { recursive: true });
  git(shop, ["init", "-q", "-b", "main"], env);
  git(shop, ["config", "user.name", "Vessel Tests"], env);
  git(shop, ["config", "user.email", "tests@vessel.invalid"], env);
  write(
    shop,
    "README.md",
    "# Shop\n\nA small shop for the code panel's tests.\n",
  );
  write(
    shop,
    "src/cart.ts",
    [
      "// The cart: items and their total.",
      "export interface Item {",
      "  name: string;",
      "  price: number;",
      "}",
      "",
      "export function total(items: readonly Item[]): number {",
      "  return items.reduce((sum, item) => sum + item.price, 0);",
      "}",
      "",
    ].join("\n"),
  );
  write(shop, "src/prices.ts", `${lines(120, "price", 7)}\n`);
  write(
    shop,
    "vessel.toml",
    [
      "[worktree]",
      'setup = ["echo installing packages", "echo setup done"]',
      'teardown = "echo bye"',
      "",
    ].join("\n"),
  );
  git(shop, ["add", "-A"], env);
  git(shop, ["commit", "-q", "-m", "A cart with a total"], env);
  write(shop, "src/tax.ts", "export const RATE = 0.1;\n");
  git(shop, ["add", "-A"], env);
  git(shop, ["commit", "-q", "-m", "Tax rate"], env);
  git(shop, ["branch", "feature/discounts"], env);
  // Work not committed yet: a change, a staged file, a new file.
  write(
    shop,
    "src/cart.ts",
    [
      "// The cart: items, their total and a discount.",
      "export interface Item {",
      "  name: string;",
      "  price: number;",
      "  quantity: number;",
      "}",
      "",
      "export function total(items: readonly Item[]): number {",
      "  return items.reduce((sum, item) => sum + item.price * item.quantity, 0);",
      "}",
      "",
      'export const CURRENCY = "EUR";',
      "",
    ].join("\n"),
  );
  write(shop, "src/prices.ts", `${lines(120, "price", 9)}\n`);
  write(shop, "src/discount.ts", "export const DISCOUNT = 0.15;\n");
  git(shop, ["add", "src/discount.ts"], env);
  write(shop, "notes/todo.md", "- pay with a card\n- receipts\n");
  write(notes, "ideas.md", "# Ideas\n\nA folder that is not a repository.\n");
  return { shop, notes };
}

/** Add `paths` as projects of the default node, with the CLI. */
export function addProjects(
  vessel: string,
  env: NodeJS.ProcessEnv,
  paths: readonly string[],
): void {
  for (const path of paths) {
    execFileSync(vessel, ["project", "add", path], { env });
  }
}
