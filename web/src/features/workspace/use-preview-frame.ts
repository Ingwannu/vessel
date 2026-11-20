// Where an HTML file's Preview is framed from (ADR 0122, TRACKER 814):
// the document is posted to `vessel ui` and framed from the address it
// answers, where its scripts run under Paseo's sandboxed preview policy.
// Each panel names itself with this tab's random prefix and React's id, so
// a panel's next document replaces only its own last one, never another
// tab's.
// - While a new document is out, the last ready frame stays (Paseo swaps
//   its frame's document in place); a first one says it is loading.
// - When the post fails (a lost link, an older `vessel ui`) the panel shows
//   the static frame, whose scripts do not run, and posts again when the
//   link opens.
// - While shown, the panel posts its document again every `KEEP_MS`. The
//   same document keeps its address and its idle time starts again, so a
//   frame that reloads itself still finds it.

import { useEffect, useId, useState } from "react";

import { useFleet, useFleetState } from "../../context";
import { framePreview } from "../../data/workspace/html-preview";

/** This tab's prefix for its panels' names (made once, at load). */
const TAB = crypto.randomUUID();

/** How often a shown panel posts its document again: well inside
 * `vessel ui`'s 10 idle minutes. */
export const KEEP_MS = 4 * 60_000;

export type PreviewFrame =
  | { status: "loading" }
  | { status: "ready"; url: string }
  /** The static `srcdoc` frame, scripts off. */
  | { status: "static" };

/** The last post's answer, for the document it posted. */
export interface Answer {
  document: string;
  url: string | null;
}

/** What the panel shows for `document`, given the last answer. */
export function frameOf(answer: Answer | null, document: string): PreviewFrame {
  if (answer === null) return { status: "loading" };
  if (answer.document !== document) {
    // A new document is out: the last ready frame stays meanwhile.
    return answer.url === null
      ? { status: "loading" }
      : { status: "ready", url: answer.url };
  }
  return answer.url === null
    ? { status: "static" }
    : { status: "ready", url: answer.url };
}

export function usePreviewFrame(document: string): PreviewFrame {
  const fleet = useFleet();
  const open = useFleetState((state) => state.link.kind === "open");
  const slot = `${TAB}-${useId().replace(/[^A-Za-z0-9_-]/g, "")}`;
  const [answer, setAnswer] = useState<Answer | null>(null);
  useEffect(() => {
    let current = true;
    const post = () => {
      const placed = open
        ? framePreview(fleet, slot, document)
        : Promise.reject(new Error("Not connected to vessel ui."));
      void placed.then(
        (url) => {
          if (!current) return;
          setAnswer((last) =>
            last?.document === document && last.url === url
              ? last
              : { document, url },
          );
        },
        () => {
          if (!current) return;
          // A failed repost keeps the frame it already has.
          setAnswer((last) =>
            last?.document === document && last.url !== null
              ? last
              : { document, url: null },
          );
        },
      );
    };
    post();
    const keep = setInterval(post, KEEP_MS);
    return () => {
      current = false;
      clearInterval(keep);
    };
  }, [fleet, slot, document, open]);
  return frameOf(answer, document);
}
