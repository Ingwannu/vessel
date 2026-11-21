// The card over the composer's suggestion list (TRACKER 834, Paseo's
// `detailCard`): drawn for the active command when it says what it does,
// following the active row, and kept out of the accessibility tree because
// the text already names the active option.

import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { type SuggestionItem, Suggestions } from "./suggestions";

const review: SuggestionItem = {
  key: "review",
  label: "/review",
  detail: "Review the changes · <optional focus>",
  card: {
    title: "/review",
    description: "Review the changes",
    hint: "<optional focus>",
  },
};

const compact: SuggestionItem = {
  key: "compact",
  label: "/compact",
  detail: "Clear history but keep a summary",
  card: { title: "/compact", description: "Clear history but keep a summary" },
};

function shown(items: readonly SuggestionItem[], active: number): string {
  return renderToStaticMarkup(
    <Suggestions
      id="test-list"
      label="Commands"
      items={items}
      active={active}
      onPick={() => undefined}
      onHover={() => undefined}
    />,
  );
}

describe("the suggestion list's command card", () => {
  it("says the active command's name, what it does and its input, before the list", () => {
    const html = shown([review, compact], 0);
    const card = html.indexOf('class="suggestion-card popup"');
    const list = html.indexOf('role="listbox"');
    expect(card).toBeGreaterThanOrEqual(0);
    expect(card).toBeLessThan(list);
    expect(html).toContain(
      '<div class="suggestion-card popup" aria-hidden="true"><p class="suggestion-card-title">/review</p><p class="suggestion-card-description">Review the changes</p><p class="suggestion-card-hint">&lt;optional focus&gt;</p></div>',
    );
  });

  it("follows the active row and leaves out an input the command does not take", () => {
    const html = shown([review, compact], 1);
    expect(html).toContain(
      '<p class="suggestion-card-title">/compact</p><p class="suggestion-card-description">Clear history but keep a summary</p></div>',
    );
    expect(html).not.toContain("suggestion-card-hint");
  });

  it("is not drawn for a row without one or a command that says nothing", () => {
    const file: SuggestionItem = { key: "a.rs", label: "a.rs", detail: "src" };
    const silent: SuggestionItem = {
      key: "x",
      label: "/x",
      card: { title: "/x", description: "" },
    };
    expect(shown([file], 0)).not.toContain("suggestion-card");
    expect(shown([silent], 0)).not.toContain("suggestion-card");
  });
});
