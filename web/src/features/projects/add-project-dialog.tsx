// Adding a project (agent-orchestrator's "New project" and its clone
// dialog): a folder on the machine, a repository cloned into a folder, or
// a new repository started in an empty folder. A clone answers at once
// and goes on on the machine; its progress is the project's row's
// (data/workspace/project-store.ts follows it), so the dialog closes.

import { useState } from "react";

import { useFleet } from "../../context";
import { nextStep } from "../../data/outcome";
import { tookProject } from "../../data/workspace/project-store";
import {
  addProject,
  cloneProject,
  initProject,
} from "../../data/workspace/projects";
import { Button, Dialog, Field, TextField } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";

/** Which way a project is added. */
export type AddWay = "folder" | "clone" | "init";

function title(way: AddWay): string {
  switch (way) {
    case "folder":
      return "Add a folder";
    case "clone":
      return "Clone a Git repository";
    case "init":
      return "Start a new project";
  }
}

/** The repository name a clone URL ends with, for the help line. */
function repositoryName(url: string): string | null {
  const last = url.trim().replace(/\/+$/u, "").split(/[/:]/u).at(-1) ?? "";
  const name = last.replace(/\.git$/u, "");
  return name === "" ? null : name;
}

export function AddProjectDialog({
  node,
  label,
  way,
  onClose,
}: {
  node: NodeKey;
  /** The machine's name. */
  label: string;
  way: AddWay;
  onClose: () => void;
}) {
  const fleet = useFleet();
  const [path, setPath] = useState("");
  const [url, setUrl] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const name = repositoryName(url);
  const ready =
    !busy &&
    path.trim() !== "" &&
    (way !== "clone" || (url.trim() !== "" && name !== null));
  const submit = () => {
    if (!ready) {
      return;
    }
    setBusy(true);
    setError(null);
    const call =
      way === "folder"
        ? addProject(fleet, node, path.trim(), null)
        : way === "init"
          ? initProject(fleet, node, path.trim(), null)
          : cloneProject(fleet, node, {
              url: url.trim(),
              parent: path.trim(),
              folder: null,
            });
    void call.then((outcome) => {
      setBusy(false);
      if (outcome.ok) {
        tookProject(fleet, node, outcome.value);
        onClose();
      } else {
        setError(`${outcome.problem.message} ${nextStep(outcome.problem)}`);
      }
    });
  };
  const folder = path.trim().replace(/[\\/]+$/u, "");
  return (
    <Dialog
      title={title(way)}
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button
            variant="primary"
            disabled={!ready}
            busy={busy}
            onClick={submit}
          >
            {way === "clone" ? "Clone" : way === "init" ? "Create" : "Add"}
          </Button>
        </>
      }
    >
      <form
        onSubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        {way === "clone" ? (
          <Field label="Repository URL" error={error}>
            <TextField
              value={url}
              onChange={setUrl}
              mono
              type="url"
              icon="link"
              placeholder="https://github.com/org/repository.git"
              autoFocus
            />
          </Field>
        ) : null}
        <Field
          label={way === "clone" ? "Clone into" : "Folder"}
          {...(way === "clone" ? {} : { error })}
          hint={
            way === "clone"
              ? name === null || folder === ""
                ? `A folder on ${label}; the repository gets a folder inside it.`
                : `Repository will be created at ${folder}/${name}.`
              : way === "init"
                ? `An empty or new folder on ${label}; git starts a repository there.`
                : `An absolute folder on ${label}; inside a repository, its top level is the project.`
          }
        >
          <TextField
            value={path}
            onChange={setPath}
            mono
            icon="folder"
            placeholder={
              way === "clone" ? "Choose a parent folder" : "/path/to/folder"
            }
            {...(way === "clone" ? {} : { autoFocus: true })}
          />
        </Field>
      </form>
    </Dialog>
  );
}
