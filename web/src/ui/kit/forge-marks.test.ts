import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

import { FORGE_MARK_PATHS, type ForgeMarkName } from "./forge-marks";

const NAMES: readonly ForgeMarkName[] = [
  "github",
  "gitlab",
  "gitea",
  "forgejo",
  "codeberg",
];

describe("the forges' marks", () => {
  it("are their recorded files' paths, unchanged", () => {
    for (const name of NAMES) {
      const file = readFileSync(
        new URL(
          `../../../../docs/design/forge-marks/${name}.svg`,
          import.meta.url,
        ),
        "utf8",
      );
      expect(file).toContain('viewBox="0 0 24 24"');
      expect(file).toContain(`<path d="${FORGE_MARK_PATHS[name]}"/>`);
    }
  });
});
