// One machine's terminal profiles as Settings › Terminals holds them (ADR
// 0117): the kept list (the shipped four when none is kept), and the save in
// flight. Every change sends the whole list, as Paseo's `patchConfig` does
// (`host-page.tsx` 1438-1443); the section is drawn from what the node
// answered, so a refused change leaves the rows as they were.

import { useCallback, useEffect, useRef, useState } from "react";

import { useFleet } from "../../context";
import type { Outcome } from "../../data/outcome";
import { readProfiles, saveProfiles } from "../../data/terminal-profiles";
import type { NodeKey } from "../../wire/bridge";
import type { TerminalProfile } from "../../wire/proto";

export interface ProfilesHold {
  listed: Outcome<readonly TerminalProfile[]> | null;
  saving: boolean;
  /** Keep `next` as the whole list; resolves with the node's answer. */
  save: (
    next: readonly TerminalProfile[],
  ) => Promise<Outcome<readonly TerminalProfile[]>>;
}

export function useTerminalProfileSettings(
  machine: NodeKey,
  supported: boolean,
): ProfilesHold {
  const fleet = useFleet();
  const [listed, setListed] = useState<Outcome<
    readonly TerminalProfile[]
  > | null>(null);
  const [saving, setSaving] = useState(false);
  // Only the newest read or save is kept.
  const asked = useRef(0);

  useEffect(() => {
    if (!supported) {
      return;
    }
    asked.current += 1;
    const mine = asked.current;
    void readProfiles(fleet, machine).then((result) => {
      if (mine === asked.current) {
        setListed(result);
      }
    });
  }, [fleet, machine, supported]);

  const save = useCallback(
    async (next: readonly TerminalProfile[]) => {
      setSaving(true);
      const result = await saveProfiles(fleet, machine, next);
      // The save ends whatever was asked meanwhile: a read started during it
      // (a reconnect) must not leave the controls waiting (P5w node review
      // note 1). Its answer is the list in force after it, so a read still
      // out is dropped.
      setSaving(false);
      if (result.ok) {
        asked.current += 1;
        setListed(result);
      }
      return result;
    },
    [fleet, machine],
  );

  return { listed, saving, save };
}
