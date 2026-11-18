// The chrome every page sits in (DESIGN.md §2.2): the sidebar and the
// framed page with its title bar, set into one grey shell. The sidebar
// can be hidden (its shortcut, or the title bar's button; kept with the
// bridge); compact (below 600px) it opens as a sheet from the title
// bar's menu button, as GOAT's does, and picking a row closes it.

import "./frame.css";

import { Outlet } from "@tanstack/react-router";
import { type ReactElement, useState } from "react";

import { useFleet, useFleetState } from "../../context";
import { prefs } from "../../data/prefs";
import { useStore } from "../../data/store";
import { useWhere } from "../../data/where";
import { labelManager } from "../../data/workspace-labels/hub";
import { Banner, Button, Sheet } from "../../ui/kit";
import { Toasts } from "../../ui/toasts";
import { useNotices } from "../notifications/use-notices";
import { PaletteHost } from "../palette/palette";
import { useShortcutListener } from "../shortcuts/listener";
import { FrameTop } from "./frame-top";
import { Sidebar } from "./sidebar";
import { TitleBarSlot } from "./title-bar";
import { useNewSessionEntry } from "./use-new-session";
import { useShellShortcuts } from "./use-shell-shortcuts";

export function Shell(): ReactElement {
  const [slot, setSlot] = useState<HTMLElement | null>(null);
  const shown = useStore(prefs, (state) => state.values.layout.sidebar);
  const where = useWhere();
  const at = where.segments.join("/");
  // The sheet is open at the place it was opened from: going anywhere
  // (picking a row) closes it.
  const [sheetAt, setSheetAt] = useState<string | null>(null);
  // The label manager opens from the sidebar's Display menu. On a phone
  // the sheet steps aside while the manager is open, so one surface sits
  // over the page, and comes back as it was left when the manager
  // closes.
  const managing = useStore(labelManager, (open) => open);
  const sheet = sheetAt === at && !managing;
  useShortcutListener();
  useShellShortcuts();
  useNotices();
  useNewSessionEntry();
  return (
    <>
      <a className="skip" href="#main">
        Skip to main
      </a>
      {/* The sidebar and the margin around the page are one shell; the
          page is a framed surface set into it, with its title bar inside
          the frame (DESIGN.md §2.2). */}
      <div className={shown === "shown" ? "app" : "app no-sidebar"}>
        {shown === "shown" ? <Sidebar /> : null}
        <div className="frame-wrap">
          <div className="frame">
            <FrameTop
              slot={setSlot}
              onMenu={() => {
                setSheetAt(at);
              }}
            />
            <TitleBarSlot.Provider value={slot}>
              <main className="main" id="main" role="main">
                <Disconnected />
                <Outlet />
              </main>
            </TitleBarSlot.Provider>
          </div>
        </div>
      </div>
      {sheet ? (
        <Sheet
          label="Sidebar"
          onClose={() => {
            setSheetAt(null);
          }}
        >
          <Sidebar
            onPick={() => {
              setSheetAt(null);
            }}
          />
        </Sheet>
      ) : null}
      <PaletteHost />
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
