// "Open on <forge>" on a session's bar (TRACKER 467 step B, ADR 0123
// "Step B"): Paseo's open-in-editor button as a browser draws it
// (`open-in-editor/button.tsx`, pinned 0.15.4). A browser has no desktop
// bridge, so its one target is the forge's page (`:112` shows it on the
// web only; `:259` draws the caret only with more than one target), the
// forge's mark in the muted colour (`:56`), named "Open workspace in
// GitHub" or "Open <file> in GitHub" (`:229-237`).
//
// [Decision Log]
// - Purpose: the person reaches the branch, or the file they are reading
//   at its line, on the forge, as in Paseo.
// - Constraint: the node sends `origin`'s host and path on a status only
//   from `ORIGIN_LINK_PROTOCOL` on; an older node sends none, and then
//   nothing is drawn (Paseo with an old daemon: no `remoteUrl`, no forge
//   target).
// - Choice: read the session's status when the bar mounts, when its folder
//   changes, and every 30 s while the page is visible (an agent may switch
//   the branch); the link is built here (`data/forge-links.ts`) and opened
//   in a new isolated tab, `https://` only.
// - Cost: one `git.status` per bar every 30 s, which on the node is one
//   more git run for `origin`.

import "./forge-open.css";

import { useEffect, useState } from "react";

import { useFleet } from "../../context";
import {
  forgeLink,
  type LinkedFile,
  openForgePage,
} from "../../data/forge-links";
import { readStatus } from "../../data/workspace/git";
import { ForgeMark } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { GitStatus, WorkspaceRoot } from "../../wire/proto";

/** How often the bar reads its status again while the page is visible. */
const FRESH_MS = 30_000;

const en = {
  openIn: (target: string) => `Open workspace in ${target}`,
  openFileIn: (file: string, target: string) => `Open ${file} in ${target}`,
};

const ko: typeof en = {
  openIn: (target) => `${target}에서 워크스페이스 열기`,
  openFileIn: (file, target) => `${target}에서 ${file} 열기`,
};

/** Paseo's words (`openInEditor.openIn`, `openFileIn`). */
export function forgeOpenWords(language: string): typeof en {
  return language.toLowerCase().startsWith("ko") ? ko : en;
}

/** A status as read, and for which folder of which node. */
interface Read {
  node: NodeKey;
  root: WorkspaceRoot;
  status: GitStatus | null;
}

/** `root`'s status, read again every [`FRESH_MS`] while the page shows;
 * null before the first answer for this folder or when it failed. */
function useStatus(node: NodeKey, root: WorkspaceRoot | null): GitStatus | null {
  const fleet = useFleet();
  const [read, setRead] = useState<Read | null>(null);
  useEffect(() => {
    if (root === null) {
      return;
    }
    let live = true;
    const ask = () => {
      void readStatus(fleet, node, root).then((outcome) => {
        if (live) {
          setRead({ node, root, status: outcome.ok ? outcome.value : null });
        }
      });
    };
    ask();
    const timer = setInterval(() => {
      if (document.visibilityState === "visible") {
        ask();
      }
    }, FRESH_MS);
    return () => {
      live = false;
      clearInterval(timer);
    };
  }, [fleet, node, root]);
  // An answer for another folder (the session moved) is not this one's.
  return read !== null && read.node === node && read.root === root
    ? read.status
    : null;
}

export function ForgeOpen({
  node,
  root,
  file,
}: {
  node: NodeKey;
  /** The session's folder; none for a session outside a project. */
  root: WorkspaceRoot | null;
  /** The file its code panel shows, at the line it was opened at. */
  file: LinkedFile | null;
}) {
  const status = useStatus(node, root);
  const link =
    status === null ? null : forgeLink(status.origin, status.branch, file);
  if (link === null) {
    return null;
  }
  const words = forgeOpenWords(navigator.language);
  const name =
    file === null
      ? words.openIn(link.name)
      : words.openFileIn(file.path.split("/").at(-1) ?? file.path, link.name);
  return (
    <button
      type="button"
      className="btn icon-btn edged sm forge-open"
      aria-label={name}
      onClick={() => {
        openForgePage(link.url);
      }}
    >
      <ForgeMark forge={link.forge} size={16} />
    </button>
  );
}
