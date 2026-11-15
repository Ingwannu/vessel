// @ts-check
// The kit's tokens, not literals (docs/QUALITY.md, M8; this README's
// "Rules every screen follows"): a screen in `app/src` takes its spacing
// from `kit::theme::spacing`, and no crate of this workspace writes a hex
// colour outside the kit's tokens (`kit/src/theme.rs`, `kit/src/theme/`).
// What was there on 2026-11-14 is listed in `scripts/tokens-allow.txt`
// per file and kind, and may only fall (../../../scripts/ratchet.mjs).
//
// Usage, from spikes/native: `node scripts/check-tokens.mjs`; `--list`
// prints today's list.
//
// [Decision Log]
// - Purpose: the README's rule ("colours, type, spacing and radii from the
//   kit's tokens, never a literal in a screen") was held by review alone;
//   the 2026-11-14 audit counted 511 `px(<number>)` literals in `app/src`,
//   most of them equal to a spacing step, against 54 uses of the steps.
// - Kinds, counted per file:
//   - `px-step`: `px(<number>)` whose number is a step of the kit's
//     spacing scale (read from `kit/src/theme/spacing.rs`, so a new step
//     counts from the day it exists): `px(spacing::S2)` says it.
//   - `px-other`: any other `px(<number>)` in a screen: a size the kit has
//     no token for yet; it may not grow either, and becomes a token or a
//     named constant with its reason.
//   - `hex`: a hex colour (`0x` with six or eight digits, or a `"#rrggbb"`
//     string) in product code outside the kit's tokens.
// - Not counted: test code, comments, `px` of an expression or a named
//   constant (a named constant states its reason where it is defined).
// - Why per file and not one total: a total lets a new screen add literals
//   while an old one loses them; per file, a new file starts at zero.
// - Behaviour the gate relies on: nothing written, output sorted, one line
//   per offence, exit 1 on any offence, 2 when it cannot run.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { compare, initialList } from "../../../scripts/ratchet.mjs";
import { mask, productLines, shape } from "../../../scripts/rust-source.mjs";
import { Stop, root, run } from "../../../scripts/workspace.mjs";

const WS = "spikes/native";
const SCREENS = `${WS}/app/src/`;
const TOKENS = [`${WS}/kit/src/theme.rs`, `${WS}/kit/src/theme/`];
const SPACING = `${WS}/kit/src/theme/spacing.rs`;
const LIST = `${WS}/scripts/tokens-allow.txt`;

/** The spacing scale's values, from the kit's own constants. */
/** @returns {Set<number>} */
function steps() {
  const text = readFileSync(join(root, SPACING), "utf8");
  const values = [
    ...text.matchAll(/pub const S[0-9_]+\s*:\s*f32\s*=\s*(\d+(?:\.\d+)?)\s*;/g),
  ].map((m) => Number(m[1]));
  if (values.length === 0) {
    throw new Stop(`${SPACING}: no \`pub const S…: f32\` steps found`);
  }
  return new Set(values);
}

run("tokens", () => {
  const scale = steps();
  /** @type {Map<string, number>} */
  const counts = new Map();
  const bump = (/** @type {string} */ key) => {
    counts.set(key, (counts.get(key) ?? 0) + 1);
  };
  const files = productLines(root, [
    `${WS}/view`,
    `${WS}/link`,
    `${WS}/kit`,
    `${WS}/app`,
  ]);
  for (const path of files.keys()) {
    const text = readFileSync(join(root, path), "utf8");
    const tests = shape(text).testLineSet;
    const code = mask(text).split("\n");
    const raw = text.split("\n");
    const screen = path.startsWith(SCREENS);
    const token = TOKENS.some((t) => path === t || path.startsWith(t));
    for (const [index, line] of code.entries()) {
      if (tests.has(index)) {
        continue;
      }
      if (screen) {
        for (const m of line.matchAll(
          /(?<![A-Za-z0-9_])px\(\s*(-?\d+(?:\.\d*)?)(?:_?f32)?\s*\)/g,
        )) {
          bump(`${scale.has(Number(m[1])) ? "px-step" : "px-other"} ${path}`);
        }
      }
      if (!token) {
        for (const m of line.matchAll(/(?<![A-Za-z0-9_])0x([0-9A-Fa-f_]+)/g)) {
          const digits = (m[1] ?? "").replaceAll("_", "").length;
          if (digits === 6 || digits === 8) {
            bump(`hex ${path}`);
          }
        }
        // A colour in a string: the masked line blanks strings, so read
        // the raw line, where the masked one shows a blank (a literal, not
        // a comment, which the masked line also blanks: accepted, as a
        // quoted colour in a comment is rare and would be a pointer to fix).
        for (const m of (raw[index] ?? "").matchAll(
          /"#(?:[0-9A-Fa-f]{6}|[0-9A-Fa-f]{8})"/g,
        )) {
          if (line.charAt(m.index) === " ") {
            bump(`hex ${path}`);
          }
        }
      }
    }
  }
  if (process.argv.includes("--list")) {
    process.stdout.write(initialList(counts, 0));
    return;
  }
  const { problems, listed } = compare(
    {
      list: LIST,
      budget: 0,
      unit: "literals",
      fix: "use the kit's tokens (kit::theme, kit::theme::spacing)",
    },
    counts,
  );
  /** @type {Record<string, number>} */
  const totals = { "px-step": 0, "px-other": 0, hex: 0 };
  for (const [key, count] of counts) {
    const kind = key.split(" ")[0] ?? "";
    totals[kind] = (totals[kind] ?? 0) + count;
  }
  const summary = Object.entries(totals)
    .map(([kind, count]) => `${kind} ${String(count)}`)
    .join(", ");
  if (problems.length > 0) {
    process.stderr.write(`tokens (${WS}):\n${problems.join("\n")}\n`);
    process.exitCode = 1;
  } else {
    process.stdout.write(
      `tokens (${WS}): ${summary}, in ${String(listed)} listed file entries, none new\n`,
    );
  }
});
