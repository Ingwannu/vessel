// A tab whose connection to `vessel ui` keeps dropping stops redialling
// and says so, but keeps its session key: Retry, or a reload, resumes the
// same launch (verification finding 1, M7). The connection is dropped
// by routing the page's websocket, so the UI itself never goes away.

import { expect, test, type WebSocketRoute } from "@playwright/test";

import { openUi } from "./open";

test("a tab that lost its connection recovers with Retry and after a reload", async ({
  page,
}) => {
  let failing = false;
  const live: WebSocketRoute[] = [];
  await page.routeWebSocket(/\/api\/ws$/, (socket) => {
    if (failing) {
      void socket.close();
      return;
    }
    socket.connectToServer();
    live.push(socket);
  });
  const drop = async () => {
    failing = true;
    for (const socket of live.splice(0)) {
      await socket.close();
    }
  };
  // The title bar names the link to `vessel ui` only while it is not up;
  // connected, the bar is quiet and the sidebar's machines serve again.
  const linkChip = page.getByRole("banner").getByText(/^vessel ui · /);
  const serving = page
    .getByRole("navigation", { name: "Sidebar" })
    .getByRole("img", { name: "serving" })
    .first();
  const connected = async () => {
    await expect(linkChip).toHaveCount(0);
    await expect(serving).toBeVisible();
  };
  const disconnected = page.getByText(
    /Disconnected — vessel ui may have restarted/,
  );

  await openUi(page);
  await connected();

  // Five failed dials (about 7.5 s of backoff), then it stops and says so.
  await drop();
  await expect(disconnected).toBeVisible({ timeout: 30_000 });
  await expect(page.getByText("vessel ui · disconnected")).toBeVisible();
  failing = false;
  await page.getByRole("button", { name: "Retry" }).click();
  await connected();
  await expect(disconnected).toBeHidden();
  await expect(page.getByRole("list", { name: "Nodes" })).toBeVisible();

  // Again, and this time a reload: the key in this tab survived.
  await drop();
  await expect(disconnected).toBeVisible({ timeout: 30_000 });
  failing = false;
  await page.reload();
  await connected();
  await expect(page.getByRole("list", { name: "Nodes" })).toBeVisible();
});
