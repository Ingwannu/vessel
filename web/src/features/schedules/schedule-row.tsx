import { useFleet, useFleetState } from "../../context";
import { useNow } from "../../data/clock";
import {
  isHeartbeat,
  rowActions,
  type ScheduleAction,
  type ScheduleRow,
  scheduleMeta,
  scheduleTarget,
  scheduleTitle,
  stateLabel,
} from "../../data/schedules/rows";
import {
  projectAt,
  projectIn,
  useProjects,
} from "../../data/workspace/project-store";
import {
  Badge,
  IconButton,
  Menu,
  MenuItem,
  MenuSeparator,
  ProviderMark,
  Row,
} from "../../ui/kit";
import type { IconName } from "../../ui/kit";
import type { HarnessRef, SessionInfo } from "../../wire/proto";

const NO_SESSIONS: readonly SessionInfo[] = [];

const ACTIONS = {
  pause: { icon: "pause", label: "Pause schedule", pending: "Pausing…" },
  resume: { icon: "play", label: "Resume schedule", pending: "Resuming…" },
  run: { icon: "play", label: "Run now", pending: "Starting…" },
} satisfies Record<
  Exclude<ScheduleAction, "delete">,
  { icon: IconName; label: string; pending: string }
>;

function providerMark(harness: HarnessRef | undefined) {
  if (typeof harness !== "string") return { icon: "bot" as const };
  switch (harness) {
    case "claude-code":
      return { node: <ProviderMark name="claude" size={16} /> };
    case "codex":
      return { node: <ProviderMark name="codex" size={16} /> };
    case "open-code":
      return { node: <ProviderMark name="opencode" size={16} /> };
    case "gemini":
      return { icon: "bot" as const };
  }
}

export function ScheduleListRow({
  row,
  machine,
  act,
}: {
  row: ScheduleRow;
  machine: string | null;
  act: (action: ScheduleAction) => void;
}) {
  const fleet = useFleet();
  const now = useNow();
  const sessions = useFleetState(
    (state) => state.sessions[row.node] ?? NO_SESSIONS,
  );
  const spec =
    row.info.definition.target.kind === "new_session"
      ? row.info.definition.target.spec
      : null;
  const projects = useProjects(fleet, spec === null ? null : row.node);
  const project =
    spec?.place?.kind === "project"
      ? projectIn(projects, spec.place.project)
      : projectAt(projects, spec?.cwd);
  const target = scheduleTarget(row.info, sessions, project?.name ?? null);
  const heartbeat = isHeartbeat(row.info);
  const title = scheduleTitle(row.info);
  const state =
    row.pending === "pause"
      ? "Paused"
      : row.pending === "resume"
        ? "Active"
        : stateLabel(row.info.state);
  const facts = scheduleMeta(row.info, machine, now, row.pending);
  const actions = rowActions(row.info);
  return (
    <Row
      mark={providerMark(target.harness)}
      title={title}
      detail={{
        description: (
          <>
            <div>{target.label}</div>
            <div className="row-meta">{facts.join(" · ")}</div>
          </>
        ),
      }}
      control={
        <Badge
          tone={
            state === "Active"
              ? "ok"
              : state === "Target gone" || state === "Creator gone"
                ? "danger"
                : "neutral"
          }
        >
          {state}
        </Badge>
      }
      menu={
        <Menu
          align="end"
          width="actions"
          trigger={<IconButton label={`Actions for ${title}`} icon="more" />}
        >
          {actions
            .filter((action) => action !== "delete")
            .map((action) => (
              <MenuItem
                key={action}
                icon={ACTIONS[action].icon}
                disabled={row.pending !== undefined}
                onSelect={() => {
                  act(action);
                }}
              >
                {row.pending === action
                  ? ACTIONS[action].pending
                  : ACTIONS[action].label}
              </MenuItem>
            ))}
          {actions.length > 1 ? <MenuSeparator /> : null}
          <MenuItem
            icon="trash"
            tone="danger"
            disabled={row.pending !== undefined}
            onSelect={() => {
              act("delete");
            }}
          >
            Delete {heartbeat ? "heartbeat" : "schedule"}
          </MenuItem>
        </Menu>
      }
    />
  );
}
