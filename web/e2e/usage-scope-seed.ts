// Long, valid profile names in the fake fleet's own configuration only.
import { appendFileSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { expect } from "@playwright/test";
import { loadEnv } from "./fleet";
import { cli } from "./fleet-home";

export const LONG_USAGE_PROFILE =
  "usage-profile-historical-with-a-long-recorded-name";

/** Actual node reports under one named profile, never an injected UI report. */
export async function seedNamedUsage(): Promise<void> {
  const env = loadEnv();
  const node = env.nodes["laptop"];
  if (node === undefined) throw new Error("Expected the fake laptop");
  const file = join(node.data, "profiles.toml");
  const heading = `[profiles.${LONG_USAGE_PROFILE}]`;
  if (!readFileSync(file, "utf8").includes(heading)) {
    appendFileSync(file, `\n${heading}\nharness = "claude"\n`);
    cli(env, ["--node", "laptop", "profiles", "--reload"]);
  }
  const session = cli(env, [
    "--node",
    "laptop",
    "run",
    "claude",
    "--structured",
    "--detach",
    "--profile",
    LONG_USAGE_PROFILE,
  ]).trim();
  if (session === "" || /\s/.test(session))
    throw new Error("Expected a session id");
  cli(env, ["--node", "laptop", "send", session, "usage-reasoning"]);
  await expect
    .poll(() =>
      cli(env, ["--node", "laptop", "usage", "session", session]).includes(
        "reported turns: 1; unreported turns: 0",
      ),
    )
    .toBe(true);
}
