// "Commit changes" (GOAT's commit dialog): the branch it lands on, the
// files it takes, what to stage first, and the message. A refusal (nothing
// to commit, no identity, a hook) is said in the dialog, which stays open.

import { useState } from "react";

import { counted } from "../../data/workspace/worktree-words";
import {
  Button,
  Dialog,
  Field,
  FieldGroup,
  Segmented,
  TextArea,
} from "../../ui/kit";
import type { CommitStage, GitStatus } from "../../wire/proto";
import { GitNotice } from "./git-notice";
import type { Git } from "./use-git";

/** How many of the status's files are staged. */
function stagedCount(status: GitStatus): number {
  return status.files.filter(
    (file) => file.index !== "unmodified" && file.index !== "untracked",
  ).length;
}

export function CommitDialog({
  status,
  git,
  isDefault,
  onClose,
}: {
  status: GitStatus;
  git: Git;
  /** Whether the branch is the project's default one (GOAT warns). */
  isDefault: boolean;
  onClose: () => void;
}) {
  const staged = stagedCount(status);
  const [message, setMessage] = useState("");
  const [stage, setStage] = useState<CommitStage>(
    staged > 0 ? "staged" : "everything",
  );
  const ready = message.trim() !== "" && git.busy === null;
  const commit = () => {
    if (!ready) {
      return;
    }
    void git.commit(message.trim(), stage).then((done) => {
      if (done) {
        onClose();
      }
    });
  };
  return (
    <Dialog
      title="Commit changes"
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button
            variant="primary"
            icon="git-commit"
            disabled={!ready}
            busy={git.busy === "commit"}
            onClick={commit}
          >
            Commit
          </Button>
        </>
      }
    >
      <div className="commit-summary">
        <div className="commit-summary-row">
          <span className="commit-summary-label">Branch</span>
          <span className="commit-summary-value">
            {status.branch ?? "Detached HEAD"}
            {isDefault ? (
              <span className="commit-default"> · Default branch</span>
            ) : null}
          </span>
        </div>
        <div className="commit-summary-row">
          <span className="commit-summary-label">Files</span>
          <span className="commit-summary-value">
            {counted(status.files.length, "file", "files")} changed ·{" "}
            {counted(staged, "file", "files")} staged
          </span>
        </div>
        <ul className="commit-files">
          {status.files.slice(0, 200).map((file) => (
            <li key={file.path}>{file.path}</li>
          ))}
          {status.more || status.files.length > 200 ? <li>…</li> : null}
        </ul>
      </div>
      <FieldGroup label="Stage first">
        <Segmented
          label="Stage first"
          size="sm"
          value={stage}
          onChange={setStage}
          options={[
            { value: "staged", label: "Staged only", disabled: staged === 0 },
            { value: "tracked", label: "Tracked files" },
            { value: "everything", label: "Everything" },
          ]}
        />
      </FieldGroup>
      <Field label="Commit message">
        <TextArea
          value={message}
          onChange={setMessage}
          placeholder="What this commit does"
          minRows={3}
          maxRows={8}
          onKeyDown={(event) => {
            if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
              event.preventDefault();
              commit();
            }
          }}
        />
      </Field>
      {git.notice === null ? null : <GitNotice notice={git.notice} />}
    </Dialog>
  );
}
