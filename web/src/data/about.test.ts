import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { BUNDLED } from "./about";

/** The `dependencies` of a package.json, read as data. */
function dependenciesOf(path: string): string[] {
  const parsed: unknown = JSON.parse(readFileSync(path, "utf8"));
  if (
    typeof parsed !== "object" ||
    parsed === null ||
    !("dependencies" in parsed) ||
    typeof parsed.dependencies !== "object" ||
    parsed.dependencies === null
  ) {
    return [];
  }
  return Object.keys(parsed.dependencies);
}

/** The `license` of an installed package. */
function licenceOf(name: string): string {
  const parsed: unknown = JSON.parse(
    readFileSync(
      join(import.meta.dirname, "../../node_modules", name, "package.json"),
      "utf8",
    ),
  );
  return typeof parsed === "object" &&
    parsed !== null &&
    "license" in parsed &&
    typeof parsed.license === "string"
    ? parsed.license
    : "";
}

describe("the licences About lists", () => {
  it("name every runtime dependency, in package.json's order", () => {
    expect(BUNDLED.map((entry) => entry.name)).toEqual(
      dependenciesOf(join(import.meta.dirname, "../../package.json")),
    );
  });

  it("are the licences the packages declare", () => {
    for (const entry of BUNDLED) {
      expect(entry.licence, entry.name).toBe(licenceOf(entry.name));
    }
  });
});
