// Count errors for a happy-path fixture without storing console text,
// which may contain connection URLs or session details.
import type { Page } from "@playwright/test";

export function watchPageErrors(page: Page): () => number {
  let errors = 0;
  page.on("pageerror", () => {
    errors += 1;
  });
  page.on("console", (message) => {
    if (message.type() === "error") errors += 1;
  });
  return () => errors;
}
