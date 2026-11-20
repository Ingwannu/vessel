// Where an HTML file's Preview is framed from (ADR 0122, TRACKER 814):
// the document is posted to `vessel ui` and framed from the address it
// answers, where its scripts run under Paseo's sandboxed preview policy.
// Each panel names itself with this tab's random prefix and React's id, so
// a panel's next document replaces only its own last one, never another
// tab's. Until the address comes the panel says it is loading; when the
// post fails (an older `vessel ui`, a lost link) the panel falls back to
// the static frame, whose scripts do not run.

import { useEffect, useId, useState } from "react";

import { useFleet } from "../../context";
import { framePreview } from "../../data/workspace/html-preview";

/** This tab's prefix for its panels' names (made once, at load). */
const TAB = crypto.randomUUID();

export type PreviewFrame =
  | { status: "loading" }
  | { status: "ready"; url: string }
  /** The static `srcdoc` frame, scripts off. */
  | { status: "static" };

interface Answer {
  document: string;
  url: string | null;
}

export function usePreviewFrame(document: string): PreviewFrame {
  const fleet = useFleet();
  const slot = `${TAB}-${useId().replace(/[^A-Za-z0-9_-]/g, "")}`;
  const [answer, setAnswer] = useState<Answer | null>(null);
  useEffect(() => {
    let current = true;
    void framePreview(fleet, slot, document).then(
      (url) => {
        if (current) setAnswer({ document, url });
      },
      () => {
        if (current) setAnswer({ document, url: null });
      },
    );
    return () => {
      current = false;
    };
  }, [fleet, slot, document]);
  if (answer?.document !== document) return { status: "loading" };
  return answer.url === null
    ? { status: "static" }
    : { status: "ready", url: answer.url };
}
