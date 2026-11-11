// A session with a timeline (DESIGN.md §3.1): the title bar, the log in a
// centred column with the composer docked under it, and the summary
// beside them.

import type { SessionInfo } from "../../wire/proto";
import { Banner } from "../../ui/kit";
import { Composer } from "./composer";
import { Inspector } from "./inspector";
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
  const { timeline, stream } = useTimeline(node, session.id);
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
        <Timeline timeline={timeline} />
        <div className="session-dock">
          <div className="session-dock-column">
            <Composer node={node} session={session} timeline={timeline} />
          </div>
        </div>
      </div>
      <Inspector label={label} session={session} timeline={timeline} />
    </>
  );
}
