// Force layout's scroll before resize delivery on the real timeline.
// The original phone regression and its deadlines stay unchanged.
import { expect, test } from "@playwright/test";

import { startSession } from "./kit";
import { openUi } from "./open";
import {
  expectWaitingAboveDock,
  leaveEndThenResize,
  resizeBeforeDelivery,
} from "./window-layout-flow";

test("layout's scroll before resize delivery preserves end intent, while an upward reader stays put", async ({
  page,
}) => {
  await openUi(page);
  await startSession(page, "laptop", "Structured", "ask claude");
  await expect(
    page.getByRole("region", { name: "Question from the agent" }),
  ).toBeVisible();
  await resizeBeforeDelivery(page);
  await expectWaitingAboveDock(page);
  await leaveEndThenResize(page);
});
