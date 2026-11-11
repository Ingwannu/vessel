// The one permission prompt (GOAT's approval row, docked over the
// composer's top edge): a shield in the review colour, what is asked
// (the tool's title, in mono), then Decline and Approve with the agent's
// own names, and the agent's other answers under "More". The first
// answer from any follower wins (protocol); the timeline keeps one quiet
// line that becomes the outcome (data/approval.ts).

import "./permission-dock.css";

import { useFleet } from "../../context";
import { answersOf } from "../../data/approval";
import { answerPermission } from "../../data/calls";
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

export function PermissionDock({
  node,
  session,
  entry,
  location,
}: {
  node: string;
  session: string;
  entry: Extract<Entry, { kind: "permission" }>;
  /** The file the tool call touches, when it says. */
  location: string | null;
}) {
  const fleet = useFleet();
  const answers = answersOf(entry.request.options);
  const answer = (option: string) => {
    void answerPermission(fleet, node, session, entry.request.id, option).then(
      (outcome) => {
        if (!outcome.ok) {
          toast(outcome.message, "error");
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
        <code className="permission-detail">{entry.title}</code>
        {location === null ? null : (
          <span className="permission-where">{location}</span>
        )}
      </span>
      <span className="permission-actions">
        {answers.decline === null ? null : (
          <Button
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
