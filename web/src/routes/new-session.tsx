// `/new` (agent-orchestrator's new-task page): the machine named in the
// address, if any, and the page (features/compose).

import type { ReactElement } from "react";

import { useWhere } from "../data/where";
import { NewSession } from "../features/compose/new-session";

export function NewSessionPage(): ReactElement {
  const where = useWhere();
  return <NewSession wanted={where.machine ?? null} />;
}
