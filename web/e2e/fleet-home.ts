// The fleet's sync setup for the screens (screens.ts, settings-screens.ts):
// laptop made the home, the other two joined to it, as a person does with
// the CLI.

import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import type { FleetEnv } from "./fleet";

/** Run `vessel` as the user of this fleet would; `data` is the local data
 * directory (a node's own, for the commands that act on this machine's
 * node, such as `fleet init` and `fleet join`). */
export function cli(
  env: FleetEnv,
  args: string[],
  input = "",
  data = join(env.root, "ui-data"),
): string {
  return execFileSync(env.vessel, args, {
    input,
    encoding: "utf8",
    env: {
      PATH: process.env["PATH"],
      HOME: env.root,
      USERPROFILE: env.root,
      VESSEL_CONFIG_DIR: join(env.root, "config"),
      VESSEL_DATA_DIR: data,
    },
  });
}

/** Make laptop the fleet's home, with gpu-box and build-server joined.
 * Once per fleet: screens.ts and settings-screens.ts both need it, and
 * a node cannot be made a home twice, so a marker in the fleet's root
 * says it is done. */
export async function fleetHome(env: FleetEnv): Promise<void> {
  const marker = join(env.root, "fleet-home.done");
  if (existsSync(marker)) {
    return;
  }
  const sources = join(env.root, "sources");
  mkdirSync(join(sources, "skills", "release-notes"), { recursive: true });
  writeFileSync(
    join(sources, "AGENTS.md"),
    "# House rules\n\nSmall commits. Tests first.\n",
  );
  writeFileSync(
    join(sources, "skills", "release-notes", "SKILL.md"),
    "---\nname: release-notes\n---\nWrite them.\n",
  );
  const laptop = env.nodes["laptop"];
  if (laptop === undefined) {
    return;
  }
  cli(
    env,
    [
      "fleet",
      "init",
      "--node",
      "laptop",
      "--instructions",
      join(sources, "AGENTS.md"),
      "--skills",
      join(sources, "skills"),
    ],
    "",
    laptop.data,
  );
  // The home publishes its first generation within a few seconds.
  await new Promise((resolve) => setTimeout(resolve, 3000));
  for (const member of ["gpu-box", "build-server"]) {
    const node = env.nodes[member];
    if (node === undefined) {
      continue;
    }
    const invite = cli(
      env,
      ["fleet", "invite", "--node", "laptop"],
      "",
      laptop.data,
    );
    const link = /vessel:\/\/pair#\S+/.exec(invite)?.[0];
    if (link !== undefined) {
      cli(
        env,
        ["fleet", "join", "--node", member, "--yes"],
        `${link}\n`,
        node.data,
      );
    }
  }
  writeFileSync(marker, "");
}
