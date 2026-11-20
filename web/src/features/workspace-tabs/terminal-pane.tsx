// The standalone terminal pane uses the same xterm mount as agent PTYs
// and follows terminal records through the bridge's resumable stream.

import { useEffect, useRef, useState } from "react";

import { useFleet } from "../../context";
import { decodeBytes } from "../../data/terminal";
import {
  followWorkspaceTerminal,
  sendWorkspaceTerminal,
} from "../../data/workspace/terminals";
import { Banner, Spinner } from "../../ui/kit";
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
  const [received, setReceived] = useState(false);
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
    );
    const stop = followWorkspaceTerminal(fleet, node, terminal, {
      record: ({ event }) => {
        if (event.type === "terminal") {
          pane.write(decodeBytes(event.data));
          setReceived(true);
        }
      },
      state: setStream,
    });
    return () => {
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
        <Banner tone="error">
          {words.terminalFailed} · {stream.message}
        </Banner>
      ) : stream?.state === "ended" ? (
        <Banner tone="error">{words.terminalExited}</Banner>
      ) : !received && stream === null ? (
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
