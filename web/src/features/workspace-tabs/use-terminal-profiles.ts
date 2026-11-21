// The workspace's machine's terminal profiles for its New tab rows (ADR
// 0117): read when the workspace opens and again whenever the window comes
// back, as Paseo's config query refetches on focus, so a list changed in
// Settings shows on return.

import { useEffect, useState } from "react";

import { useFleet } from "../../context";
import { profilesServed, readProfiles } from "../../data/terminal-profiles";
import type { NodeKey } from "../../wire/bridge";
import type { TerminalProfile } from "../../wire/proto";

/** `node`'s profiles; none while unread, below their protocol, or when a
 * read failed (the rows are then left out, not shown stale). */
export function useTerminalProfiles(
  node: NodeKey,
  protocol: number | null,
  enabled: boolean,
): readonly TerminalProfile[] {
  const fleet = useFleet();
  // Kept with the machine it was read from, so another machine's list is
  // never shown while this one's is read.
  const [read, setRead] = useState<{
    node: NodeKey;
    profiles: readonly TerminalProfile[];
  } | null>(null);
  const served = enabled && profilesServed(protocol);
  useEffect(() => {
    if (!served) {
      return undefined;
    }
    let live = true;
    const load = async () => {
      const result = await readProfiles(fleet, node);
      if (live) {
        setRead({ node, profiles: result.ok ? result.value : [] });
      }
    };
    void load();
    const again = () => {
      if (document.visibilityState === "visible") {
        void load();
      }
    };
    window.addEventListener("focus", again);
    document.addEventListener("visibilitychange", again);
    return () => {
      live = false;
      window.removeEventListener("focus", again);
      document.removeEventListener("visibilitychange", again);
    };
  }, [fleet, node, served]);
  return served && read?.node === node ? read.profiles : NONE;
}

const NONE: readonly TerminalProfile[] = [];
