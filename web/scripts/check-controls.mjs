// @ts-check
// Nothing the browser draws by itself, outside the kit (ADR 0021): no
// `<select>`, `<input>` or `<textarea>` element, and no `title`
// attribute (the browser's own tooltip) on an element, anywhere in
// `src/` but `src/ui/kit/`.
//
// [Decision Log]
// - Purpose: the owner asked that every control be drawn for this design.
//   A native control slips back in with one line; this makes it a lint
//   failure, not a review comment.
// - Choice: TypeScript's own parser over every .tsx file, so a control is
//   found however its JSX is laid out, and a component prop that happens
//   to be called `title` (a Dialog's, an EmptyState's) is not mistaken for
//   the attribute: only intrinsic elements (lowercase tags) and the
//   router's `Link` (which renders an <a>) are checked for `title`.
// - Alternatives: a regular expression (misses a tag split over lines and
//   flags props); an ESLint rule (`react/forbid-elements` needs a plugin
//   the stack does not have, and cannot tell the kit's directory apart
//   without a second config block per rule).
// - What it does not see: an element created with `createElement` by
//   name, which no code here does.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";

import ts from "typescript";

const web = fileURLToPath(new URL("..", import.meta.url));
const src = join(web, "src");
const kit = join(src, "ui", "kit") + sep;

/** The elements the kit replaces. */
const NATIVE = new Set(["select", "input", "textarea"]);

/** Every .tsx file under `path`.
 * @param {string} path
 * @returns {string[]} */
function sources(path) {
  if (statSync(path).isFile()) {
    return path.endsWith(".tsx") ? [path] : [];
  }
  return readdirSync(path).flatMap((name) => sources(join(path, name)));
}

/** @typedef {{ file: string, line: number, message: string }} Finding */

/** What is drawn natively in `file`.
 * @param {string} file
 * @returns {Finding[]} */
function findings(file) {
  const text = readFileSync(file, "utf8");
  const tree = ts.createSourceFile(
    file,
    text,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TSX,
  );
  /** @type {Finding[]} */
  const found = [];
  /** @param {ts.Node} node @param {string} message */
  const report = (node, message) => {
    const { line } = tree.getLineAndCharacterOfPosition(node.getStart(tree));
    found.push({ file, line: line + 1, message });
  };
  /** @param {ts.Node} node */
  const visit = (node) => {
    if (ts.isJsxOpeningElement(node) || ts.isJsxSelfClosingElement(node)) {
      const tag = node.tagName.getText(tree);
      if (NATIVE.has(tag)) {
        report(node, `<${tag}> is the browser's; use the kit's control`);
      }
      const intrinsic = /^[a-z]/.test(tag) || tag === "Link";
      for (const attribute of node.attributes.properties) {
        if (
          intrinsic &&
          ts.isJsxAttribute(attribute) &&
          attribute.name.getText(tree) === "title"
        ) {
          report(
            attribute,
            `title on <${tag}> is the browser's tooltip; use the kit's Tooltip`,
          );
        }
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(tree);
  return found;
}

const all = sources(src)
  .filter((file) => !file.startsWith(kit))
  .flatMap(findings);
if (all.length > 0) {
  process.stderr.write(
    `Native controls outside src/ui/kit/ (ADR 0021):\n${all
      .map(
        (finding) =>
          `${relative(web, finding.file)}:${String(finding.line)}: ${finding.message}`,
      )
      .join("\n")}\n`,
  );
  process.exitCode = 1;
} else {
  process.stdout.write("controls: nothing native outside the kit\n");
}
