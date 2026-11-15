// Existing usage.query agent/profile scopes against actual fake-node reports.
// Seed only the isolated fleet; every agent here is the testkit ACP binary.
import { expect, type Page, test } from "@playwright/test";
import type { RollingUsageScope } from "../src/data/usage/scope";
import type { UsageReport } from "../src/wire/proto";
import { loadEnv } from "./fleet";
import { cli } from "./fleet-home";
import { choose } from "./kit";
import { openUi } from "./open";
import { watchPageErrors } from "./page-errors";
import { watchUsage } from "./usage-observer";

async function seedUsage(profile: string | null): Promise<void> {
  const env = loadEnv();
  const selected = profile === null ? [] : ["--profile", profile];
  const session = cli(env, [
    "--node",
    "laptop",
    "run",
    "claude",
    "--structured",
    "--detach",
    ...selected,
  ]).trim();
  if (session === "" || /\s/.test(session))
    throw new Error("Expected a session id");
  cli(env, ["--node", "laptop", "send", session, "usage-reasoning"]);
  await expect
    .poll(() =>
      cli(env, ["--node", "laptop", "usage", "session", session]).includes(
        "reported turns: 1; unreported turns: 0",
      ),
    )
    .toBe(true);
}

async function selectedReport(
  page: Page,
  seen: ReturnType<typeof watchUsage>,
  scope: RollingUsageScope,
): Promise<UsageReport> {
  await expect.poll(() => seen.reportedScope).toEqual(scope);
  await expect(
    page.getByRole("button", { name: "Refresh", exact: true }),
  ).toBeEnabled();
  const report = seen.report;
  if (report === null) throw new Error("Expected a node usage report");
  return report;
}

async function expectInput(page: Page, report: UsageReport): Promise<number> {
  if (report.totals.tokens === null)
    throw new Error("Expected node token counts");
  const input = report.totals.tokens.input;
  const row = page
    .getByRole("list", { name: "Reported usage", exact: true })
    .getByRole("listitem")
    .filter({ has: page.getByText("Input tokens", { exact: true }) });
  await expect(row.locator(".fact-value")).toHaveText(
    new Intl.NumberFormat("en-US", {
      notation: "compact",
      maximumFractionDigits: 1,
    }).format(input),
  );
  return input;
}

test("Usage selects reported agent/profile totals, retains the scope across periods and resets it on another machine", async ({
  page,
}) => {
  const seen = watchUsage(page);
  const errors = watchPageErrors(page);
  const origin = await openUi(page);
  await seedUsage(null);
  await seedUsage("claude-qwen");
  await page.goto(`${origin}/settings/usage?machine=laptop`);
  const scope = page.getByRole("combobox", { name: "Usage agent or profile" });
  await expect(scope).toBeVisible();
  await choose(page, scope, /^Claude Code · All profiles$/);
  const allScope = {
    kind: "agent",
    harness: "claude-code",
    profile: null,
  } as const;
  const all = await expectInput(
    page,
    await selectedReport(page, seen, allScope),
  );
  expect(all).toBeGreaterThanOrEqual(40);
  await choose(page, scope, /^Claude Code · Profile: claude-qwen$/);
  const namedScope = { ...allScope, profile: "claude-qwen" };
  const named = await expectInput(
    page,
    await selectedReport(page, seen, namedScope),
  );
  expect(named).toBeGreaterThanOrEqual(20);
  expect(named).toBeLessThan(all);
  const prior = seen.replies;
  await choose(
    page,
    page.getByRole("combobox", { name: "Usage period" }),
    /^Past 7 days$/,
  );
  await expect.poll(() => seen.replies).toBeGreaterThan(prior);
  await expect
    .poll(() => seen.selection)
    .toEqual({ node: "laptop", period: "week" });
  await expectInput(page, await selectedReport(page, seen, namedScope));
  await choose(
    page,
    page.getByRole("combobox", { name: "Machine", exact: true }),
    /^gpu-box$/,
  );
  await expect
    .poll(() => seen.selection)
    .toEqual({ node: "gpu-box", period: "week" });
  await selectedReport(page, seen, { kind: "machine" });
  const other = page.getByRole("list", { name: "Usage scope" });
  await expect(other).toContainText("All agents");
  await expect(other).not.toContainText("claude-qwen");
  await choose(
    page,
    page.getByRole("combobox", { name: "Machine", exact: true }),
    /^laptop$/,
  );
  await expect
    .poll(() => seen.selection)
    .toEqual({ node: "laptop", period: "week" });
  await expectInput(
    page,
    await selectedReport(page, seen, { kind: "machine" }),
  );
  await expect(scope).toHaveText("All agents");
  await expect(page.getByRole("combobox", { name: "Usage period" })).toHaveText(
    "Past 7 days",
  );
  expect(errors()).toBe(0);
});
