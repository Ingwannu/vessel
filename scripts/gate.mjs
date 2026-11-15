// @ts-check
// The local root gate keeps the fixture feature explicit: --workspace
// alone skips its executable and feature-only tests. Prebuild the same
// fakes/product used by tests before their safety clocks start. The
// calling environment, including isolated test storage, stays unchanged.
import { spawnSync } from "node:child_process";
import { root } from "./workspace.mjs";

/** @type {readonly (readonly [string, readonly string[]])[]} */
const steps = [
  ["cargo", ["fmt", "--all", "--", "--check"]],
  [process.execPath, ["scripts/check-rust-size.mjs"]],
  [process.execPath, ["scripts/check-boundaries.mjs"]],
  ["cargo", ["build", "-p", "vessel-testkit", "--bins"]],
  ["cargo", ["build", "-p", "vessel-node", "--bin", "vesseld"]],
  ["cargo", ["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]],
  [
    "cargo",
    [
      "clippy",
      "-p",
      "vessel-node",
      "--features",
      "forge-fixture",
      "--all-targets",
      "--",
      "-D",
      "warnings",
    ],
  ],
  ["cargo", ["test", "--workspace", "--no-fail-fast"]],
  [
    "cargo",
    [
      "test",
      "-p",
      "vessel-node",
      "--features",
      "forge-fixture",
      "--test",
      "forge_fixture",
    ],
  ],
];

for (const [program, args] of steps) {
  process.stdout.write(`gate: ${program} ${args.join(" ")}\n`);
  const result = spawnSync(program, args, {
    cwd: root,
    env: process.env,
    stdio: "inherit",
  });
  if (result.error) {
    process.stderr.write(`gate: could not start ${program}\n`);
    process.exitCode = 1;
    break;
  }
  if (result.status !== 0) {
    process.exitCode = result.status ?? 1;
    break;
  }
}
