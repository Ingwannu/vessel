// A terminal session (DESIGN.md §6.8): xterm.js in the design's frame,
// under the same title bar as a session with a timeline.
//
// The view replays the session's terminal records from the start, as
// `vessel attach` does, then follows live. Keys go to the session as
// input. The size is sent only by a tab that has focus and has typed, so
// a tab that only watches never resizes the harness (Paseo's rule). Keys
// and sizes go out one send at a time, in order (data/terminal-sender.ts).
// A routed session's model calls (protocol 7) are summed into the frame's
// chrome; the records hold metadata only.

import { useEffect, useRef, useState } from "react";

import { useFleet } from "../../context";
import { followSession, sendTerminal } from "../../data/calls";
import { addCall, type Usage, usageLine } from "../../data/models";
import { decodeBytes } from "../../data/terminal";
import { Banner, Tooltip } from "../../ui/kit";
import { toast } from "../../ui/toasts";
import type { NodeKey, StreamState } from "../../wire/bridge";
import type { SessionInfo } from "../../wire/proto";
import { mountTerminal } from "./mount-terminal";

export function TerminalView({
  node,
  label,
  session,
}: {
  node: NodeKey;
  label: string;
  session: SessionInfo;
}) {
  const fleet = useFleet();
  const host = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState<{ cols: number; rows: number } | null>(null);
  const [stream, setStream] = useState<StreamState | null>(null);
  // Kept with the session and the next record it expects, so a follow
  // that starts again from the first record counts no call twice.
  const [usage, setUsage] = useState<{
    id: string;
    next: number;
    usage: Usage;
  } | null>(null);
  useEffect(() => {
    const element = host.current;
    if (element === null) {
      return undefined;
    }
    const terminal = mountTerminal(
      element,
      (input) => sendTerminal(fleet, node, session.id, input),
      setSize,
      () =>
        toast("Keys dropped: the session is not reading its input", "error"),
    );
    const stop = followSession(fleet, node, session.id, 0, {
      record: (record) => {
        const event = record.event;
        if (event.type === "terminal") {
          terminal.write(decodeBytes(event.data));
        } else if (event.type === "model_call") {
          setUsage((current) => {
            const mine = current?.id === session.id ? current : null;
            if (mine !== null && record.seq < mine.next) {
              return current;
            }
            return {
              id: session.id,
              next: record.seq + 1,
              usage: addCall(mine?.usage ?? null, event),
            };
          });
        }
      },
      state: setStream,
    });
    return () => {
      stop();
      terminal.close();
    };
  }, [fleet, node, session.id]);
  const frozen = stream?.state === "reconnecting" || stream?.state === "failed";
  const calls = usage?.id === session.id ? usage.usage : null;
  return (
    <>
      {stream?.state === "reconnecting" ? (
        <Banner>
          Reconnecting to {label} · {stream.message} · the terminal is frozen
        </Banner>
      ) : stream?.state === "failed" ? (
        <Banner tone="error">The stream stopped: {stream.message}</Banner>
      ) : null}
      <div className="term-wrap">
        <div className="term-frame">
          <div className="term-inner">
            <div className="term-chrome">
              <span>{label}</span>
              <span>
                {size === null
                  ? ""
                  : `${String(size.cols)}×${String(size.rows)}`}
              </span>
              {calls === null ? null : (
                <Tooltip label={calls.models.join(", ")}>
                  <span className="calls">{usageLine(calls)}</span>
                </Tooltip>
              )}
              <span className="live">
                {frozen
                  ? "frozen"
                  : stream?.state === "ended"
                    ? "ended"
                    : "live"}
              </span>
            </div>
            <div
              className="term xterm-host"
              ref={host}
              aria-label="Session terminal"
            />
          </div>
        </div>
      </div>
    </>
  );
}
