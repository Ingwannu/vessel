// End-to-end tests against a real fleet (e2e/fleet.ts), the `screens`
// project that captures every main screen into artifacts/screens/
// (untracked) for review, and the `perf` project that measures the page
// (e2e/perf.ts, on demand).
import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "e2e",
  outputDir: "test-results",
  globalSetup: "./e2e/global-setup.ts",
  globalTeardown: "./e2e/global-teardown.ts",
  // One fleet, shared: tests run one at a time.
  workers: 1,
  fullyParallel: false,
  timeout: 60_000,
  expect: { timeout: 15_000 },
  reporter: "list",
  use: {
    ...devices["Desktop Chrome"],
    viewport: { width: 1440, height: 900 },
    deviceScaleFactor: 1,
    // The page follows the system's scheme until the operator chooses
    // (data/theme.ts): the tests and the screens run it light, and the
    // screens choose dark where they show it.
    colorScheme: "light",
    // WS frames and navigation URLs can contain credentials. Never record them.
    trace: "off",
    screenshot: "off",
    video: "off",
  },
  projects: [
    { name: "e2e", testMatch: /\.spec\.ts$/ },
    { name: "screens", testMatch: /screens\.ts$/ },
    // `npm run perf` only: the page's speed (e2e/perf.ts); its modules
    // under e2e/perf/ are not tests.
    { name: "perf", testMatch: /[\\/]perf\.ts$/ },
  ],
});
