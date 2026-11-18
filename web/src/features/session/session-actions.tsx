// A session's actions, in a menu at the end of its title bar ("If a
// control is not needed this minute, it is in a menu", DESIGN.md §1):
// its marks where the machine keeps them (protocol 10: pin, read or
// unread, start again, archive or unarchive, delete; use-mark-actions.tsx),
// move to another profile, copy its id, kill. Moving, archiving, deleting
// and killing ask first, in a dialog.

import { useState } from "react";

import { useFleet, useFleetState } from "../../context";
import { type MarkAction, markActions } from "../../data/marks";
import { killSession } from "../../data/calls";
import { isLive, shortId } from "../../data/format";
import {
  Button,
  Dialog,
  IconButton,
  type IconName,
  Menu,
  MenuItem,
  MenuSeparator,
  Tooltip,
} from "../../ui/kit";
import { toast, toastCopied } from "../../ui/toasts";
import type { SessionInfo } from "../../wire/proto";
import { MoveProfile } from "./move-profile";
import { useMarkActions } from "./use-mark-actions";
import type { NodeKey } from "../../wire/bridge";

export function SessionActions({
  node,
  label,
  session,
}: {
  node: NodeKey;
  /** The machine's name. */
  label: string;
  session: SessionInfo;
}) {
  const fleet = useFleet();
  const [dialog, setDialog] = useState<"kill" | "move" | null>(null);
  const live = isLive(session);
  const protocol = useFleetState((state) => {
    const status = state.statuses[node];
    return status?.state === "connected" ? status.protocol : null;
  });
  const marks = useMarkActions(node, session, label);
  const offered = markActions(session, protocol);
  const item = (action: MarkAction) =>
    offered.includes(action) ? (
      <MenuItem
        key={action}
        icon={markIcon(action)}
        tone={action === "delete" ? "danger" : "default"}
        onSelect={() => {
          marks.run(action);
        }}
      >
        {markLabel(action)}
      </MenuItem>
    ) : null;
  const close = () => {
    setDialog(null);
  };
  const kill = () => {
    setDialog(null);
    void killSession(fleet, node, session.id).then((outcome) => {
      if (outcome.ok) {
        toast(`Killed ${shortId(session.id)}`);
        void fleet.refreshSessions(node);
      } else {
        toast(outcome.problem.message, "error");
      }
    });
  };
  return (
    <>
      <Menu
        align="end"
        trigger={
          <Tooltip label="Session actions">
            <IconButton label="Session actions" icon="more" />
          </Tooltip>
        }
      >
        {item("pin")}
        {item("unpin")}
        {item("mark-unread")}
        {item("mark-read")}
        {item("start-again")}
        {item("archive")}
        {item("unarchive")}
        {session.spec.mode === "structured" && live ? (
          <MenuItem
            icon="move"
            onSelect={() => {
              setDialog("move");
            }}
          >
            Move to another profile
          </MenuItem>
        ) : null}
        <MenuItem
          icon="copy"
          onSelect={() => {
            navigator.clipboard.writeText(session.id).then(
              () => {
                toastCopied("Copied the session id");
              },
              () => {
                toast("The clipboard refused the text", "error");
              },
            );
          }}
        >
          Copy session id
        </MenuItem>
        {offered.includes("delete") ? (
          <>
            <MenuSeparator />
            {item("delete")}
          </>
        ) : null}
        {live ? (
          <>
            <MenuSeparator />
            <MenuItem
              icon="kill"
              tone="danger"
              onSelect={() => {
                setDialog("kill");
              }}
            >
              Kill
            </MenuItem>
          </>
        ) : null}
      </Menu>
      {dialog === "kill" ? (
        <Dialog
          title={`Kill ${shortId(session.id)}?`}
          onClose={close}
          footer={
            <>
              <Button variant="ghost" onClick={close}>
                Cancel
              </Button>
              <Button variant="danger-solid" onClick={kill}>
                Kill
              </Button>
            </>
          }
        >
          <p className="hint">
            The agent on {label} and everything it started stop now. The
            session&apos;s log stays.
          </p>
        </Dialog>
      ) : null}
      {marks.dialog}
      {dialog === "move" ? (
        <MoveProfile
          node={node}
          session={session}
          close={close}
          moved={() => {
            setDialog(null);
            void fleet.refreshSessions(node);
          }}
        />
      ) : null}
    </>
  );
}

function markLabel(action: MarkAction): string {
  switch (action) {
    case "pin":
      return "Pin session";
    case "unpin":
      return "Unpin session";
    case "mark-unread":
      return "Mark as unread";
    case "mark-read":
      return "Mark as read";
    case "start-again":
      return "Start again";
    case "archive":
      return "Archive session";
    case "unarchive":
      return "Unarchive";
    case "delete":
      return "Delete permanently";
  }
}

function markIcon(action: MarkAction): IconName {
  switch (action) {
    case "pin":
    case "unpin":
      return "pin";
    case "mark-unread":
    case "mark-read":
      return "dot";
    case "start-again":
      return "restore";
    case "archive":
    case "unarchive":
      return "archive";
    case "delete":
      return "trash";
  }
}
