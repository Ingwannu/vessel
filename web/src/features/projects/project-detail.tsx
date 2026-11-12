// One project in Settings › Projects, under the list (as a machine's
// detail is): its folder and default branch; the commands its
// `vessel.toml` names, and whether that set is approved, with Approve or
// Withdraw; its worktrees, each removable (by the node's rule, then the
// person's choice when work would be lost) and set up again; its
// snapshots, each droppable. What the node refuses is said in words above
// the lists until dismissed, never in a toast that goes by.

import { useCallback, useEffect, useState } from "react";

import { useFleet } from "../../context";
import { when } from "../../data/format";
import type { ListedCommands } from "../../data/workspace/command-changes";
import {
  approveCommands,
  projectCommands,
} from "../../data/workspace/projects";
import {
  keptReasonWords,
  setupWord,
} from "../../data/workspace/worktree-words";
import {
  listSnapshots,
  listWorktrees,
  removeWorktree,
  runSetup,
} from "../../data/workspace/worktrees";
import { Banner, Button, Row, RowList } from "../../ui/kit";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import type {
  KeptReason,
  ProjectCommands,
  ProjectInfo,
  Snapshot,
  WorktreeInfo,
} from "../../wire/proto";
import { FactValue, SettingsSection } from "../settings/settings-page";
import { Commands } from "./project-commands";
import { ApprovalDialog } from "./approval-dialog";
import { ProjectSnapshots } from "./project-snapshots";
import { RemoveWorktreeDialog } from "./remove-worktree-dialog";

/** What the project's lists hold, once read. */
interface Lists {
  commands: ProjectCommands | null;
  worktrees: readonly WorktreeInfo[];
  snapshots: readonly Snapshot[];
}

export function ProjectDetail({
  node,
  label,
  project,
}: {
  node: NodeKey;
  label: string;
  project: ProjectInfo;
}) {
  const fleet = useFleet();
  const [lists, setLists] = useState<Lists>({
    commands: null,
    worktrees: [],
    snapshots: [],
  });
  const [approving, setApproving] = useState<ListedCommands | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [removing, setRemoving] = useState<{
    worktree: WorktreeInfo;
    reasons: readonly KeptReason[];
  } | null>(null);
  const load = useCallback(() => {
    void Promise.all([
      projectCommands(fleet, node, project.id),
      listWorktrees(fleet, node, project.id),
      project.git ? listSnapshots(fleet, node, project.id) : null,
    ]).then(([commands, worktrees, snapshots]) => {
      setLists({
        commands: commands.ok ? commands.value : null,
        worktrees: worktrees.ok ? worktrees.value.worktrees : [],
        snapshots: snapshots?.ok === true ? snapshots.value.snapshots : [],
      });
    });
  }, [fleet, node, project.id, project.git]);
  useEffect(load, [load]);
  const approve = (listed: ListedCommands, approved: boolean) => {
    void approveCommands(fleet, node, project.id, listed.digest, approved).then(
      (outcome) => {
        setApproving(null);
        if (!outcome.ok) {
          setFailure(
            `The commands were not ${approved ? "approved" : "withdrawn"}: ${outcome.problem.message}`,
          );
        }
        load();
      },
    );
  };
  const remove = (worktree: WorktreeInfo) => {
    void removeWorktree(fleet, node, worktree.id, "keep").then((outcome) => {
      if (!outcome.ok) {
        setFailure(
          `${worktree.branch ?? "The worktree"} was not removed: ${outcome.problem.message}`,
        );
      } else if (outcome.value.outcome === "kept") {
        setRemoving({ worktree, reasons: outcome.value.reasons });
      } else {
        toast(`Removed ${worktree.branch ?? "the worktree"}`);
      }
      load();
    });
  };
  return (
    <>
      <SettingsSection title={project.name}>
        <RowList label="Facts">
          <Row
            title="Folder"
            control={<FactValue mono>{project.root}</FactValue>}
          />
          <Row
            title="Default branch"
            control={
              <FactValue mono>
                {project.git
                  ? (project.default_branch ?? "none yet")
                  : "not a git repository"}
              </FactValue>
            }
          />
          <Row
            title="Added"
            control={<FactValue>{when(project.created_at)}</FactValue>}
          />
        </RowList>
      </SettingsSection>
      {failure === null ? null : (
        <Banner
          tone="error"
          action={
            <Button
              size="xs"
              variant="ghost"
              onClick={() => {
                setFailure(null);
              }}
            >
              Dismiss
            </Button>
          }
        >
          {failure}
        </Banner>
      )}
      <SettingsSection title="Commands">
        <Commands
          commands={lists.commands}
          onApprove={setApproving}
          onWithdraw={(listed) => {
            approve(listed, false);
          }}
        />
      </SettingsSection>
      {project.git ? (
        <SettingsSection title="Worktrees">
          {lists.worktrees.length === 0 ? (
            <p className="settings-empty">
              None. A session started in a new worktree makes one.
            </p>
          ) : (
            <RowList label="Worktrees">
              {lists.worktrees.map((worktree) => (
                <Row
                  key={worktree.id}
                  mark={{ icon: "git-branch" }}
                  title={
                    worktree.branch ?? worktree.created_branch ?? "detached"
                  }
                  detail={{
                    facts: [
                      setupWord(worktree.setup),
                      worktree.sessions.length === 0
                        ? "no live session"
                        : `${String(worktree.sessions.length)} live`,
                      ...(worktree.kept === null || worktree.kept === undefined
                        ? []
                        : [`kept: ${keptReasonWords(worktree.kept)}`]),
                    ],
                  }}
                  control={
                    <>
                      {worktree.setup.state === "failed" ||
                      worktree.setup.state === "skipped" ? (
                        <Button
                          size="xs"
                          variant="ghost"
                          onClick={() => {
                            void runSetup(fleet, node, worktree.id).then(
                              (outcome) => {
                                if (!outcome.ok) {
                                  setFailure(
                                    `Setup did not start: ${outcome.problem.message}`,
                                  );
                                }
                                load();
                              },
                            );
                          }}
                        >
                          Run setup
                        </Button>
                      ) : null}
                      <Button
                        size="xs"
                        variant="ghost"
                        onClick={() => {
                          remove(worktree);
                        }}
                      >
                        Remove…
                      </Button>
                    </>
                  }
                />
              ))}
            </RowList>
          )}
        </SettingsSection>
      ) : null}
      <ProjectSnapshots
        node={node}
        project={project.id}
        snapshots={lists.snapshots}
        onDropped={load}
        onFailed={setFailure}
      />
      {approving === null ? null : (
        <ApprovalDialog
          commands={approving}
          compared={{ kind: "none" }}
          where={`in each new worktree of ${project.name} on ${label}`}
          busy={false}
          onApprove={() => {
            approve(approving, true);
          }}
          onClose={() => {
            setApproving(null);
          }}
        />
      )}
      {removing === null ? null : (
        <RemoveWorktreeDialog
          node={node}
          worktree={removing.worktree}
          reasons={removing.reasons}
          onClose={() => {
            setRemoving(null);
          }}
          onRemoved={() => {
            setRemoving(null);
            load();
          }}
        />
      )}
    </>
  );
}
