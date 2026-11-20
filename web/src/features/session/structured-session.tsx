// A session with a timeline (DESIGN.md §3.1): the title bar, the log in a
// centred column with the composer docked under it, and the summary
// beside them (hidden or shown by its shortcut, kept with the bridge).
// Over the log: the find bar (its shortcut, Mod+F by default) and the
// outline of the prompts, both reaching the log through its one seam
// (`reveal.ts`).

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { useFleet, useFleetState } from "../../context";
import { isLive } from "../../data/format";
import type { Match } from "../../data/find";
import {
  forgetOpening,
  openings,
  promptLogged,
  withOpening,
} from "../../data/openings";
import { outline, sameOutline } from "../../data/outline";
import { prefer, prefs } from "../../data/prefs";
import { useStore } from "../../data/store";
import type { SessionInfo } from "../../wire/proto";
import { Banner, Sheet } from "../../ui/kit";
import { useShortcut } from "../shortcuts/handlers";
import { Composer } from "./composer";
import { FindBar } from "./find-bar";
import { FoldProvider } from "./folds";
import { Inspector } from "./inspector";
import { Outline } from "./outline";
import { useMedia } from "../shell/use-media";
import { clearMarks, type Reveal } from "./reveal";
import { SessionBar } from "./session-bar";
import { Timeline } from "./timeline";
import { TimelineSourceContext } from "./timeline-source";
import { useReadMarks } from "./use-read-marks";
import { useTimeline } from "./use-timeline";
import type { NodeKey } from "../../wire/bridge";

export function StructuredSession({
  node,
  label,
  session,
  openChanges,
}: {
  node: NodeKey;
  /** The machine's name. */
  label: string;
  session: SessionInfo;
  openChanges: () => void;
}) {
  const fleet = useFleet();
  const { timeline, stream } = useTimeline(node, session.id, isLive(session));
  const protocol = useFleetState((state) => {
    const status = state.statuses[node];
    return status?.state === "connected" ? status.protocol : null;
  });
  useReadMarks(node, session, timeline, protocol);
  // A first prompt sent from the new-session page, shown at the log's end
  // until the log has it (data/openings.ts).
  const opening = useStore(openings, (kept) => kept.get(session.id));
  const shown = useMemo(
    () => withOpening(timeline, opening),
    [timeline, opening],
  );
  const logged = opening !== undefined && promptLogged(timeline);
  useEffect(() => {
    if (logged) {
      forgetOpening(session.id);
    }
  }, [logged, session.id]);
  const inspector = useStore(prefs, (state) => state.values.layout.inspector);
  const narrow = useMedia("(width < 1280px)");
  const [summaryOpen, setSummaryOpen] = useState(false);
  const [finding, setFinding] = useState<number | null>(null);
  const [reveal, setReveal] = useState<Reveal | null>(null);
  const [scroller, setScroller] = useState<HTMLDivElement | null>(null);
  const asked = useRef(0);
  const source = useMemo(
    () => ({ node, session: session.id, scroller }),
    [node, session.id, scroller],
  );
  // The outline as last drawn while a batch changes no prompt or reply,
  // so the outline draws nothing for it (data/outline.ts).
  const worked = useMemo(() => outline(timeline.entries), [timeline.entries]);
  const [items, setItems] = useState(worked);
  if (items !== worked && !sameOutline(items, worked)) {
    setItems(worked);
  }

  useShortcut("find", () => {
    setFinding((count) => (count ?? 0) + 1);
    return true;
  });
  useShortcut("toggle-inspector", () => {
    void prefer(fleet, {
      set: "inspector_panel",
      value: inspector === "shown" ? "hidden" : "shown",
    });
    return true;
  });
  const onReveal = useCallback((match: Match | null, query: string) => {
    asked.current += 1;
    setReveal(
      match === null
        ? null
        : {
            key: match.key,
            seq: asked.current,
            match: { query, nth: match.nth },
          },
    );
    if (match === null) {
      clearMarks();
    }
  }, []);
  // Find's marks are the document's (the CSS highlight registry), not
  // this screen's: they go with it, so another session opened with find
  // still open shows none of this one's.
  useEffect(
    () => () => {
      clearMarks();
    },
    [],
  );
  const onJump = useCallback((key: string) => {
    asked.current += 1;
    setReveal({ key, seq: asked.current, match: null });
  }, []);
  const closeFind = useCallback(() => {
    setFinding(null);
    setReveal(null);
    clearMarks();
  }, []);
  return (
    <>
      <SessionBar
        node={node}
        label={label}
        session={session}
        renamed={timeline.title}
        onPlace={openChanges}
        {...(narrow
          ? {
              onSummary: () => {
                setSummaryOpen(true);
              },
            }
          : {})}
      />
      <div className="session-main">
        {stream?.state === "reconnecting" ? (
          <Banner>
            Reconnecting to {label} · {stream.message}
          </Banner>
        ) : stream?.state === "failed" ? (
          <Banner tone="error">The stream stopped: {stream.message}</Banner>
        ) : null}
        <div className="session-log">
          <TimelineSourceContext value={source}>
            <FoldProvider entries={timeline.entries}>
              <Timeline
                timeline={shown}
                reveal={reveal}
                scrollerRef={setScroller}
              />
            </FoldProvider>
          </TimelineSourceContext>
          <Outline items={items} scroller={scroller} onJump={onJump} />
          {finding === null ? null : (
            <FindBar
              entries={timeline.entries}
              onReveal={onReveal}
              onClose={closeFind}
              focusRequest={finding}
            />
          )}
        </div>
        <div className="session-dock">
          <div className="session-dock-column">
            <Composer
              node={node}
              label={label}
              session={session}
              timeline={timeline}
            />
          </div>
        </div>
      </div>
      {inspector === "shown" && !narrow ? (
        <Inspector
          node={node}
          label={label}
          session={session}
          timeline={timeline}
        />
      ) : null}{" "}
      {narrow && summaryOpen ? (
        <Sheet
          side="right"
          label="Summary"
          onClose={() => {
            setSummaryOpen(false);
          }}
        >
          <Inspector
            node={node}
            label={label}
            session={session}
            timeline={timeline}
          />
        </Sheet>
      ) : null}
    </>
  );
}
