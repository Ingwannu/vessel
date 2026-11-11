// `/sessions/<machine>/<id>`: one session (features/session).

import type { ReactElement } from "react";

import { useWhere } from "../data/where";
import { SessionView } from "../features/session/session-page";

export function SessionPage(): ReactElement {
  const where = useWhere();
  return (
    <SessionView node={where.segments[1] ?? ""} id={where.segments[2] ?? ""} />
  );
}
