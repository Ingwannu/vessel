// A painted shell precedes the bridge's verified welcome. Consumers that
// reload or inspect tab storage must wait for its independent receipt.
import type { Page } from "@playwright/test";

export async function waitForTabSession(page: Page): Promise<void> {
  await page.waitForFunction(
    () => window.sessionStorage.getItem("vessel.session") !== null,
  );
}
