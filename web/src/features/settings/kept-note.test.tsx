import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { keptNote } from "../../data/prefs";
import { KeptNoteView } from "./kept-note";

describe("where Settings keeps its choices (TRACKER 820)", () => {
  it("says a choice kept in memory as a warning Alert in the flow", () => {
    const note = keptNote({ kind: "this_launch", why: { kind: "no_place" } });
    expect(note).not.toBeNull();
    const html = renderToStaticMarkup(<KeptNoteView note={note ?? ""} />);
    expect(html).toContain('role="status"');
    expect(html).toContain("alert-sm");
    expect(html).toContain("alert-warning");
    expect(html).toContain(
      "Kept until vessel ui stops: this machine gives the page no place to keep them.",
    );
  });
});
