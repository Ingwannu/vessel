import "./schedules.css";

import { useEffect, useMemo, useState } from "react";

import { useCrumbs, useFleet, useFleetState } from "../../context";
import { machineState } from "../../data/machines";
import { machineMeant, nodeLabel } from "../../data/nodes";
import { aggregate, visibleRows } from "../../data/schedules/aggregate";
import { editable } from "../../data/schedules/form";
import {
  MachineSchedules,
  supportsSchedules,
} from "../../data/schedules/machine-schedules";
import {
  rowKey,
  type ScheduleAction,
  type ScheduleBucket,
  type ScheduleRow,
} from "../../data/schedules/rows";
import { schedulesLabel } from "../../data/schedules/words";
import { useStore } from "../../data/store";
import {
  Alert,
  Button,
  EmptyState,
  HostFilter,
  RowList,
  Segmented,
  Spinner,
} from "../../ui/kit";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import { DeleteSchedule } from "./delete-schedule";
import { ScheduleDetail } from "./schedule-detail";
import { ScheduleForm } from "./schedule-form";
import { ScheduleHeartbeat } from "./schedule-heartbeat";
import { ScheduleListRow } from "./schedule-row";

const FILTERS = [
  { value: "active", label: "Active" },
  { value: "ended", label: "Ended" },
] as const;

const ALL_HOSTS = "";

type OpenSheet =
  { kind: "create"; node: NodeKey } | { kind: "row"; row: ScheduleRow };

