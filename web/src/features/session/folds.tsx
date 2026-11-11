// How much of a turn the timeline shows (Settings › Appearance,
// "Conversation display"; Paseo's "Full" and "Collapse tool calls"). With
// tool calls collapsed, a run of them (and the thoughts between) is one
// row saying what was done, opened on a click; the rest of the turn is
// drawn as it is. The runs are data/tool-runs.ts's; which are open is
// this page's, not kept.

import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useMemo,
  useState,
} from "react";

import { prefs } from "../../data/prefs";
import { useStore } from "../../data/store";
import type { Entry } from "../../data/timeline-model";
import { type Folds, type ToolRun, toolRuns } from "../../data/tool-runs";
import { Icon } from "../../ui/kit";
import { TimelineItem } from "./timeline-item";

interface FoldState {
  folds: Folds | null;
  open: ReadonlySet<string>;
  toggle: (lead: string) => void;
}

const FoldContext = createContext<FoldState>({
  folds: null,
  open: new Set(),
  toggle: () => undefined,
});

/** The folds of `entries`, as Appearance says, for the timeline under it. */
export function FoldProvider({
  entries,
  children,
}: {
  entries: readonly Entry[];
  children: ReactNode;
}) {
  const display = useStore(
    prefs,
    (state) => state.values.appearance.conversation,
  );
  const folds = useMemo(
    () => (display === "collapse_tools" ? toolRuns(entries) : null),
    [display, entries],
  );
  const [open, setOpen] = useState<ReadonlySet<string>>(new Set());
  const toggle = useCallback((lead: string) => {
    setOpen((current) => {
      const next = new Set(current);
      if (!next.delete(lead)) {
        next.add(lead);
      }
      return next;
    });
  }, []);
  const value = useMemo(() => ({ folds, open, toggle }), [folds, open, toggle]);
  return <FoldContext.Provider value={value}>{children}</FoldContext.Provider>;
}

function FoldRow({
  run,
  expanded,
  onToggle,
}: {
  run: ToolRun;
  expanded: boolean;
  onToggle: () => void;
}) {
  return (
    <div className="t-item meta-item fold">
      <span className="glyph" aria-hidden="true" />
      <button
        type="button"
        className="fold-row"
        aria-expanded={expanded}
        onClick={onToggle}
      >
        <Icon name="chevron" size={12} />
        <span>{run.summary}</span>
      </button>
    </div>
  );
}

/** One entry as the conversation display draws it: itself, a run's
 * summary row, or nothing while its run is closed. */
export function TimelineEntry({ entry }: { entry: Entry }) {
  const { folds, open, toggle } = useContext(FoldContext);
  const run = folds?.get(entry.key);
  if (run === undefined) {
    return <TimelineItem entry={entry} />;
  }
  const expanded = open.has(run.lead);
  if (entry.key !== run.lead) {
    return expanded ? <TimelineItem entry={entry} /> : null;
  }
  return (
    <>
      <FoldRow
        run={run}
        expanded={expanded}
        onToggle={() => {
          toggle(run.lead);
        }}
      />
      {expanded ? <TimelineItem entry={entry} /> : null}
    </>
  );
}
