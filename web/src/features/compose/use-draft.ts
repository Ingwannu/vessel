// A composer's text as its draft (data/drafts.ts): what was typed comes
// back when the composer does, and is saved as it is typed, without a
// write per key.

import { useCallback, useEffect, useMemo, useState } from "react";

import { useFleet } from "../../context";
import type { DraftNote } from "../../data/drafts";
import { useStore } from "../../data/store";
import type { DraftKey } from "../../wire/bridge";
import type { SessionId } from "../../wire/proto";

export interface DraftState {
  text: string;
  setText: (text: string) => void;
  /** Show text that is not the draft (a recalled prompt): saved only once
   * it is edited, as agent-orchestrator does. */
  show: (text: string) => void;
  /** Empty it, once what it held was sent. */
  clear: () => void;
  note: DraftNote;
}

/** The draft of `session`'s composer; `null` is the new-session page's. */
export function useDraft(session: SessionId | null): DraftState {
  const fleet = useFleet();
  const drafts = fleet.drafts;
  const stable = useMemo(
    (): DraftKey =>
      session === null ? { kind: "new" } : { kind: "session", session },
    [session],
  );
  useStore(drafts.version, (count) => count);
  // A recalled prompt shown in place of the draft, until it is edited.
  const [shown, setShown] = useState<{ key: DraftKey; text: string } | null>(
    null,
  );
  useEffect(() => {
    void drafts.readOne(stable);
    return () => {
      drafts.flush();
    };
  }, [drafts, stable]);
  const setText = useCallback(
    (next: string) => {
      setShown(null);
      drafts.type(stable, next);
    },
    [drafts, stable],
  );
  const show = useCallback(
    (next: string) => {
      setShown({ key: stable, text: next });
    },
    [stable],
  );
  const clear = useCallback(() => {
    setShown(null);
    drafts.clear(stable);
  }, [drafts, stable]);
  const text =
    shown !== null && shown.key === stable ? shown.text : drafts.text(stable);
  return { text, setText, show, clear, note: drafts.note(stable) };
}
