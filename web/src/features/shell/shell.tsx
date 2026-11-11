// The chrome every page sits in (DESIGN.md §2.2): the sidebar and the
// framed page with its title bar, set into one grey shell.

import "./frame.css";

import { Outlet } from "@tanstack/react-router";
import { type ReactElement, useState } from "react";

import { useFleet, useFleetState } from "../../context";
import { Banner, Button } from "../../ui/kit";
import { Toasts } from "../../ui/toasts";
import { useGoShortcuts } from "../palette/shortcuts";
import { FrameTop } from "./frame-top";
import { Sidebar } from "./sidebar";
import { TitleBarSlot } from "./title-bar";
import { useNewSessionEntry } from "./use-new-session";

export function Shell(): ReactElement {
  const [slot, setSlot] = useState<HTMLElement | null>(null);
  useGoShortcuts();
  useNewSessionEntry();
  return (
    <>
      <a className="skip" href="#main">
        Skip to main
      </a>
      {/* The sidebar and the margin around the page are one shell; the
          page is a framed surface set into it, with its title bar inside
          the frame (DESIGN.md §2.2). */}
      <div className="app">
        <Sidebar />
        <div className="frame-wrap">
          <div className="frame">
            <FrameTop slot={setSlot} />
            <TitleBarSlot.Provider value={slot}>
              <main className="main" id="main" role="main">
                <Disconnected />
                <Outlet />
              </main>
            </TitleBarSlot.Provider>
          </div>
        </div>
      </div>
      <Toasts />
    </>
  );
}

/** After repeated failures the tab stops redialling (bridge/client.ts):
 * say so, and offer to try again with the same key. */
function Disconnected() {
  const fleet = useFleet();
  const link = useFleetState((state) => state.link);
  if (link.kind !== "disconnected") {
    return null;
  }
  return (
    <Banner
      tone="error"
      action={
        <Button
          size="sm"
          onClick={() => {
            fleet.bridge.retry();
          }}
        >
          Retry
        </Button>
      }
    >
      Disconnected — vessel ui may have restarted. If it did, open the address
      it printed.
    </Banner>
  );
}
