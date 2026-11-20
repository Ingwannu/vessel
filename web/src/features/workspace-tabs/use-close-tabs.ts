// One preview for single and bulk close. Recheck destructive targets
// after confirmation because another client can open terminals meanwhile.

import { useEffect, useRef, useState } from "react";

import { useFleet } from "../../context";
import { isLive } from "../../data/format";
import type { WorkspaceTab } from "../../data/workspace/tab-layout";
import {
  applyClose,
  type CloseActions,
  type CloseImpact,
  impactGrew,
  inspectClose,
} from "./close-tabs";
import type { TabWords } from "./words";

type CloseRequest =
  | { kind: "single"; tab: WorkspaceTab }
  | { kind: "bulk"; tabs: readonly WorkspaceTab[]; title: string };

interface CloseInput extends CloseActions {
  dirty: (tab: WorkspaceTab) => boolean;
  words: TabWords;
}

interface ConfirmationCopy {
  title: string;
  message: string;
  label: string;
}

interface Confirmation extends ConfirmationCopy {
  request: CloseRequest;
  impact: CloseImpact;
}

interface CloseState {
  confirmation: Confirmation | null;
  close: (request: CloseRequest) => void;
  cancel: () => void;
  confirm: () => void;
}

function confirmationFor(
  request: CloseRequest,
  impact: CloseImpact,
  { dirty, words }: CloseInput,
): ConfirmationCopy | null {
  if (request.kind === "bulk") {
    return {
      title: `${request.title}?`,
      message: words.bulk(
        impact.agents.size,
        impact.terminals.size,
        impact.passive,
        request.tabs.filter(dirty).length,
      ),
      label: words.close,
    };
  }
  const tab = request.tab;
  if (tab.kind === "terminal") {
    return {
      title: words.terminalTitle,
      message: words.terminalBody,
      label: words.close,
    };
  }
  if (tab.kind === "agent" && impact.agents.has(tab.id)) {
    if (impact.terminals.size > 0) {
      return {
        title: words.archiveAgentTitle,
        message: words.bulk(1, impact.terminals.size, 0, 0),
        label: words.archive,
      };
    }
    const session = impact.agents.get(tab.id);
    if (session !== undefined && isLive(session)) {
      return {
        title: words.agentTitle,
        message: words.agentBody,
        label: words.archive,
      };
    }
  }
  return dirty(tab)
    ? { title: words.fileTitle, message: words.fileBody, label: words.close }
    : null;
}

export function useCloseTabs(input: CloseInput): CloseState {
  const fleet = useFleet();
  const [confirmation, setConfirmation] = useState<Confirmation | null>(null);
  const pending = useRef(false);
  const present = useRef(true);
  useEffect(() => {
    present.current = true;
    return () => {
      present.current = false;
    };
  }, []);

  const close = async (request: CloseRequest, accepted: CloseImpact | null) => {
    const tabs = request.kind === "single" ? [request.tab] : request.tabs;
    if (pending.current || tabs.length === 0) {
      return;
    }
    pending.current = true;
    try {
      const impact = await inspectClose(fleet, input, tabs);
      if (!present.current || impact === null) {
        return;
      }
      const copy = confirmationFor(request, impact, input);
      if (
        copy !== null &&
        (accepted === null || impactGrew(accepted, impact))
      ) {
        setConfirmation({ ...copy, request, impact });
        return;
      }
      await applyClose(fleet, input, tabs, impact);
    } finally {
      pending.current = false;
    }
  };

  return {
    confirmation,
    close: (request) => void close(request, null),
    cancel: () => {
      setConfirmation(null);
    },
    confirm: () => {
      if (confirmation !== null) {
        setConfirmation(null);
        void close(confirmation.request, confirmation.impact);
      }
    },
  };
}
