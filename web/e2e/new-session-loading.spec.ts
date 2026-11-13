// The new-session page before the machines have listed their agents
// (seen on the preview, 2026-11-13): opened by its address, it said "This
// machine has no agent installed." for about a second, until the list
// came. It says the agents are loading, and that there is none only once
// the list has come and holds none. The bridge's answers to the agents'
// list are held back here for a few seconds.

import { expect, test } from "@playwright/test";

import { openUi } from "./open";

/** How long the agents' lists are held back. */
const HELD_MS = 3_000;

test("the new-session page says the agents are loading, not missing, until they are listed", async ({
  page,
}) => {
  const held = new Set<number>();
  let holding = true;
  await page.routeWebSocket(/\/api\/ws$/, (socket) => {
    const server = socket.connectToServer();
    socket.onMessage((message) => {
      const text = typeof message === "string" ? message : "";
      if (holding && text.includes('"harness.list"')) {
        const frame: unknown = JSON.parse(text);
        if (
          typeof frame === "object" &&
          frame !== null &&
          "id" in frame &&
          typeof frame.id === "number"
        ) {
          held.add(frame.id);
        }
      }
      server.send(message);
    });
    server.onMessage((message) => {
      const text = typeof message === "string" ? message : "";
      const frame: unknown = text === "" ? null : JSON.parse(text);
      const id =
        typeof frame === "object" &&
        frame !== null &&
        "id" in frame &&
        typeof frame.id === "number"
          ? frame.id
          : null;
      if (id !== null && held.has(id)) {
        setTimeout(() => {
          socket.send(message);
        }, HELD_MS);
        return;
      }
      socket.send(message);
    });
  });
  await openUi(page);
  await expect(
    page.getByRole("heading", { name: "What do you want to work on?" }),
  ).toBeVisible();
  await expect(page.getByText("Loading the machine's agents…")).toBeVisible();
  await expect(
    page.getByText("This machine has no agent installed."),
  ).toHaveCount(0);
  expect(held.size).toBeGreaterThan(0);
  holding = false;
  // Listed: the agent's control, and no word of loading or of none.
  await expect(
    page.getByRole("button", { name: /^Agent, model and effort/ }),
  ).toBeVisible({ timeout: HELD_MS + 15_000 });
  await expect(page.getByText("Loading the machine's agents…")).toHaveCount(0);
  await expect(
    page.getByText("This machine has no agent installed."),
  ).toHaveCount(0);
});
