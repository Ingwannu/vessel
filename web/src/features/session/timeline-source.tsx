// What a timeline's rows need to fetch what they show (a prompt's
// images): the machine, the session and the scrolling element, given
// once by the session's screen instead of passed through every row, so a
// row stays a function of its entry alone (`TimelineItem` is memoised by
// entry).

import { createContext, useContext } from "react";

import type { NodeKey } from "../../wire/bridge";
import type { SessionId } from "../../wire/proto";

export interface TimelineSource {
  node: NodeKey;
  session: SessionId;
  /** The timeline's scrolling element, the root its images are near. */
  scroller: HTMLElement | null;
}

export const TimelineSourceContext = createContext<TimelineSource | null>(null);

/** The session the timeline shows; `null` outside one (a test, the
 * gallery). */
export function useTimelineSource(): TimelineSource | null {
  return useContext(TimelineSourceContext);
}
