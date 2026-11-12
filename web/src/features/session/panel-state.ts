// Which tab a session's panel shows and which file its Files tab has
// open, kept by the session's page (a new session starts on Summary).
// Opening a file from anywhere (a diff's header, a search) shows Files.
// The branch's chip opens Changes: in the inspector where the window is
// wide enough for it (1280px, session-page.css), else in a sheet from the
// right, so a phone reaches the code panel too.

import { useCallback, useState, useSyncExternalStore } from "react";

import { useFleet } from "../../context";
import { prefer } from "../../data/prefs";
import type { Opened } from "../workspace/files-tab";

/** The panel's tabs. */
export type PanelTab = "summary" | "changes" | "files";

/** Below this window width the inspector is not drawn (session-page.css). */
const NARROW = "(max-width: 1279px)";

function onWidth(change: () => void): () => void {
  const query = window.matchMedia(NARROW);
  query.addEventListener("change", change);
  return () => {
    query.removeEventListener("change", change);
  };
}

function narrowNow(): boolean {
  return window.matchMedia(NARROW).matches;
}

export interface Panel {
  tab: PanelTab;
  setTab: (tab: PanelTab) => void;
  opened: Opened | null;
  setOpened: (opened: Opened | null) => void;
  /** Whether the window is too narrow for the inspector: it is then not
   * mounted at all (hidden by the stylesheet alone, its tabs would go on
   * reading the node beside the sheet's). */
  narrow: boolean;
  /** Whether the panel is open as a sheet (a narrow window). */
  sheet: boolean;
  closeSheet: () => void;
  /** Show the session's changes, wherever the panel can be. */
  openChanges: () => void;
}

export function usePanel(inspector: "shown" | "hidden"): Panel {
  const fleet = useFleet();
  const [tab, setTab] = useState<PanelTab>("summary");
  const [opened, setOpenedState] = useState<Opened | null>(null);
  const [sheet, setSheet] = useState(false);
  const narrow = useSyncExternalStore(onWidth, narrowNow);
  const setOpened = useCallback((next: Opened | null) => {
    setOpenedState(next);
    if (next !== null) {
      setTab("files");
    }
  }, []);
  const openChanges = useCallback(() => {
    setTab("changes");
    if (narrowNow()) {
      setSheet(true);
    } else if (inspector !== "shown") {
      void prefer(fleet, { set: "inspector_panel", value: "shown" });
    }
  }, [fleet, inspector]);
  const closeSheet = useCallback(() => {
    setSheet(false);
  }, []);
  return {
    tab,
    setTab,
    opened,
    setOpened,
    narrow,
    sheet: sheet && narrow,
    closeSheet,
    openChanges,
  };
}
