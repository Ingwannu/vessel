// The composer's `/` and `@` menu (milestone 11, Track A-page): what the
// caret is in decides the list (data/suggest.ts); `/` lists the agent's
// commands as `commands_changed` last said, `@` searches the session's
// files through the one wrapper of `file.find` (data/workspace/find.ts).
// The text keeps focus and its keys drive the list.
//
// [Decision Log]
// - Purpose: agent-orchestrator's, GOAT's and Paseo's one suggestion
//   menu for commands and files.
// - Constraint: the text must keep focus (typing narrows the list), and
//   a search runs against a repository of any size (use-file-search.ts).
// - Choice: the list is the kit's, its state on the text (a combobox);
//   Up and Down move, Enter or Tab insert, Escape hides the list until
//   the word changes; a session with no project, a command list the agent
//   has not sent, a search under way or empty each say so in the list.

import {
  type KeyboardEvent,
  type ReactNode,
  type RefObject,
  useEffect,
  useState,
} from "react";

import {
  applyCommand,
  applyFile,
  matchCommands,
  pathParts,
  type Trigger,
  triggerAt,
} from "../../data/suggest";
import {
  moveActive,
  suggestionId,
  type SuggestionItem,
  Suggestions,
} from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { AgentCommand, WorkspaceRoot } from "../../wire/proto";
import { useFileSearch } from "./use-file-search";

const LIST_ID = "composer-suggestions";

export interface SuggestProps {
  node: NodeKey;
  /** The agent's commands; `null` until it said. */
  commands: readonly AgentCommand[] | null;
  /** Where `@` searches; `null` for a session with no project. */
  root: WorkspaceRoot | null;
  agent: string;
  text: RefObject<HTMLTextAreaElement | null>;
  /** Write a choice into the text, the caret after it. */
  apply: (text: string, caret: number) => void;
}

export interface Suggest {
  menu: ReactNode | undefined;
  onKey: (event: KeyboardEvent<HTMLTextAreaElement>) => boolean;
  onCaret: (text: string, caret: number) => void;
}

function highlighted(name: string, marks: ReadonlySet<number>): ReactNode {
  return Array.from(name).map((character, index) =>
    marks.has(index) ? <mark key={index}>{character}</mark> : character,
  );
}

export function useSuggest(props: SuggestProps): Suggest {
  const { node, commands, root, agent, text, apply } = props;
  const [trigger, setTrigger] = useState<Trigger | null>(null);
  const [hidden, setHidden] = useState<number | null>(null);
  const [active, setActive] = useState(0);

  const query = trigger?.kind === "file" ? trigger.query : null;
  const found = useFileSearch(node, root, query);
  const shown = trigger !== null && hidden !== trigger.start;
  let items: SuggestionItem[] = [];
  let note: string | undefined;
  let footer: string | undefined;
  let choose: ((index: number) => void) | null = null;
  if (shown && trigger.kind === "command") {
    const matched =
      commands === null ? [] : matchCommands(commands, trigger.query);
    items = matched.map((command) => ({
      key: command.name,
      label: `/${command.name}`,
      detail:
        command.hint === undefined || command.hint === null
          ? command.description
          : `${command.description} · ${command.hint}`,
    }));
    note =
      commands === null
        ? `No commands from ${agent} yet`
        : commands.length === 0
          ? `${agent} offers no commands`
          : items.length === 0
            ? "No matching command."
            : undefined;
    choose = (index) => {
      const command = matched[index];
      const area = text.current;
      if (command !== undefined && area !== null) {
        const next = applyCommand(area.value, trigger, command.name);
        apply(next.text, next.caret);
      }
    };
  } else if (shown && trigger.kind === "file") {
    const files =
      found?.kind === "found" && found.query === trigger.query
        ? found.files
        : [];
    items = files.map((file) => {
      const parts = pathParts(file);
      return {
        key: file.path,
        label: highlighted(parts.name, parts.marks),
        detail: parts.folder === "" ? undefined : parts.folder,
        icon: "file",
      };
    });
    note =
      root === null
        ? "This session has no project to search."
        : trigger.query.trim() === ""
          ? "Type to search files"
          : found?.query !== trigger.query || found.kind === "searching"
            ? "Searching files…"
            : found.kind === "failed"
              ? found.message
              : items.length === 0
                ? "No matching files or folders."
                : undefined;
    footer =
      found?.kind === "found" && !found.complete && items.length > 0
        ? "Showing part of a large project. Type more to narrow it."
        : undefined;
    choose = (index) => {
      const file = files[index];
      const area = text.current;
      if (file !== undefined && area !== null) {
        const next = applyFile(area.value, trigger, file.path);
        apply(next.text, next.caret);
      }
    };
  }
  const open = shown && (items.length > 0 || note !== undefined);
  const current = Math.min(active, Math.max(0, items.length - 1));

  // The list belongs to the text, which keeps its own role (a textarea
  // takes no other) and names the list and its active row: the textbox's
  // own `aria-autocomplete`, `aria-controls` and `aria-activedescendant`,
  // set on the element the kit gives by ref.
  useEffect(() => {
    const area = text.current;
    if (area === null) {
      return;
    }
    area.setAttribute("aria-autocomplete", "list");
    if (open && items.length > 0) {
      area.setAttribute("aria-controls", LIST_ID);
      area.setAttribute(
        "aria-activedescendant",
        suggestionId(LIST_ID, current),
      );
    } else {
      area.removeAttribute("aria-controls");
      area.removeAttribute("aria-activedescendant");
    }
  }, [text, open, items.length, current]);

  const pick = (index: number) => {
    choose?.(index);
    setTrigger(null);
    setActive(0);
  };
  return {
    menu: open ? (
      <Suggestions
        id={LIST_ID}
        label={trigger.kind === "command" ? "Commands" : "Files"}
        items={items}
        active={current}
        {...(note === undefined ? {} : { note })}
        {...(footer === undefined ? {} : { footer })}
        onPick={pick}
        onHover={setActive}
      />
    ) : undefined,
    onKey: (event) => {
      if (!open || event.nativeEvent.isComposing) {
        return false;
      }
      if (event.key === "Escape") {
        setHidden(trigger.start);
        return true;
      }
      if (items.length === 0) {
        return false;
      }
      switch (event.key) {
        case "ArrowDown":
          setActive(moveActive(current, 1, items.length));
          return true;
        case "ArrowUp":
          setActive(moveActive(current, -1, items.length));
          return true;
        case "Enter":
        case "Tab":
          if (event.shiftKey) {
            return false;
          }
          pick(current);
          return true;
        default:
          return false;
      }
    },
    onCaret: (value, caret) => {
      const next = triggerAt(value, caret);
      setTrigger((before) =>
        before?.start === next?.start &&
        before?.kind === next?.kind &&
        before?.query === next?.query
          ? before
          : next,
      );
      if (next?.start !== hidden) {
        setHidden(null);
      }
      if (next?.query !== trigger?.query) {
        setActive(0);
      }
    },
  };
}
