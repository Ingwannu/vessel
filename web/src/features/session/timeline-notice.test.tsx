import { renderToStaticMarkup } from "react-dom/server";
import { expect, test } from "vitest";
import type { Entry } from "../../data/timeline-model";
import { TimelineItem } from "./timeline-item";

test("an associated input draws no user bubble while its canonical notice draws once", () => {
  const input: Entry = {
    kind: "prompt",
    key: "r2",
    seq: 2,
    at: "2026-11-16T00:00:00Z",
    text: "",
    turn: 1,
    replayed: false,
    attachments: [],
    internalNotice: 1,
  };
  expect(renderToStaticMarkup(<TimelineItem entry={input} />)).toBe("");
  const notice: Entry = {
    kind: "notification",
    key: "r1",
    seq: 1,
    at: input.at,
    item: {
      event: "notification",
      level: "warning",
      message: "Child finished",
    },
  };
  const html = renderToStaticMarkup(<TimelineItem entry={notice} />);
  expect(html.split("Child finished")).toHaveLength(2);
  expect(html).not.toContain('data-kind="prompt"');
});
