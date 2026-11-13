// A project's commands in Settings › Projects: what its `vessel.toml`
// names for a new worktree and as scripts and services, and whether that
// exact set is approved, with Review… (the approval dialog, which shows
// every one) or Withdraw.

import type { ListedCommands } from "../../data/workspace/command-changes";
import { Badge, Button, Row, RowList } from "../../ui/kit";
import type { ProjectCommands } from "../../wire/proto";

/** "2 setup, 1 teardown, 3 scripts and services". */
function commandsTitle(commands: ListedCommands): string {
  const scripts = commands.scripts?.length ?? 0;
  const lists = `${String(commands.setup.length)} setup, ${String(commands.teardown.length)} teardown`;
  return scripts === 0
    ? lists
    : `${lists}, ${String(scripts)} ${scripts === 1 ? "script or service" : "scripts and services"}`;
}

export function Commands({
  commands,
  onApprove,
  onWithdraw,
}: {
  commands: ProjectCommands | null;
  onApprove: (listed: ListedCommands) => void;
  onWithdraw: (listed: ListedCommands) => void;
}) {
  if (commands === null) {
    return <p className="settings-empty">Reading the project&apos;s file…</p>;
  }
  switch (commands.state) {
    case "none":
      return (
        <p className="settings-empty">
          Its folder has no <code>vessel.toml</code> with commands, so a new
          worktree has no setup and the project has no scripts.
        </p>
      );
    case "invalid":
      return (
        <p className="settings-empty">
          Its <code>vessel.toml</code> does not parse, so nothing in it runs:{" "}
          {commands.message}
        </p>
      );
    case "listed":
      return (
        <RowList label="Commands">
          <Row
            mark={{ icon: "terminal" }}
            title={commandsTitle(commands)}
            detail={{
              facts: [
                ...commands.setup,
                ...commands.teardown,
                ...(commands.scripts ?? []).map((script) => script.name),
              ].slice(0, 4),
            }}
            control={
              commands.approved ? (
                <>
                  <Badge tone="ok">Approved</Badge>
                  <Button
                    size="xs"
                    variant="ghost"
                    onClick={() => {
                      onWithdraw(commands);
                    }}
                  >
                    Withdraw
                  </Button>
                </>
              ) : (
                <>
                  <Badge tone="warn">Not approved</Badge>
                  <Button
                    size="xs"
                    onClick={() => {
                      onApprove(commands);
                    }}
                  >
                    Review…
                  </Button>
                </>
              )
            }
          />
        </RowList>
      );
  }
}
