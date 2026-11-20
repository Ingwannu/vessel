// The standalone page owns the bridge's saved blank draft. A workspace
// draft has its own lifetime so it cannot replace that text or vice versa.

import { useState } from "react";

import type { SessionId, SessionPlace } from "../../wire/proto";
import { useDraft, type DraftState } from "./use-draft";

export interface WorkspaceDraft {
  place: SessionPlace | null;
  folder: string | null;
  tray: string;
  done: (id: SessionId) => void;
  onDirtyChange: (dirty: boolean) => void;
}

type NewSessionDraft = Pick<DraftState, "text" | "setText" | "clear"> & {
  note: DraftState["note"] | undefined;
};

export function useNewSessionDraft(
  markDirty?: (dirty: boolean) => void,
): NewSessionDraft {
  const saved = useDraft(null);
  const [local, setLocal] = useState("");
  if (markDirty === undefined) {
    return saved;
  }
  return {
    text: local,
    note: undefined,
    setText: (value: string) => {
      setLocal(value);
      markDirty(value !== "");
    },
    clear: () => {
      setLocal("");
      markDirty(false);
    },
  };
}
