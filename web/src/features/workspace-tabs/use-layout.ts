import { useRouter } from "@tanstack/react-router";
import { type Dispatch, type SetStateAction, useEffect, useState } from "react";

import {
  addDiscovered,
  enterRoute,
  loadTabLayout,
  removeTab,
  tabKey,
  saveTabLayout,
  type TabLayout,
  type WorkspaceTab,
} from "../../data/workspace/tab-layout";
import type { NodeKey } from "../../wire/bridge";
import type { SessionId } from "../../wire/proto";

interface LayoutState {
  layout: TabLayout;
  update: Dispatch<SetStateAction<TabLayout>>;
}

export function useLayout(
  scope: string,
  node: NodeKey,
  route: SessionId,
  discovered: readonly WorkspaceTab[],
  ended: readonly WorkspaceTab[],
): LayoutState {
  const [layout, update] = useState<TabLayout>(() =>
    addDiscovered(
      enterRoute(loadTabLayout(window.localStorage, scope), route),
      discovered,
    ),
  );
  const router = useRouter();
  const [at, setAt] = useState(route);
  // Closing the anchor clears its persisted route, but only a navigation
  // or reload can reveal it again. Same-URL navigation also emits resolved.
  const entering = at !== route;
  const current = ended.reduce(
    (kept, tab) => removeTab(kept, tabKey(tab)),
    addDiscovered(entering ? enterRoute(layout, route) : layout, discovered),
  );
  if (entering) {
    setAt(route);
  }
  if (current !== layout) {
    update(current);
  }
  useEffect(
    () =>
      router.subscribe("onResolved", ({ toLocation }) => {
        if (toLocation.pathname === `/sessions/${node}/${route}`) {
          update((layout) => enterRoute(layout, route));
        }
      }),
    [node, route, router],
  );
  useEffect(() => {
    saveTabLayout(window.localStorage, scope, layout);
  }, [layout, scope]);
  return { layout: current, update };
}
