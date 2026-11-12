// Where a session's attached files go: the session's upload calls, what
// its agent takes and the node's limits, given to the tray once all are
// known and the session runs; nowhere otherwise (the files wait, checked
// once there is somewhere).

import { useEffect } from "react";

import { useFleet } from "../../context";
import {
  beginUpload,
  finishUpload,
  writeUpload,
} from "../../data/conversation-calls";
import type { Tray } from "../../data/tray";
import type { NodeKey } from "../../wire/bridge";
import type { AgentAbilities, SessionId } from "../../wire/proto";
import type { Limits } from "./use-limits";

export function useTrayTarget(
  tray: Tray,
  node: NodeKey,
  session: SessionId,
  live: boolean,
  abilities: AgentAbilities | null,
  limits: Limits,
  agent: string,
): void {
  const fleet = useFleet();
  const read = limits.kind === "read" ? limits.limits : null;
  useEffect(() => {
    if (!live || read === null) {
      tray.target(null);
      return;
    }
    tray.target({
      calls: {
        begin: (name, mime, size) =>
          beginUpload(fleet, node, session, name, mime, size),
        write: (upload, offset, data) =>
          writeUpload(fleet, node, upload, offset, data),
        finish: (upload) => finishUpload(fleet, node, upload),
      },
      abilities,
      limits: read,
      agent,
    });
  }, [tray, fleet, node, session, live, abilities, read, agent]);
}
