// The standalone terminal pane uses the same xterm mount as agent PTYs
// and follows terminal records through the bridge's resumable stream.

import { useEffect, useRef, useState } from "react";

import { useFleet } from "../../context";
import { decodeBytes } from "../../data/terminal";
import {
  followWorkspaceTerminal,
  readWorkspaceTerminal,
  sendWorkspaceTerminal,
} from "../../data/workspace/terminals";
import { Spinner } from "../../ui/kit";
import { toast } from "../../ui/toasts";
import type { NodeKey, StreamState } from "../../wire/bridge";
import type { TerminalId } from "../../wire/proto";
import { mountTerminal } from "../session/mount-terminal";
import type { TabWords } from "./words";

export function TerminalPane({
  node,
  terminal,
  connected,
  words,
}: {
  node: NodeKey;
  terminal: TerminalId;
  connected: boolean;
  words: TabWords;
}) {
  const fleet = useFleet();
  const host = useRef<HTMLDivElement>(null);
  const [stream, setStream] = useState<StreamState | null>(null);
  const [ready, setReady] = useState(false);
  useEffect(() => {
    const element = host.current;
    if (element === null) {
      return undefined;
    }
    const pane = mountTerminal(
      element,
      (input) => sendWorkspaceTerminal(fleet, node, terminal, input),
      () => undefined,
      () => toast(words.dropped, "error"),
      "on-attach",
    );
    const stop = followWorkspaceTerminal(fleet, node, terminal, {
      record: ({ event }) => {
        if (event.type === "terminal") {
          pane.write(decodeBytes(event.data));
          setReady(true);
        }
      },
      state: setStream,
    });
    let live = true;
    void readWorkspaceTerminal(fleet, node, terminal).then((result) => {
      if (live && result.ok) {
        setReady(true);
      }
    });
    return () => {
      live = false;
      stop();
      pane.close();
    };
  }, [fleet, node, terminal, words.dropped]);

  return (
    <div className="workspace-terminal">
      <div className="xterm-host" ref={host} aria-label={words.terminal} />
      {!connected ? (
        <div className="workspace-terminal-state">{words.disconnected}</div>
      ) : stream?.state === "failed" ? (
        <div role="alert" className="workspace-terminal-error">
          {stream.reason === "removed"
            ? words.terminalMissing
            : words.terminalFailed}
        </div>
      ) : stream?.state === "ended" ? (
        <div role="status" className="workspace-terminal-error">
          {words.terminalExited}
        </div>
      ) : !ready && stream?.state !== "resumed" ? (
        <div
          className="workspace-terminal-state loading"
          aria-label={words.loading}
        >
          <Spinner />
        </div>
      ) : null}
    </div>
  );
}
