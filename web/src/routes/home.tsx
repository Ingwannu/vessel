// The index (track W2): the board once a machine is connected, the
// welcome while none is, nothing while the page cannot tell yet
// (`data/welcome.ts` decides which).

import { Navigate } from "@tanstack/react-router";
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
  useCrumbs(screen === "welcome" ? [{ label: "Welcome" }] : []);
  switch (screen) {
    case "wait":
      return null;
    case "welcome":
      return <Welcome />;
    case "board":
      return <Navigate to="/sessions" replace />;
  }
}
