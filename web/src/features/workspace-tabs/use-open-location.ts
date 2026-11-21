import { useRef, useState } from "react";

import { prefs } from "../../data/prefs";
import {
  addDiscovered,
  tabKey,
  type TabLayout,
  type WorkspaceTab,
} from "../../data/workspace/tab-layout";
import type {
  OpenLocationSource,
  PullRequestLocation,
} from "../../wire/bridge";
import type { RelPath } from "../../wire/proto";
import { useMedia } from "../shell/use-media";

type SupportingPlace = Exclude<PullRequestLocation, "main">;

/**
 * Both panes use the same tab registry. When a supporting tab returns to
 * the strip, the existing editor and its unsaved buffer travel with it.
 */
export function useOpenLocation(
  layout: TabLayout,
  update: (apply: (layout: TabLayout) => TabLayout) => void,
  selectMain: (tab: WorkspaceTab) => void,
) {
  const wide = useMedia("(width >= 1100px)");
  const [placed, setPlaced] = useState<{
    key: string;
    place: SupportingPlace;
  } | null>(null);
  const supporting =
    wide &&
    placed !== null &&
    layout.active !== placed.key &&
    layout.tabs.some((tab) => tabKey(tab) === placed.key)
      ? placed
      : null;

  const open = (tab: WorkspaceTab, place: PullRequestLocation) => {
    const key = tabKey(tab);
    if (
      !wide ||
      place === "main" ||
      layout.active === null ||
      layout.active === key
    ) {
      setPlaced((current) => (current?.key === key ? null : current));
      selectMain(tab);
      return;
    }
    update((current) => addDiscovered(current, [tab]));
    setPlaced({ key, place });
  };

  const [lines, setLines] = useState<
    ReadonlyMap<RelPath, { line: number; seq: number }>
  >(() => new Map());
  const nextLine = useRef(0);
  const openFile = (
    path: RelPath,
    line: number | null,
    source?: OpenLocationSource,
  ) => {
    if (line !== null) {
      nextLine.current += 1;
      const seek = { line, seq: nextLine.current };
      setLines((current) => new Map(current).set(path, seek));
    }
    const places = prefs.get().values.open_location;
    open(
      { kind: "file", path },
      source === undefined ? "main" : places[source],
    );
  };

  return {
    lines,
    openFile,
    supporting,
    forget: (key: string) => {
      setPlaced((current) => (current?.key === key ? null : current));
    },
    openChanges: (kind: "changes" | "diff") => {
      open({ kind }, prefs.get().values.open_location.diffs);
    },
    openPull: () => {
      open({ kind: "pull" }, prefs.get().values.open_location.pull_requests);
    },
    openExplorer: () => {
      open({ kind: "files" }, "explorer");
    },
  };
}
