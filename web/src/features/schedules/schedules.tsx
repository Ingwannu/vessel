import "./schedules.css";

import { useEffect, useMemo, useState } from "react";

import { useCrumbs, useFleet, useFleetState } from "../../context";
import { machineState } from "../../data/machines";
import { nodeLabel } from "../../data/nodes";
import { aggregate, visibleRows } from "../../data/schedules/aggregate";
import { MachineSchedules } from "../../data/schedules/machine-schedules";
import {
  isHeartbeat,
  rowKey,
  type ScheduleAction,
  type ScheduleBucket,
  type ScheduleRow,
  scheduleTitle,
} from "../../data/schedules/rows";
import { schedulesLabel } from "../../data/schedules/words";
import { useStore } from "../../data/store";
import {
  Alert,
  Button,
  Dialog,
  EmptyState,
  HostFilter,
  RowList,
  Segmented,
  Spinner,
} from "../../ui/kit";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import { ScheduleListRow } from "./schedule-row";

const FILTERS = [
  { value: "active", label: "Active" },
  { value: "ended", label: "Ended" },
] as const;

const ALL_HOSTS = "";

export function Schedules() {
  const fleet = useFleet();
  const [schedules] = useState(() => new MachineSchedules(fleet.bridge));
  const reads = useStore(schedules.store, (state) => state);
  const connections = useFleetState((state) => state.statuses);
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

  if (view.phase === "loading") {
    return (
      <section className="schedules-centered" aria-label={heading}>
        <Spinner label="Loading schedules" size={16} />
      </section>
    );
  }
  if (view.phase === "failed") {
    return (
      <section className="schedules-centered" aria-label={heading}>
        <p>Unable to load schedules</p>
        <Button variant="ghost" onClick={retry}>
          Try again
        </Button>
      </section>
    );
  }

  const rows = visibleRows(view.rows, host, status);
  const activeEmpty = view.rows.length === 0 || status === "active";
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
              />
            ))}
          </RowList>
        )}
      </div>
      {deleting === null ? null : (
        <Dialog
          title={
            isHeartbeat(deleting.info) ? "Delete heartbeat" : "Delete schedule"
          }
          sheet="content"
          onClose={() => {
            setDeleting(null);
          }}
          footer={
            <>
              <Button
                onClick={() => {
                  setDeleting(null);
                }}
              >
                Cancel
              </Button>
              <Button
                variant="danger-solid"
                onClick={() => {
                  setDeleting(null);
                  run(deleting, "delete");
                }}
              >
                Delete
              </Button>
            </>
          }
        >
          <p>Delete “{scheduleTitle(deleting.info)}”? This cannot be undone.</p>
        </Dialog>
      )}
    </section>
  );
}
