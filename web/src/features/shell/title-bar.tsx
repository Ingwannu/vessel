// The frame's title bar as a place a page can draw into (DESIGN.md §2.2).
//
// [Decision Log]
// - Purpose: most pages are named by their breadcrumb, which the frame
//   draws from `useCrumbs`. A session is named by its title, which a
//   person renames where it stands (agent-orchestrator's session tab;
//   protocol 9's `session.rename`), and which needs the session's node
//   and calls to do so.
// - Alternatives: crumbs that carry a rename callback (the chrome's
//   store would hold functions and components); the session drawing a
//   second bar under the frame's (milestone 9's sub-line, which the owner
//   asked to go).
// - Choice: the frame's bar has a slot; a page that names itself renders
//   `<TitleBar>`, which is portalled into it, and sets no crumbs. The
//   bar's own actions (New) stay where they are.

import { createContext, type ReactNode, useContext } from "react";
import { createPortal } from "react-dom";

/** The slot's element, once the frame has drawn it. */
export const TitleBarSlot = createContext<HTMLElement | null>(null);

/** Draw `children` in the frame's title bar, where the crumbs go. */
export function TitleBar({ children }: { children: ReactNode }) {
  const slot = useContext(TitleBarSlot);
  return slot === null ? null : createPortal(children, slot);
}
