// The node's attachment limits for a session (`session.conversation`'s
// `limits`), read once when the session's composer opens on a machine
// speaking protocol 10, and again when the session comes back to life
// (a start again): everything else the conversation holds comes in the
// log. A failure leaves attachments unoffered, with its reason.

import { useEffect, useState } from "react";

import { useFleet } from "../../context";
import { readConversation } from "../../data/conversation-calls";
import type { NodeKey } from "../../wire/bridge";
import type { AttachmentLimits, SessionId } from "../../wire/proto";

export type Limits =
  | { kind: "none" }
  | { kind: "read"; limits: AttachmentLimits }
  | { kind: "failed"; message: string };

export function useLimits(
  node: NodeKey,
  session: SessionId,
  wanted: boolean,
): Limits {
  const fleet = useFleet();
  const [limits, setLimits] = useState<Limits>({ kind: "none" });
  useEffect(() => {
    if (!wanted) {
      return undefined;
    }
    let current = true;
    void readConversation(fleet, node, session).then((outcome) => {
      if (current) {
        setLimits(
          outcome.ok
            ? { kind: "read", limits: outcome.value.limits }
            : { kind: "failed", message: outcome.problem.message },
        );
      }
    });
    return () => {
      current = false;
    };
  }, [fleet, node, session, wanted]);
  return wanted ? limits : { kind: "none" };
}
