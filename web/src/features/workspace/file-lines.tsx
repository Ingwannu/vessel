// A file's text as numbered lines in its syntax colours, windowed as the
// timeline is (`useWindow`, read from its top): only the lines near the
// view exist, the next page is read when the reader nears the end, and a
// line asked for (a search result) is put in the middle of the view once
// the pages up to it are read (they are asked for until it is).

import { useEffect, useMemo } from "react";

import { lineStates } from "../../data/workspace/syntax";
import type { Language } from "../../data/workspace/syntax-languages";
import type { SessionId } from "../../wire/proto";
import { useWindow } from "../session/use-window";
import { WindowRow } from "../session/window-row";
import type { Target } from "../session/window-pin";
import { CodeText } from "./code-text";
import { SelectionAsk } from "./selection-ask";
import { useSelectionAsk } from "./use-selection";

/** How close to the last drawn line the next page is read, in lines. */
const NEAR = 200;

export function FileLines({
  session,
  path,
  lines,
  language,
  hasMore,
  onMore,
  goTo,
}: {
  session: SessionId;
  path: string;
  lines: readonly string[];
  language: Language;
  hasMore: boolean;
  onMore: () => void;
  /** A line to show, from 1, with a number that changes per request. */
  goTo: { line: number; seq: number } | null;
}) {
  const items = useMemo(
    () => lines.map((_, index) => ({ key: String(index) })),
    [lines],
  );
  // What the line before each leaves open (a block comment), read once
  // per page in order.
  const states = useMemo(() => lineStates(lines, language), [lines, language]);
  // The window takes a target once, so it is given only when its line is
  // read; until then the pages up to it are asked for (below).
  const wanted = goTo === null ? 0 : goTo.line;
  const reached = goTo !== null && goTo.line <= lines.length;
  const target = useMemo(
    (): Target | null =>
      goTo === null || !reached
        ? null
        : {
            key: String(goTo.line - 1),
            seq: goTo.seq,
            align: "center",
            show: (row) => {
              row.setAttribute("data-target", "");
              return row;
            },
          },
    [goTo, reached],
  );
  const windowed = useWindow(items, target, { follow: false });
  const { scroller, span } = windowed;
  // Asked again after each page (`lines` is a new array then), so a page
  // that ends inside a long first line does not stop the reading.
  useEffect(() => {
    if (hasMore && (span.end > lines.length - NEAR || wanted > lines.length)) {
      onMore();
    }
  }, [hasMore, span.end, lines, wanted, onMore]);
  const fence = useMemo(
    () => () => (language.name === "text" ? "" : language.name),
    [language],
  );
  const selection = useSelectionAsk(scroller, session, fence);
  return (
    <div className="file-scroll" ref={scroller}>
      <section className="file-lines" aria-label={`Contents of ${path}`}>
        <div style={{ height: span.before }} />
        {lines.slice(span.start, span.end).map((line, offset) => {
          const index = span.start + offset;
          return (
            <WindowRow
              key={index}
              id={String(index)}
              last={index === lines.length - 1}
              rows={windowed.rows}
            >
              <div
                className="f-line"
                data-path={path}
                data-kind="file"
                data-new={index + 1}
              >
                <span className="d-num" aria-hidden="true">
                  {index + 1}
                </span>
                <span className="f-code">
                  <CodeText
                    text={line}
                    language={language}
                    state={states[index] ?? "code"}
                  />
                </span>
              </div>
            </WindowRow>
          );
        })}
        <div style={{ height: span.after }} />
        {hasMore ? <p className="d-note">Reading more of this file…</p> : null}
      </section>
      {selection.ask === null ? null : (
        <SelectionAsk
          ask={selection.ask}
          onSend={selection.send}
          onDismiss={selection.dismiss}
        />
      )}
    </div>
  );
}
