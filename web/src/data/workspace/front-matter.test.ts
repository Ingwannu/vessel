import { describe, expect, it } from "vitest";

import { markdownDocument } from "./front-matter";

// Paseo's `markdown-preview/document.test.ts` cases.
describe("a Markdown file's front matter", () => {
  it("separates YAML front matter from the Markdown body", () => {
    expect(
      markdownDocument(`---
name: release-beta
description: Cut a beta release
allowed-tools:
  - Bash
  - Read
---
# Release beta

Ship it.
`),
    ).toEqual({
      frontMatter: [
        { key: "name", value: "release-beta" },
        { key: "description", value: "Cut a beta release" },
        { key: "allowed-tools", value: "Bash, Read" },
      ],
      body: "# Release beta\n\nShip it.\n",
    });
  });

  it("formats nested metadata without flattening its meaning", () => {
    expect(
      markdownDocument(`---
metadata:
  audience: maintainers
  stable: true
---
Body
`),
    ).toEqual({
      frontMatter: [
        { key: "metadata", value: "audience: maintainers\nstable: true" },
      ],
      body: "Body\n",
    });
  });

  it("leaves the document intact when front matter is invalid", () => {
    const source = `---
name: [broken
---
Body
`;
    expect(markdownDocument(source)).toEqual({ frontMatter: [], body: source });
  });

  it("leaves the document intact when YAML refuses excessive aliases", () => {
    const aliases = Array.from({ length: 101 }, () => "*item").join(", ");
    const source = `---
item: &item value
items: [${aliases}]
---
Body
`;
    expect(markdownDocument(source)).toEqual({ frontMatter: [], body: source });
  });

  it("does not take a thematic break inside the document for front matter", () => {
    const source = `Introduction

---

Body
`;
    expect(markdownDocument(source)).toEqual({ frontMatter: [], body: source });
  });
});