export function Schedules() {
  const fleet = useFleet();
  const [schedules] = useState(() => new MachineSchedules(fleet.bridge));
  const reads = useStore(schedules.store, (state) => state);
  const connections = useFleetState((state) => state.statuses);
  const selected = useFleetState((state) => state.selected);
  const [sheet, setSheet] = useState<OpenSheet | null>(null);
  const [status, setStatus] = useState<ScheduleBucket>("active");
  const [chosenHost, setHost] = useState<NodeKey | null>(null);
  const [deleting, setDeleting] = useState<ScheduleRow | null>(null);
  const heading = schedulesLabel(navigator.language);
  useCrumbs([{ label: heading }]);

  useEffect(() => {
    const sync = () => {
      const state = fleet.store.get();
      schedules.sync(state);
      setHost((chosen) =>
        state.book?.some((node) => node.key === chosen) === true
          ? chosen
          : null,
      );
    };
    const unsubscribe = fleet.store.subscribe(sync);
    sync();
    return () => {
      unsubscribe();
      schedules.stop();
    };
  }, [fleet, schedules]);

  const book = reads.book ?? [];
  const host = book.some((node) => node.key === chosenHost) ? chosenHost : null;
  const view = useMemo(() => aggregate(reads), [reads]);

  const run = (row: ScheduleRow, action: ScheduleAction) => {
    void schedules
      .act(row.node, row.info.id, action)
      .catch((error: unknown) => {
        const detail = error instanceof Error ? error.message : String(error);
        toast(`Could not ${action} schedule: ${detail}`, "error");
      });
  };
  const onAction = (row: ScheduleRow, action: ScheduleAction) => {
    if (action === "delete") {
      setDeleting(row);
    } else {
      run(row, action);
    }
  };
  const retry = () => {
    if (reads.bookError !== null) void fleet.loadBook();
    schedules.retry();
  };

  const closeSheet = () => {
    setSheet(null);
  };
  const saved = (node: NodeKey) => {
    schedules.refresh(node);
    closeSheet();
  };
  // Keep the draft mounted across disconnect and retry. The list's
  // connection state is independent of the dialog's lifetime.
  const dialogs = (
    <>
      {sheet?.kind === "create" ? (
        <ScheduleForm
          initialNode={sheet.node}
          close={closeSheet}
          saved={saved}
        />
      ) : sheet?.kind === "row" ? (
        sheet.row.heartbeatRoute !== undefined ? (
          <ScheduleHeartbeat
            row={sheet.row}
            schedules={schedules}
            available={schedules.heartbeatAvailable(sheet.row)}
            close={closeSheet}
          />
        ) : editable(sheet.row.info) ? (
          <ScheduleForm
            initialNode={sheet.row.node}
            edit={sheet.row.info}
            close={closeSheet}
            saved={saved}
          />
        ) : (
          <ScheduleDetail row={sheet.row} close={closeSheet} />
        )
      ) : null}
      {deleting === null ? null : (
        <DeleteSchedule
          row={deleting}
          close={() => {
            setDeleting(null);
          }}
          confirm={() => {
            setDeleting(null);
            run(deleting, "delete");
          }}
        />
      )}
    </>
  );

  if (view.phase === "loading") {
    return (
      <>
        <section className="schedules-centered" aria-label={heading}>
          <Spinner label="Loading schedules" size={16} />
        </section>
        {dialogs}
      </>
    );
  }
  if (view.phase === "failed") {
    return (
      <>
        <section className="schedules-centered" aria-label={heading}>
          <p>Unable to load schedules</p>
          <Button variant="ghost" onClick={retry}>
            Try again
          </Button>
        </section>
        {dialogs}
      </>
    );
  }

  const rows = visibleRows(view.rows, host, status);
  const activeEmpty = view.rows.length === 0 || status === "active";
  const createNode = machineMeant(book, [host, selected], (node) =>
    supportsSchedules(connections[node]),
  );
  const newSchedule =
    createNode === null ? null : (
      <Button
        icon="plus"
        variant="outline"
        onClick={() => {
          setSheet({ kind: "create", node: createNode });
        }}
      >
        New schedule
      </Button>
    );
  const choices = [
    {
      value: ALL_HOSTS,
      label: "All hosts",
      tone: "neutral" as const,
      status: "",
    },
    ...book.map((node) => {
      const connection = connections[node.key];
      const { tone, word } = machineState(connection);
      return {
        value: node.key,
        label: nodeLabel(node, connection),
        tone,
        status: word,
      };
    }),
  ];

  const names = new Map(choices.map(({ value, label }) => [value, label]));
  const machineName = (node: NodeKey) => names.get(node) ?? node;

  return (
    <>
      <section className="schedules-page" aria-label={heading}>
        {view.rows.length === 0 ? null : (
          <div className="schedules-toolbar">
            {book.length > 1 ? (
              <HostFilter
                hosts={choices}
                size="sm"
                value={host ?? ALL_HOSTS}
                onChange={(value) => {
                  setHost(book.find((node) => node.key === value)?.key ?? null);
                }}
              />
            ) : null}
            <Segmented
              label="Schedule status"
              value={status}
              options={FILTERS}
              onChange={setStatus}
            />
            <span className="schedules-toolbar-action">{newSchedule}</span>
          </div>
        )}
        <div
          className={
            rows.length === 0
              ? "schedules-list schedules-empty"
              : "schedules-list"
          }
        >
          {view.errors.length === 0 ? null : (
            <Alert
              variant="error"
              size="sm"
              description={view.errors.map(({ node, reason }) => (
                <div key={node}>
                  {machineName(node)}: {reason}
                </div>
              ))}
            />
          )}
          {rows.length === 0 ? (
            <EmptyState
              icon="calendar-clock"
              title={activeEmpty ? "No active schedules" : "No ended schedules"}
              action={activeEmpty ? newSchedule : undefined}
            >
              {activeEmpty ? "Schedules run agents on a cadence." : undefined}
            </EmptyState>
          ) : (
            <RowList label="Schedules">
              {rows.map((row) => (
                <ScheduleListRow
                  key={rowKey(row)}
                  row={row}
                  machine={book.length > 1 ? machineName(row.node) : null}
                  act={(action) => {
                    onAction(row, action);
                  }}
                  open={() => {
                    setSheet({ kind: "row", row });
                  }}
                />
              ))}
            </RowList>
          )}
        </div>
      </section>
      {dialogs}
    </>
  );
}
