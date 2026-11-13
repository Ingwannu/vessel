// The index (track W2): the new-session page once a machine is connected
// (GOAT's first screen is a draft's composer, the new chat, 2026-11-13),
// the welcome while none is, nothing while the page cannot tell yet
// (`data/welcome.ts` decides which). The board is the sidebar's Board.

import { Navigate, useRouterState } from "@tanstack/react-router";
import { type ReactElement, useState } from "react";

import { useCrumbs, useFleetState } from "../context";
import { firstScreen } from "../data/welcome";
import { Welcome } from "../features/welcome/welcome";

export function HomePage(): ReactElement | null {
  // Once shown, the welcome stays while a machine is redialled
  // (`firstScreen`'s `before`): a flag set during render, React's way of
  // keeping what an earlier render saw.
  const [welcomed, setWelcomed] = useState(false);
  const screen = useFleetState((state) =>
    firstScreen(state, welcomed ? "welcome" : "wait"),
  );
  if (screen === "welcome" && !welcomed) {
    setWelcomed(true);
  }
  // Where the router is headed: while a screen loaded on demand (app.tsx)
  // is fetched, this page stays drawn but the reader has already gone
  // elsewhere, and the board's redirect must not take them back.
  const headed = useRouterState({ select: (state) => state.location.pathname });
  useCrumbs(screen === "welcome" ? [{ label: "Welcome" }] : []);
  switch (screen) {
    case "wait":
      return null;
    case "welcome":
      return <Welcome />;
    case "board":
      return headed === "/" ? <Navigate to="/new" replace /> : null;
  }
}
