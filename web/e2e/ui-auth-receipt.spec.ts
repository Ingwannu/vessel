// The same readiness predicate used before the history/reload assertions,
// exercised with a real authenticated bridge whose welcome is held back.
import { expect, test } from "@playwright/test";
import { launchUi } from "./open";
import { MAX_WELCOME_HOLD_MS, slowWelcome } from "./slow-welcome";
import { waitForTabSession } from "./tab-session";

test("sidebar paint precedes a delayed verified tab-session receipt", async ({
  page,
}) => {
  const delay = await slowWelcome(page);
  try {
    const { url } = await launchUi(page);
    try {
      await page.goto(url);
    } catch {
      throw new Error("UI navigation failed");
    }
    await expect(
      page.getByRole("navigation", { name: "Sidebar" }),
    ).toBeVisible();
    await expect
      .poll(delay.isHeld, { timeout: MAX_WELCOME_HOLD_MS })
      .toBe(true);
    let received = false;
    const receipt = waitForTabSession(page).then(
      () => {
        received = true;
        return true;
      },
      () => false,
    );
    expect(
      await page.evaluate(
        () => sessionStorage.getItem("vessel.session") === null,
      ),
    ).toBe(true);
    expect(received).toBe(false);
    expect(delay.didOverrun()).toBe(false);
    delay.release();
    expect(await receipt).toBe(true);
    expect(received).toBe(true);
    expect(delay.transportFailed()).toBe(false);
    expect(delay.didOverrun()).toBe(false);
  } finally {
    delay.release();
  }
});
