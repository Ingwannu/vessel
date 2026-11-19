// A machine's "Log file" row and its "Node logs" dialog (ADR 0111, TRACKER
// 569), as Paseo's desktop daemon card (`desktop-updates-section.tsx`
// "Log file", "Copy path", "Open logs", and the `managed-daemon-logs-
// dialog`). Paseo shows only the desktop's own daemon; the page shows any
// machine's whose connection may administer it (`node.log` needs
// `admin`).

import { useEffect, useState } from "react";

import { useFleet, useFleetState } from "../../context";
import { nodeLog } from "../../data/calls";
import type { Outcome } from "../../data/outcome";
import type { NodeKey } from "../../wire/bridge";
import { DIAGNOSTICS_WIRE_PROTOCOL, type NodeLog } from "../../wire/proto";
import { Button, Dialog, Row, Spinner } from "../../ui/kit";
import { toast } from "../../ui/toasts";
import "./diagnostics.css";

/** Paseo's words, with the node's noun. */
export const LOG_WORDS = {
  title: "Log file",
  unavailable: "Log path unavailable",
  copyPath: "Copy path",
  open: "Open logs",
  dialog: "Node logs",
  empty: "(log file is empty)",
  copied: "Log path copied.",
  copyFailed: "Unable to copy log path.",
  loading: "Loading node logs...",
} as const;

/** Where a node without a log file writes, in words. */
export function writesTo(log: NodeLog): string {
  if (log.state === "file") return log.path;
  return log.writes_to === "terminal"
    ? "This node writes its log to the terminal it was started in."
    : "This node's log is not a file it keeps (a service manager's journal, for one).";
}

/** The row, on a machine speaking the protocol; nothing otherwise. */
export function NodeLogRow({ machine }: { machine: NodeKey }) {
  const fleet = useFleet();
  const status = useFleetState((state) => state.statuses[machine]);
  const speaks =
    status?.state === "connected" &&
    status.protocol >= DIAGNOSTICS_WIRE_PROTOCOL;
  const [found, setFound] = useState<Outcome<NodeLog> | null>(null);
  const [open, setOpen] = useState(false);
  useEffect(() => {
    if (!speaks) return;
    let current = true;
    void nodeLog(fleet, machine, 1).then((outcome) => {
      if (current) setFound(outcome);
    });
    return () => {
      current = false;
    };
  }, [fleet, machine, speaks]);
  if (!speaks) return null;
  const path =
    found?.ok === true && found.value.state === "file"
      ? found.value.path
      : null;
  const description =
    found?.ok === true && found.value.state === "no_file"
      ? writesTo(found.value)
      : (path ?? LOG_WORDS.unavailable);
  return (
    <>
      <Row
        title={LOG_WORDS.title}
        detail={{ description }}
        control={
          <>
            <Button
              size="sm"
              disabled={path === null}
              onClick={() => {
                if (path === null) return;
                navigator.clipboard.writeText(path).then(
                  () => {
                    toast(LOG_WORDS.copied, "success");
                  },
                  () => {
                    toast(LOG_WORDS.copyFailed, "error");
                  },
                );
              }}
            >
              {LOG_WORDS.copyPath}
            </Button>
            <Button
              size="sm"
              disabled={path === null}
              onClick={() => {
                setOpen(true);
              }}
            >
              {LOG_WORDS.open}
            </Button>
          </>
        }
      />
      {open ? (
        <NodeLogDialog
          machine={machine}
          onClose={() => {
            setOpen(false);
          }}
        />
      ) : null}
    </>
  );
}

function NodeLogDialog({
  machine,
  onClose,
}: {
  machine: NodeKey;
  onClose: () => void;
}) {
  const fleet = useFleet();
  const [log, setLog] = useState<Outcome<NodeLog> | null>(null);
  useEffect(() => {
    let current = true;
    void nodeLog(fleet, machine, null).then((outcome) => {
      if (current) setLog(outcome);
    });
    return () => {
      current = false;
    };
  }, [fleet, machine]);
  return (
    <Dialog title={LOG_WORDS.dialog} onClose={onClose} size="lg">
      {log === null ? (
        <p className="diagnostic-wait">
          <Spinner />
          {LOG_WORDS.loading}
        </p>
      ) : !log.ok ? (
        <p className="diagnostic-wait">{log.problem.message}</p>
      ) : (
        <>
          <p className="diagnostic-wait">{writesTo(log.value)}</p>
          {log.value.state === "file" ? (
            <pre className="diagnostic-text" data-testid="node-log-lines">
              {log.value.lines.length === 0
                ? LOG_WORDS.empty
                : log.value.lines.join("\n")}
            </pre>
          ) : null}
        </>
      )}
    </Dialog>
  );
}
