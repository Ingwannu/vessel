// The CLI creates two schedules on different fake nodes. The page must
// aggregate them, navigate through its normal entry points and send row
// actions to the machine that owns the id.
import { expect, test } from "@playwright/test";
import { loadEnv } from "./fleet";
import { cli } from "./fleet-home";
import { openUi } from "./open";

test("lists schedules across machines, filters, runs and removes a schedule", async ({
  page,
}) => {
  const env = loadEnv();
  const laptop = "Scheduled fixture on laptop";
  const gpu = "Scheduled fixture on GPU";
  const create = (node: string, name: string) => {
    cli(env, [
      "--node",
      node,
      "schedule",
      "create",
      "--name",
      name,
      "--harness",
      "claude-code",
      "--every",
      "1d",
      "--prompt",
      "test schedule run",
      "--max-runs",
      "1",
    ]);
  };
  create("laptop", laptop);
  create("gpu-box", gpu);
  try {
    await openUi(page);
    await page.getByRole("link", { name: "Schedules", exact: true }).click();
    await expect(page).toHaveURL(/\/schedules$/);
    await page.reload();
    const list = page.getByRole("list", { name: "Schedules" });
    const laptopRow = list.getByRole("listitem").filter({ hasText: laptop });
    const gpuRow = list.getByRole("listitem").filter({ hasText: gpu });
    await expect(laptopRow).toBeVisible();
    await expect(gpuRow).toBeVisible();

    await page.getByRole("button", { name: "Filter: All hosts" }).click();
    await page.getByRole("menuitemradio", { name: "laptop" }).click();
    await expect(laptopRow).toBeVisible();
    await expect(gpuRow).toBeHidden();

    const menu = () =>
      laptopRow.getByRole("button", { name: `Actions for ${laptop}` }).click();
    await menu();
    await page.getByRole("menuitem", { name: "Pause schedule" }).click();
    await expect(laptopRow.getByText("Paused", { exact: true })).toBeVisible();
    await menu();
    await page.getByRole("menuitem", { name: "Resume schedule" }).click();
    await expect(laptopRow.getByText("Active", { exact: true })).toBeVisible();
    await menu();
    await page.getByRole("menuitem", { name: "Run now" }).click();
    await expect(laptopRow).toBeHidden();
    await page.getByRole("radio", { name: "Ended" }).click();
    await expect(
      laptopRow.getByText("Finished", { exact: true }),
    ).toBeVisible();

    await menu();
    await page.getByRole("menuitem", { name: "Delete schedule" }).click();
    const confirm = page.getByRole("dialog", { name: "Delete schedule" });
    await expect(confirm).toContainText(laptop);
    await confirm.getByRole("button", { name: "Delete", exact: true }).click();
    await expect(laptopRow).toBeHidden();
    await expect(
      page.getByRole("heading", { name: "No ended schedules" }),
    ).toBeVisible();
  } finally {
    // Delete by name so a failed UI action does not leak a fixture.
    for (const [node, name] of [
      ["laptop", laptop],
      ["gpu-box", gpu],
    ] as const) {
      if (cli(env, ["--node", node, "schedule", "list"]).includes(name)) {
        cli(env, ["--node", node, "schedule", "delete", name]);
      }
    }
  }
});
