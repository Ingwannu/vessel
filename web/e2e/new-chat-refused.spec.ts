// A new session's first prompt that the machine refuses (the new chat,
// 2026-11-13): back in the session's box, with why, not lost.

import { expect, test } from "@playwright/test";

import { newSessionPage, timeline } from "./kit";
import { openUi } from "./open";

test("a first prompt the machine refuses comes back in the session's box, with why", async ({
  page,
}) => {
  // The bridge's answer to the first prompt is replaced by a refusal: the
  // node itself refuses one only in races (the session ended as it came).
  await page.routeWebSocket(/\/api\/ws$/, (socket) => {
    const server = socket.connectToServer();
    socket.onMessage((message) => {
      const text = typeof message === "string" ? message : "";
      const frame: unknown = text === "" ? null : JSON.parse(text);
      if (
        text.includes('"session.send"') &&
        text.includes("refuse this") &&
        typeof frame === "object" &&
        frame !== null &&
        "id" in frame
      ) {
        socket.send(
          JSON.stringify({
            type: "reply",
            id: frame.id,
            outcome: {
              err: {
                kind: "remote",
                error: { code: "conflict", message: "The agent is busy." },
              },
            },
          }),
        );
        return;
      }
      server.send(message);
    });
  });
  await openUi(page);
  const box = await newSessionPage(page, "laptop", "Structured");
  await box.getByRole("textbox", { name: "Prompt" }).fill("refuse this");
  await box.getByRole("button", { name: "Send" }).click();
  await expect(page).toHaveURL(/\/sessions\/laptop\/[^/]+$/);
  const composer = page.getByRole("form", { name: "Prompt" });
  await expect(composer.getByRole("textbox", { name: "Prompt" })).toHaveValue(
    "refuse this",
  );
  // Under the box (a toast says it too, and that the text is back).
  await expect(
    page.getByRole("alert").filter({
      hasText: /^The first message was not sent: The agent is busy\.$/,
    }),
  ).toBeVisible();
  // Not shown as sent.
  await expect(
    timeline(page).getByText("refuse this", { exact: true }),
  ).toHaveCount(0);
});
