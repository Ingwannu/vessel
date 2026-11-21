// The standalone terminal pane uses the same xterm mount as agent PTYs
// and follows terminal records through the bridge's resumable stream.
// Which records are live, and so may have their queries answered, is the
// terminal's `next_seq` read as the follow starts and again after every
// gap (`data/terminal-live.ts`, TRACKER 826).

import { useEffect, useRef, useState } from "react";

import { useFleet } from "../../context";
import { errorCode } from "../../data/outcome";
import { decodeBytes } from "../../data/terminal";
import { liveBoundary } from "../../data/terminal-live";
import {
  followWorkspaceTerminal,
  openedInThisTab,
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
  const [registry, setRegistry] = useState<"waiting" | "ready" | "missing">(
    "waiting",
  );
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
      openedInThisTab(terminal) ? "opened" : "attached",
    );
    const live = liveBoundary({
      read: () => readWorkspaceTerminal(fleet, node, terminal),
      onLink: (listener) => fleet.bridge.onLink(listener),
    });
    const stop = followWorkspaceTerminal(fleet, node, terminal, {
      record: ({ seq, event }) => {
        if (event.type === "terminal") {
          pane.write(decodeBytes(event.data), live.written(seq));
          setRegistry("ready");
        }
      },
      state: (state) => {
        live.stream(state);
        setStream(state);
      },
    });
    return () => {
      live.stop();
      stop();
      pane.close();
    };
  }, [fleet, node, terminal, words.dropped]);

  useEffect(() => {
    if (!connected) {
      return undefined;
    }
    let live = true;
    void readWorkspaceTerminal(fleet, node, terminal).then((result) => {
      if (!live) {
        return;
      }
      if (result.ok) {
        setRegistry("ready");
      } else if (errorCode(result.problem) === "not_found") {
        setRegistry("missing");
      }
    });
    return () => {
      live = false;
    };
  }, [connected, fleet, node, terminal]);

  return (
    <div className="workspace-terminal">
      <div className="xterm-host" ref={host} aria-label={words.terminal} />
      {!connected ? (
        <div className="workspace-terminal-state">{words.disconnected}</div>
      ) : registry === "missing" || stream?.state === "failed" ? (
        <div role="alert" className="workspace-terminal-error">
          {registry === "missing"
            ? words.terminalMissing
            : words.terminalFailed}
        </div>
      ) : stream?.state === "ended" ? (
        <div role="status" className="workspace-terminal-error">
          {words.terminalExited}
        </div>
      ) : registry === "waiting" && stream?.state !== "resumed" ? (
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
