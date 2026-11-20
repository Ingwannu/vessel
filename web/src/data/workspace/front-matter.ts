// A Markdown file's front matter as Paseo's preview shows it, its
// `file-panels/markdown-preview/document.ts` (pinned 0.15.4): a `---`
// block at the very top, read as YAML with the core schema, becomes
// key/value rows above the body; anything that does not read as a map
// stays part of the body.
//
// [Decision Log]
// - Purpose: F-F38's front matter table, as Paseo draws it.
// - Constraint: the page had no YAML reader.
// - Alternatives: a hand parser for the common `key: value` lines (lists,
//   quotes, multi-line values and anchors would differ from Paseo's).
// - Choice: the `yaml` package, Paseo's own dependency (ISC, no
//   dependencies of its own), called as Paseo calls it.
// - Cost: one package in the page's bundle.

import { parseDocument, stringify } from "yaml";

/** One row of the table. */
export interface FrontMatterRow {
  key: string;
  value: string;
}

/** The table's rows and the body below them. */
export interface MarkdownDocument {
  frontMatter: FrontMatterRow[];
  body: string;
}

const FRONT_MATTER =
  /^\uFEFF?---[ \t]*\r?\n([\s\S]*?)\r?\n---[ \t]*(?:\r?\n|$)/u;

type Scalar = string | number | boolean | null;

function isScalar(value: unknown): value is Scalar {
  return (
    value === null ||
    typeof value === "string" ||
    typeof value === "number" ||
    typeof value === "boolean"
  );
}

function formatScalar(value: Scalar): string {
  return value === null ? "null" : String(value);
}

function formatValue(value: unknown): string {
  if (Array.isArray(value) && value.every(isScalar)) {
    return value.map(formatScalar).join(", ");
  }
  if (isScalar(value)) return formatScalar(value);
  return stringify(value).trim();
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function parsed(yamlText: string): Record<string, unknown> | null {
  try {
    const document = parseDocument(yamlText, { schema: "core" });
    if (document.errors.length > 0) return null;
    const metadata: unknown = document.toJS({ maxAliasCount: 100 });
    return isRecord(metadata) ? metadata : null;
  } catch {
    return null;
  }
}

/** `source` as the preview shows it. */
export function markdownDocument(source: string): MarkdownDocument {
  const match = FRONT_MATTER.exec(source);
  const yamlText = match?.[1];
  if (match === null || yamlText === undefined) {
    return { frontMatter: [], body: source };
  }
  const metadata = parsed(yamlText);
  if (metadata === null) return { frontMatter: [], body: source };
  return {
    frontMatter: Object.entries(metadata).map(([key, value]) => ({
      key,
      value: formatValue(value),
    })),
    body: source.slice(match[0].length),
  };
}
