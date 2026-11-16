// The app: the fleet controller in context, the routes (DESIGN.md §2.3),
// and the pages' shared chrome.

import {
  createRootRoute,
  createRoute,
  createRouter,
  lazyRouteComponent,
  RouterProvider,
} from "@tanstack/react-router";
import { type ReactElement, useEffect, useState } from "react";

import { Bridge } from "./bridge/client";
import { forgetToken, type Credential } from "./bridge/token";
import { FleetContext } from "./context";
import { Fleet } from "./data/fleet";
import { followPreferences } from "./data/prefs";
import { AddMachineHost } from "./features/machines/add-machine";
import { LabelManagerHost } from "./features/workspace-labels/manager";
import { Shell } from "./features/shell/shell";
import { HomePage } from "./routes/home";
import {
  DevicesRedirect,
  FleetRedirect,
  SettingsIndex,
  SyncRedirect,
} from "./routes/redirects";
import { SessionsPage } from "./routes/sessions";
import { TooltipProvider } from "./ui/kit";

/** `?node=` on the pages scoped by node. */
function nodeSearch(search: Record<string, unknown>): { node?: string } {
  return typeof search["node"] === "string" ? { node: search["node"] } : {};
}

/** `?machine=` and `?agent=` on Settings' sections (track W2): the
 * machine a section is about, the agent whose card is open. */
function settingsSearch(search: Record<string, unknown>): {
  machine?: string;
  agent?: string;
} {
  return {
    ...machineSearch(search),
    ...(typeof search["agent"] === "string" ? { agent: search["agent"] } : {}),
  };
}

/** `?machine=` on the new-session page and on Settings: the machine in
 * view. */
function machineSearch(search: Record<string, unknown>): { machine?: string } {
  return typeof search["machine"] === "string"
    ? { machine: search["machine"] }
    : {};
}

/** The frame, and the add-machine dialog, which Settings, the welcome
 * screen and the palette all open (`data/add-machine.ts`). It is drawn
 * here, inside the router, so its last step can lead to a session. */
function Root(): ReactElement {
  return (
    <>
      <Shell />
      <AddMachineHost />
      <LabelManagerHost />
    </>
  );
}

const rootRoute = createRootRoute({ component: Root });

/* [Decision Log] Screens loaded when they open (milestone 11, track P)
   - Purpose: the page's script was one 1,124 KB file, the terminal
     (xterm.js) and Markdown included, fetched and compiled before the
     board's first paint; over a phone's link that is seconds
     (docs/PERFORMANCE.md).
   - Constraint: ADR 0017's defence: no script runs that the index did
     not pin. A chunk imported later is pinned by the index's import map
     (`integrity`, scripts/integrity.mjs), which the CSP admits by its
     hash (vessel-web's files and guard); security.spec.ts poisons a
     chunk and checks it does not run.
   - Choice: a session, a new session and Settings are chunks of their
     own (the kit's gallery too, main.tsx); the board, the shell and the
     controls stay in the entry, since every visit draws them.
   - Cost: opening one of those screens the first time waits for its
     chunk (one request; the measured sizes are in PERFORMANCE.md). */

const routeTree = rootRoute.addChildren([
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/",
    component: HomePage,
  }),
  // Fleet, Sync and Devices are Settings' sections now; their old
  // addresses (bookmarks, the CLI's printed links) lead there.
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/fleet",
    component: FleetRedirect,
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/fleet/$node",
    component: FleetRedirect,
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/new",
    validateSearch: machineSearch,
    component: lazyRouteComponent(
      () => import("./routes/new-session"),
      "NewSessionPage",
    ),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/sessions",
    validateSearch: nodeSearch,
    component: SessionsPage,
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/sessions/$node/$id",
    component: lazyRouteComponent(
      () => import("./routes/session"),
      "SessionPage",
    ),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/sync",
    validateSearch: nodeSearch,
    component: SyncRedirect,
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/devices",
    validateSearch: nodeSearch,
    component: DevicesRedirect,
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/settings",
    component: SettingsIndex,
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/settings/$section",
    validateSearch: settingsSearch,
    component: lazyRouteComponent(
      () => import("./routes/settings"),
      "SettingsPage",
    ),
  }),
]);

export const router = createRouter({ routeTree, defaultPreload: false });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

/** Shown when the page has no token, or the server refused it. */
function NoToken({ refused }: { refused: boolean }) {
  return (
    <div className="empty" role="main">
      <h2>
        {refused
          ? "The UI was restarted or its identity could not be verified"
          : "Open Vessel from its address"}
      </h2>
      <p>
        Run <code>vessel ui</code> and open the address it prints. Each launch
        has a one-time address; keep this tab open after using it.
      </p>
    </div>
  );
}

export function App({ token }: { token: Credential | null }) {
  const [fleet] = useState(() =>
    token === null ? null : new Fleet(new Bridge(token)),
  );
  const [refused, setRefused] = useState(false);
  useEffect(() => {
    if (fleet === null) {
      return undefined;
    }
    const unfollow = followPreferences(fleet);
    // A draft typed in the last half second is saved as the page goes
    // (hidden, or closed), not lost with it (data/drafts.ts).
    const save = () => {
      if (document.visibilityState === "hidden") {
        fleet.drafts.flush();
      }
    };
    const leaving = () => {
      fleet.drafts.flush();
    };
    document.addEventListener("visibilitychange", save);
    window.addEventListener("pagehide", leaving);
    const off = fleet.bridge.onLink((link) => {
      if (link.kind === "refused") {
        forgetToken();
        setRefused(true);
      }
    });
    fleet.start();
    return () => {
      document.removeEventListener("visibilitychange", save);
      window.removeEventListener("pagehide", leaving);
      unfollow();
      off();
      fleet.stop();
    };
  }, [fleet]);
  if (fleet === null || refused) {
    return <NoToken refused={refused} />;
  }
  return (
    <FleetContext.Provider value={fleet}>
      <TooltipProvider>
        <RouterProvider router={router} />
      </TooltipProvider>
    </FleetContext.Provider>
  );
}
