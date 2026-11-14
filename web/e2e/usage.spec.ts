// A real bridge/node ledger, fed by the existing testkit's ACP response.
// A fake login or a vendor request is not involved in usage.query.
import { expect, type Page, test } from "@playwright/test";

import { cli } from "./fleet-home";
import { loadEnv } from "./fleet";
import { choose, startSession, timeline } from "./kit";
import { openUi } from "./open";
import { watchUsage } from "./usage-observer";

function row(page: Page, label: string) {
  return page
    .getByRole("list", { name: "Reported usage", exact: true })
    .getByRole("listitem")
    .filter({ has: page.getByText(label, { exact: true }) });
}

function formatted(value: number): string {
  return new Intl.NumberFormat("en-US", {
    notation: "compact",
    maximumFractionDigits: 1,
  }).format(value);
}

async function openUsage(
  page: Page,
  seen: ReturnType<typeof watchUsage>,
): Promise<string> {
  await openUi(page);
  await startSession(page, "gpu-box", "Structured", "usage-reasoning");
  await expect(
    timeline(page).getByText(
      /Turn ended · end turn · 30 tokens · 20 in · 10 out/,
    ),
  ).toBeVisible();
  const session = new URL(page.url()).pathname.split("/").at(-1);
  if (session === undefined || session === "")
    throw new Error("Expected a session route");
  await page.getByRole("link", { name: "Settings", exact: true }).click();
  await page.getByRole("link", { name: "Usage", exact: true }).click();
  await choose(
    page,
    page.getByRole("combobox", { name: "Machine", exact: true }),
    /^gpu-box$/,
  );
  await expect(page).toHaveURL(/\/settings\/usage\?machine=gpu-box$/);
  await expect(page).toHaveTitle("Vessel");
  await expect(
    page.getByRole("heading", { name: "Reported totals", exact: true }),
  ).toBeVisible();
  await expect
    .poll(() => seen.selection)
    .toEqual({ node: "gpu-box", period: "day" });
  await expect
    .poll(() => seen.report?.totals.tokens?.input)
    .toBeGreaterThanOrEqual(20);
  const tokens = seen.report?.totals.tokens;
  if (tokens === undefined || tokens === null)
    throw new Error("Expected reported node totals");
  await expect(row(page, "Input tokens").locator(".fact-value")).toHaveText(
    formatted(tokens.input),
  );
  await expect(row(page, "Reasoning tokens").locator(".fact-value")).toHaveText(
    "0",
  );
  await expect(
    row(page, "Cache write tokens").locator(".fact-value"),
  ).toHaveText("0");
  const replies = seen.replies;
  await choose(
    page,
    page.getByRole("combobox", { name: "Usage period" }),
    /^Past 7 days$/,
  );
  await expect.poll(() => seen.replies).toBeGreaterThan(replies);
  await expect
    .poll(() => seen.selection)
    .toEqual({ node: "gpu-box", period: "week" });
  await expect(
    page.getByRole("button", { name: "Refresh", exact: true }),
  ).toBeEnabled();
  return session;
}

async function refreshTurn(
  page: Page,
  seen: ReturnType<typeof watchUsage>,
  session: string,
): Promise<void> {
  const priorInput = seen.report?.totals.tokens?.input;
  const priorCache = seen.report?.totals.tokens?.cached_read;
  if (
    priorInput === undefined ||
    priorCache === undefined ||
    priorCache === null
  )
    throw new Error("Expected cached token reports");
  // Add an actual second turn while the page remains on Usage. Poll only
  // a boolean from the CLI's accounting result, never log a transcript.
  const env = loadEnv();
  cli(env, ["--node", "gpu-box", "send", session, "usage-reasoning"]);
  await expect
    .poll(() =>
      cli(env, ["--node", "gpu-box", "usage", "session", session]).includes(
        "reported turns: 2; unreported turns: 0",
      ),
    )
    .toBe(true);
  const refreshed = seen.replies;
  await page.getByRole("button", { name: "Refresh", exact: true }).click();
  await expect.poll(() => seen.replies).toBeGreaterThan(refreshed);
  await expect
    .poll(() => seen.report?.totals.tokens?.input)
    .toBe(priorInput + 20);
  await expect(row(page, "Input tokens").locator(".fact-value")).toHaveText(
    formatted(priorInput + 20),
  );
  await expect(
    row(page, "Cache read tokens").locator(".fact-value"),
  ).toHaveText(formatted(priorCache + 12));
  await expect(
    page.getByRole("list", { name: "Accounting coverage" }),
  ).not.toContainText("T12:00:00Z");
  await expect(page.locator("vite-error-overlay")).toHaveCount(0);
}

test("Usage reads the selected machine's ledger, changes interval and refreshes real reports", async ({
  page,
}) => {
  const seen = watchUsage(page);
  let errors = 0;
  page.on("pageerror", () => {
    errors += 1;
  });
  page.on("console", (message) => {
    if (message.type() === "error") errors += 1;
  });
  const session = await openUsage(page, seen);
  await refreshTurn(page, seen, session);
  expect(errors).toBe(0);
});
