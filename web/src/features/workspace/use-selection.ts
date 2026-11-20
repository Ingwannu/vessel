// Lines selected in the code panel (a diff's or a file's), and where the
// "Add to chat" button sits for them (agent-orchestrator's
// `SelectionAskButton`: beside the selection's end, 6px right, 14px up).
// Sending puts the reference in the session's draft, the one the
// composer already keeps (data/drafts.ts), never through a channel of
// its own.

import {
  type RefObject,
  useCallback,
  useContext,
  useEffect,
  useState,
} from "react";

import { useFleet } from "../../context";
import {
  diffReference,
  fileReference,
  type SelectedLine,
  withReference,
} from "../../data/workspace/reference";
import { WorkspaceChat } from "./workspace-chat";
import type { SessionId } from "../../wire/proto";

/** A selection the button would send, and where to draw the button. */
export interface Ask {
  x: number;
  y: number;
  text: string;
}

/** The kinds a diff line's element carries in `data-kind`. */
const LINE_KINDS: readonly SelectedLine["kind"][] = [
  "added",
  "removed",
  "context",
  "no_newline",
];

/** A diff line's kind from the element's mark, when it is one. */
function kindOf(value: string | undefined): SelectedLine["kind"] | null {
  return LINE_KINDS.find((kind) => kind === value) ?? null;
}

function numberOf(value: string | null): number | null {
  if (value === null || value === "") {
    return null;
  }
  const parsed = Number(value);
  return Number.isInteger(parsed) ? parsed : null;
}

/** The reference for the drawn lines `range` touches, in the file the
 * selection starts in; `null` when it touches none. */
function referenceOf(
  container: HTMLElement,
  range: Range,
  fence: (path: string) => string,
): string | null {
  const lines = Array.from(
    container.querySelectorAll<HTMLElement>("[data-path][data-kind]"),
  ).filter((line) => range.intersectsNode(line));
  const path = lines[0]?.dataset["path"];
  if (path === undefined) {
    return null;
  }
  const mine = lines.filter((line) => line.dataset["path"] === path);
  const code = (line: HTMLElement) =>
    line.querySelector(".d-code, .f-code")?.textContent ?? "";
  if (mine.every((line) => line.dataset["kind"] === "file")) {
    const start = numberOf(mine[0]?.dataset["new"] ?? null) ?? 1;
    return fileReference(path, start, mine.map(code), fence(path));
  }
  const selected = mine.flatMap((line): SelectedLine[] => {
    const kind = kindOf(line.dataset["kind"]);
    if (kind === null) {
      return [];
    }
    const number =
      kind === "removed"
        ? numberOf(line.dataset["old"] ?? null)
        : numberOf(line.dataset["new"] ?? null);
    return [{ kind, text: code(line), line: number }];
  });
  return selected.length === 0 ? null : diffReference(path, selected);
}

/** Watch selections inside `container` and offer to send them. */
export function useSelectionAsk(
  container: RefObject<HTMLElement | null>,
  session: SessionId,
  fence: (path: string) => string,
): { ask: Ask | null; send: () => void; dismiss: () => void } {
  const fleet = useFleet();
  const openChat = useContext(WorkspaceChat);
  const [ask, setAsk] = useState<Ask | null>(null);
  useEffect(() => {
    const element = container.current;
    if (element === null) {
      return undefined;
    }
    const look = () => {
      const selection = window.getSelection();
      if (
        selection === null ||
        selection.isCollapsed ||
        selection.rangeCount === 0 ||
        !element.contains(selection.anchorNode)
      ) {
        setAsk(null);
        return;
      }
      const range = selection.getRangeAt(0);
      const text = referenceOf(element, range, fence);
      const rect = range
        .getClientRects()
        .item(range.getClientRects().length - 1);
      setAsk(
        text === null || rect === null
          ? null
          : { x: rect.right + 6, y: rect.top - 14, text },
      );
    };
    const clear = () => {
      setAsk(null);
    };
    element.addEventListener("mouseup", look);
    element.addEventListener("keyup", look);
    element.addEventListener("scroll", clear, { passive: true });
    return () => {
      element.removeEventListener("mouseup", look);
      element.removeEventListener("keyup", look);
      element.removeEventListener("scroll", clear);
    };
  }, [container, fence]);
  const send = useCallback(() => {
    if (ask === null) {
      return;
    }
    const key = { kind: "session" as const, session };
    fleet.drafts.type(key, withReference(fleet.drafts.text(key), ask.text));
    window.getSelection()?.removeAllRanges();
    setAsk(null);
    openChat?.();
  }, [ask, fleet, openChat, session]);
  const dismiss = useCallback(() => {
    setAsk(null);
  }, []);
  return { ask, send, dismiss };
}
