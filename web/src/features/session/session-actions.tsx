// A session's actions, in a menu at the end of its title bar ("If a
// control is not needed this minute, it is in a menu", DESIGN.md §1):
// move to another profile, copy its id, kill. Moving and killing ask
// first, in a dialog.

import { useState } from "react";

import { useFleet } from "../../context";
import { killSession } from "../../data/calls";
import { isLive, shortId } from "../../data/format";
import {
  Button,
  Dialog,
  IconButton,
  Menu,
  MenuItem,
  MenuSeparator,
  Tooltip,
} from "../../ui/kit";
import { toast } from "../../ui/toasts";
import type { SessionInfo } from "../../wire/proto";
import { MoveProfile } from "./move-profile";

export function SessionActions({
  node,
  label,
  session,
}: {
  node: string;
  /** The machine's name. */
  label: string;
  session: SessionInfo;
}) {
  const fleet = useFleet();
  const [dialog, setDialog] = useState<"kill" | "move" | null>(null);
  const live = isLive(session);
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
                toast("Copied the session id");
              },
              () => {
                toast("The clipboard refused the text", "error");
              },
            );
          }}
        >
          Copy session id
        </MenuItem>
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
