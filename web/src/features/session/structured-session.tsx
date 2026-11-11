// A session with a timeline (DESIGN.md §3.1): the title bar, the log in a
// centred column with the composer docked under it, and the summary
// beside them (hidden or shown by its shortcut, kept with the bridge).
// Over the log: the find bar (its shortcut, Mod+F by default) and the
// outline of the prompts, both reaching the log through its one seam
// (`reveal.ts`).

import { useCallback, useMemo, useRef, useState } from "react";

import { useFleet } from "../../context";
import type { Match } from "../../data/find";
import { outline } from "../../data/outline";
import { prefer, prefs } from "../../data/prefs";
import { useStore } from "../../data/store";
import type { SessionInfo } from "../../wire/proto";
import { Banner } from "../../ui/kit";
import { useShortcut } from "../shortcuts/handlers";
import { Composer } from "./composer";
import { FindBar } from "./find-bar";
import { FoldProvider } from "./folds";
import { Inspector } from "./inspector";
import { Outline } from "./outline";
import { clearMarks, type Reveal } from "./reveal";
import { SessionBar } from "./session-bar";
import { Timeline } from "./timeline";
import { useTimeline } from "./use-timeline";

export function StructuredSession({
  node,
  label,
  session,
}: {
  node: string;
  /** The machine's name. */
  label: string;
  session: SessionInfo;
}) {
  const fleet = useFleet();
  const { timeline, stream } = useTimeline(node, session.id);
  const inspector = useStore(prefs, (state) => state.values.layout.inspector);
  const [finding, setFinding] = useState<number | null>(null);
  const [reveal, setReveal] = useState<Reveal | null>(null);
  const [scroller, setScroller] = useState<HTMLDivElement | null>(null);
  const asked = useRef(0);
  const items = useMemo(() => outline(timeline.entries), [timeline.entries]);

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
          <FoldProvider entries={timeline.entries}>
            <Timeline
              timeline={timeline}
              reveal={reveal}
              scrollerRef={setScroller}
            />
          </FoldProvider>
          <Outline
            items={items}
            scroller={scroller}
            onJump={(key) => {
              asked.current += 1;
              setReveal({ key, seq: asked.current, match: null });
            }}
          />
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
            <Composer node={node} session={session} timeline={timeline} />
          </div>
        </div>
      </div>
      {inspector === "shown" ? (
        <Inspector label={label} session={session} timeline={timeline} />
      ) : null}
    </>
  );
}
