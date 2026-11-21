import "./scripts-control.css";

import { useMemo, useState } from "react";
import { compareApproved } from "../../data/workspace/command-changes";
import {
  scriptIsLive,
  type ScriptRow,
  scriptRows,
  scriptStatus,
} from "../../data/workspace/scripts";
import {
  Alert,
  Button,
  Dialog,
  Icon,
  IconButton,
  Popover,
  Row,
  RowList,
  Tooltip,
} from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type {
  ProjectId,
  ScriptPlace,
  SessionId,
  TerminalId,
  TerminalInfo,
} from "../../wire/proto";
import { ApprovalDialog } from "../projects/approval-dialog";
import { useScripts } from "./use-scripts";

const ACTIONS = {
  run: { label: "Run", icon: "exec" },
  restart: { label: "Restart", icon: "refresh" },
  stop: { label: "Stop", icon: "stop" },
  view: { label: "View terminal", icon: "terminal" },
} as const;
type RowAction = keyof typeof ACTIONS;

function rowActions(row: ScriptRow): RowAction[] {
  if (row.terminal === null || !scriptIsLive(row.terminal)) return ["run"];
  return row.configured ? ["view", "restart", "stop"] : ["view", "stop"];
}

export function ScriptsControl({
  node,
  project,
  session,
  compact,
  terminals,
  onTerminal,
}: {
  node: NodeKey;
  project: ProjectId;
  session: SessionId | null;
  compact: boolean;
  terminals: readonly TerminalInfo[];
  onTerminal: (terminal: TerminalId) => void;
}) {
  const [open, setOpen] = useState(false);
  const [detail, setDetail] = useState<string | null>(null);
  const place = useMemo<ScriptPlace>(
    () =>
      session === null
        ? { kind: "project", project }
        : { kind: "session", session },
    [project, session],
  );
  const activity = terminals
    .filter(
      (terminal) =>
        terminal.kind.kind === "script" || terminal.kind.kind === "service",
    )
    .map(({ id, revision }) => `${id}:${String(revision)}`)
    .join(",");
  const show = (next: boolean) => {
    setOpen(next);
    setDetail(null);
  };
  const view = (terminal: TerminalId) => {
    show(false);
    onTerminal(terminal);
  };
  const scripts = useScripts({
    node,
    project,
    place,
    open,
    activity,
    onTerminal: view,
  });
  const rows = scripts.snapshot === null ? [] : scriptRows(scripts.snapshot);
  const configProblem =
    scripts.snapshot?.commands.state === "invalid"
      ? scripts.snapshot.commands.message
      : null;
  const error = scripts.error ?? scripts.listError ?? configProblem;
  const selected = rows.find((row) => row.name === detail);
  const approval = scripts.approval;
  if (rows.length === 0 && error === null && !open && approval === null)
    return null;

  const act = (action: RowAction, row: ScriptRow) => {
    if (action === "run") {
      scripts.run({ kind: "run", name: row.name });
    } else if (row.terminal !== null) {
      const terminal = row.terminal.id;
      switch (action) {
        case "view":
          view(terminal);
          break;
        case "restart":
          scripts.run({ kind: "restart", terminal });
          break;
        case "stop":
          scripts.stop(terminal);
          break;
      }
    }
  };
  const active = rows.some((row) => scriptIsLive(row.terminal));
  const content = (
    <div className="workspace-script-content">
      {error === null ? null : (
        <Alert description={error} variant="error" size="xs" />
      )}
      {compact && selected !== undefined ? (
        <div className="workspace-script-detail">
          <Button
            size="sm"
            variant="ghost"
            icon="back"
            onClick={() => {
              setDetail(null);
            }}
          >
            Scripts
          </Button>
          <p>{scriptStatus(selected)}</p>
          {rowActions(selected).map((action) => (
            <Button
              key={action}
              icon={ACTIONS[action].icon}
              disabled={scripts.busy || scripts.listError !== null}
              onClick={() => {
                act(action, selected);
              }}
            >
              {ACTIONS[action].label}
            </Button>
          ))}
        </div>
      ) : (
        <RowList label="Workspace scripts">
          {rows.map((row) => (
            <Row
              key={row.name}
              mark={{ icon: "exec" }}
              title={
                compact ? (
                  <Button
                    variant="ghost"
                    size="sm"
                    disabled={scripts.busy || scripts.listError !== null}
                    onClick={() => {
                      if (scriptIsLive(row.terminal)) setDetail(row.name);
                      else act("run", row);
                    }}
                  >
                    {row.name}
                  </Button>
                ) : (
                  row.name
                )
              }
              detail={{
                description: `${row.kind === "service" ? "Service · " : ""}${scriptStatus(row)}`,
              }}
              control={
                compact ? null : (
                  <div className="workspace-script-actions">
                    {rowActions(row).map((action) => (
                      <Tooltip key={action} label={ACTIONS[action].label}>
                        <IconButton
                          label={`${ACTIONS[action].label} ${row.name}`}
                          icon={ACTIONS[action].icon}
                          size="xs"
                          disabled={scripts.busy || scripts.listError !== null}
                          onClick={() => {
                            act(action, row);
                          }}
                        />
                      </Tooltip>
                    ))}
                  </div>
                )
              }
            />
          ))}
        </RowList>
      )}
    </div>
  );
  return (
    <>
      <span className="workspace-script-trigger" data-running={active}>
        {compact ? (
          <IconButton
            label="Scripts"
            icon="exec"
            size="sm"
            onClick={() => {
              show(true);
            }}
          />
        ) : (
          <Popover
            title="Scripts"
            showTitle={false}
            width="sm"
            align="end"
            open={open && approval === null}
            onOpenChange={show}
            trigger={
              <Button
                aria-label="Scripts"
                variant="ghost"
                size="sm"
                icon="exec"
                end={<Icon name="chevron-down" size={14} />}
              >
                {null}
              </Button>
            }
          >
            {content}
          </Popover>
        )}
      </span>
      {compact && open && approval === null ? (
        <Dialog
          title={selected?.name ?? "Scripts"}
          sheet="content"
          onClose={() => {
            show(false);
          }}
        >
          {content}
        </Dialog>
      ) : null}
      {approval === null ? null : (
        <ApprovalDialog
          commands={approval.commands}
          compared={compareApproved(approval.commands, null)}
          where="in this checkout"
          busy={scripts.busy}
          error={scripts.error}
          onApprove={scripts.approve}
          onClose={scripts.clearApproval}
        />
      )}
    </>
  );
}
