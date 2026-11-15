// The one permission prompt (GOAT's approval row, docked over the
// composer's top edge): a shield in the review colour, what is asked
// (the tool's title, in mono), then Decline and Approve with the agent's
// own names, and the agent's other answers under "More". The first
// answer from any follower wins (protocol); the timeline keeps one quiet
// line that becomes the outcome (data/approval.ts).

import "./permission-dock.css";
import { useState } from "react";

import { Markdown } from "../../data/markdown";
import { useFleet } from "../../context";
import { answersOf } from "../../data/approval";
import {
  permissionReceipt,
  respondApproval,
} from "../../data/response-receipts";
import type { Entry } from "../../data/timeline-model";
import {
  Button,
  Icon,
  IconButton,
  Menu,
  MenuItem,
  Tooltip,
} from "../../ui/kit";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import type { PermissionOptionId, SessionId } from "../../wire/proto";

export function PermissionDock({
  node,
  session,
  entry,
  location,
}: {
  node: NodeKey;
  session: SessionId;
  entry: Extract<Entry, { kind: "permission" }>;
  /** The file the tool call touches, when it says. */
  location: string | null;
}) {
  const fleet = useFleet();
  const answers = answersOf(entry.request.options);
  const [sending, setSending] = useState(false);
  const blocked =
    sending ||
    (entry.request.delivery_state !== undefined &&
      entry.request.delivery_state !== "pending");
  const answer = (option: PermissionOptionId) => {
    if (blocked) return;
    setSending(true);
    void respondApproval(fleet, node, session, entry.request, option).then(
      (outcome) => {
        if (!outcome.ok) {
          const notSent =
            outcome.problem.from === "call" &&
            outcome.problem.failure.kind === "not_in_protocol";
          if (notSent) setSending(false);
          toast(outcome.problem.message, "error");
          return;
        }
        const state = permissionReceipt(entry.request, option, outcome.value);
        if (state === "refused") setSending(false);
        if (state !== "applied" && state !== "legacy_sent") {
          toast(
            state === "uncertain"
              ? "The response outcome is uncertain. Reconcile before trying again."
              : "The request was refused or changed. Reconcile its current state.",
            "error",
          );
        }
      },
    );
  };

  return (
    <div
      className="permission-dock"
      role="group"
      aria-label="Permission request"
      aria-live="assertive"
    >
      <span className="permission-icon">
        <Icon name="shield" size={16} />
      </span>
      <span className="permission-what">
        <span className="permission-kind">Approval needed</span>
        <code className="permission-detail">
          {entry.request.presentation?.title ?? entry.title}
        </code>
        {entry.request.presentation?.description === undefined ||
        entry.request.presentation.description === null ? null : (
          <span>{entry.request.presentation.description}</span>
        )}
        {entry.request.presentation?.plan_text === undefined ||
        entry.request.presentation.plan_text === null ? null : (
          <div className="md">
            <Markdown>{entry.request.presentation.plan_text}</Markdown>
          </div>
        )}
        {location === null ? null : (
          <span className="permission-where">{location}</span>
        )}
      </span>
      <span className="permission-actions">
        {answers.decline === null ? null : (
          <Button
            disabled={blocked}
            size="xs"
            onClick={() => {
              if (answers.decline !== null) {
                answer(answers.decline.id);
              }
            }}
          >
            {answers.decline.name}
          </Button>
        )}
        {answers.approve === null ? null : (
          <Button
            disabled={blocked}
            size="xs"
            variant="primary"
            onClick={() => {
              if (answers.approve !== null) {
                answer(answers.approve.id);
              }
            }}
          >
            {answers.approve.name}
          </Button>
        )}
        {answers.more.length === 0 ? null : (
          <Menu
            side="top"
            align="end"
            trigger={
              <Tooltip label="More answers">
                <IconButton
                  disabled={blocked}
                  label="More answers"
                  icon="more"
                  size="xs"
                  variant="secondary"
                />
              </Tooltip>
            }
          >
            {answers.more.map((option) => (
              <MenuItem
                disabled={blocked}
                key={option.id}
                onSelect={() => {
                  answer(option.id);
                }}
              >
                {option.name}
              </MenuItem>
            ))}
          </Menu>
        )}
      </span>
    </div>
  );
}
