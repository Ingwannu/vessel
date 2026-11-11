// `/sessions/<machine>/<id>`: one session (features/session). The two
// segments become a machine's key and a session's id here, at the edge
// (`bridge/ids.ts`); an address with anything else names no session, and
// says so before any call is sent.

import type { ReactElement } from "react";

import { nodeKeyOf, sessionIdOf } from "../bridge/ids";
import { useWhere } from "../data/where";
import { SessionView } from "../features/session/session-page";
import { Banner } from "../ui/kit";

export function SessionPage(): ReactElement {
  const where = useWhere();
  const node = nodeKeyOf(where.segments[1] ?? "");
  const id = sessionIdOf(where.segments[2] ?? "");
  if (node === null || id === null) {
    return (
      <div className="page">
        <Banner tone="error">No session has this address.</Banner>
      </div>
    );
  }
  return <SessionView node={node} id={id} />;
}
