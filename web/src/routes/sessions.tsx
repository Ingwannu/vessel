// `/sessions`: the board of every machine, or of the one in `?node=`
// (features/board).

import type { ReactElement } from "react";

import { useWhere } from "../data/where";
import { Board } from "../features/board/board-page";

export function SessionsPage(): ReactElement {
  const where = useWhere();
  return <Board scope={where.node ?? null} />;
}
