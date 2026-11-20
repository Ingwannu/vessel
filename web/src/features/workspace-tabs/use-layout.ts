import { type Dispatch, type SetStateAction, useEffect, useState } from "react";

import {
  addDiscovered,
  enterRoute,
  loadTabLayout,
  saveTabLayout,
  type TabLayout,
  type WorkspaceTab,
} from "../../data/workspace/tab-layout";
import type { SessionId } from "../../wire/proto";

interface LayoutState {
  layout: TabLayout;
  update: Dispatch<SetStateAction<TabLayout>>;
}

export function useLayout(
  scope: string,
  route: SessionId,
  discovered: readonly WorkspaceTab[],
): LayoutState {
  const [layout, update] = useState<TabLayout>(() =>
    addDiscovered(
      enterRoute(loadTabLayout(window.localStorage, scope), route),
      discovered,
    ),
  );
  // Apply route changes before drawing so an old workspace selection
  // cannot appear under a URL that was just opened from the sidebar.
  const current = addDiscovered(enterRoute(layout, route), discovered);
  if (current !== layout) {
    update(current);
  }
  useEffect(() => {
    saveTabLayout(window.localStorage, scope, layout);
  }, [layout, scope]);
  return { layout: current, update };
}
