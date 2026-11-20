import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { ICON_NAMES } from "./icon";

/** The kit's icon stylesheets (`icon.css` and its continuation). */
function stylesheets(): string {
  return ["icon.css", "icon-m12.css"]
    .map((name) => readFileSync(join(import.meta.dirname, name), "utf8"))
    .join("\n");
}

/** The names the stylesheets draw, in the order their rules come. */
function drawn(): string[] {
  const css = stylesheets();
  return [...css.matchAll(/^\.ico-([\w-]+) \{\n {2}--ico: url\(/gm)].map(
    (match) => match[1] ?? "",
  );
}

/** Each drawing's path data, to find two names drawing the same thing. */
function drawings(): string[] {
  const css = stylesheets();
  return [...css.matchAll(/^ {2}--ico: url\("([^"]+)"\);$/gm)].map(
    (match) => match[1] ?? "",
  );
}

describe("the icon set", () => {
  it("draws every name it lists, and lists every drawing", () => {
    expect([...drawn()].sort()).toEqual([...ICON_NAMES].sort());
  });

  it("has one name per drawing", () => {
    const all = drawings();
    expect(new Set(all).size).toBe(all.length);
  });
});
