// Settings › Projects (Paseo's projects in its settings): a machine's
// projects as rows, Add project at the heading's right (Paseo's sheet,
// on this machine: add-project-flow.tsx, TRACKER 234), the machine's git
// when it cannot be used, and the chosen project's detail under the list.

import "./projects.css";

import { useState } from "react";

import { useFleet, useFleetState } from "../../context";
import { machineFor } from "../../data/machines";
import { nodeLabel } from "../../data/nodes";
import { openAddProject } from "../../data/workspace/add-project/request";
import { loadProjects, useProjects } from "../../data/workspace/project-store";
import { removeProject } from "../../data/workspace/projects";
import {
  cloneFailureWords,
  gitSupportWords,
  projectStateWord,
} from "../../data/workspace/worktree-words";
import {
  Badge,
  Banner,
  Button,
  EmptyState,
  IconButton,
  Menu,
  MenuItem,
  Row,
  RowList,
  Skeleton,
} from "../../ui/kit";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import type { ProjectId, ProjectInfo } from "../../wire/proto";
import { MachinePicker } from "../machines/machine-picker";
import { SettingsPage, SettingsSection } from "../settings/settings-page";
import { ProjectDetail } from "./project-detail";
import { ProjectIcon } from "./project-icon";

const LEAD =
  "The folders a machine works in. A session runs in a project's folder as it is, or in a worktree of its own on a new branch.";

function facts(project: ProjectInfo): string[] {
  const state = projectStateWord(project.state);
  if (state !== null) {
    return [project.root, state];
  }
  return [
    project.root,
    project.git ? (project.default_branch ?? "no commit yet") : "no git",
  ];
}

export function ProjectsSection({ machine }: { machine: NodeKey | undefined }) {
  const fleet = useFleet();
  const book = useFleetState((state) => state.book);
  const statuses = useFleetState((state) => state.statuses);
  const selected = useFleetState((state) => state.selected);
  const node = machineFor(book ?? [], machine, selected);
  const projects = useProjects(fleet, node?.key ?? null);
  const [open, setOpen] = useState<ProjectId | null>(null);
  // A removal the node refused, in words under the list until dismissed.
  const [failure, setFailure] = useState<string | null>(null);
  if (node === null) {
    return (
      <SettingsPage lead={LEAD}>
        {book === null ? (
          <Skeleton />
        ) : (
          <p className="settings-empty">No machines yet.</p>
        )}
      </SettingsPage>
    );
  }
  const label = nodeLabel(node, statuses[node.key]);
  const list = projects.kind === "ready" ? projects.list.projects : [];
  const chosen = list.find((project) => project.id === open) ?? null;
  const add = (
    <Button
      variant="ghost"
      size="xs"
      icon="plus"
      onClick={() => {
        openAddProject(node.key);
      }}
    >
      Add project
    </Button>
  );
  const git =
    projects.kind === "ready" ? gitSupportWords(projects.list.git) : null;
  return (
    <SettingsPage lead={LEAD}>
      {git === null ? null : <Banner>{git}</Banner>}
      <SettingsSection
        title={`Projects on ${label}`}
        action={
          <>
            <MachinePicker section="projects" machine={node.key} />
            {projects.kind === "ready" ? add : null}
          </>
        }
      >
        {projects.kind === "loading" ? (
          <Skeleton />
        ) : projects.kind === "older" ? (
          <p className="settings-empty">
            {label} runs a Vessel older than projects.
          </p>
        ) : projects.kind === "failed" ? (
          <p className="settings-empty">{projects.problem.message}</p>
        ) : list.length === 0 ? (
          <EmptyState title="No projects yet" icon="folder" action={add}>
            Add a folder of {label}, clone a repository there, or make a new
            one.
          </EmptyState>
        ) : (
          <RowList label="Projects">
            {list.map((project) => (
              <Row
                key={project.id}
                selected={project.id === open}
                mark={{
                  node: (
                    <span className="row-icon">
                      <ProjectIcon
                        node={node.key}
                        project={project.id}
                        name={project.name}
                      />
                    </span>
                  ),
                }}
                title={
                  <button
                    type="button"
                    className="row-link"
                    aria-current={project.id === open ? "true" : undefined}
                    onClick={() => {
                      setOpen(project.id === open ? null : project.id);
                    }}
                  >
                    {project.name}
                  </button>
                }
                detail={{ facts: facts(project) }}
                control={
                  project.state.state === "clone_failed" ? (
                    <Badge tone="danger">Clone failed</Badge>
                  ) : project.state.state === "missing" ? (
                    <Badge tone="warn">Folder missing</Badge>
                  ) : null
                }
                menu={
                  <Menu
                    align="end"
                    trigger={
                      <IconButton
                        label={`${project.name} actions`}
                        icon="more"
                        size="xs"
                        variant="ghost"
                      />
                    }
                  >
                    <MenuItem
                      icon="trash"
                      tone="danger"
                      onSelect={() => {
                        void removeProject(fleet, node.key, project.id).then(
                          (outcome) => {
                            if (outcome.ok) {
                              toast(`Forgot ${project.name}; its files stay`);
                              if (open === project.id) {
                                setOpen(null);
                              }
                            } else {
                              setFailure(
                                `${project.name} was not removed: ${outcome.problem.message}`,
                              );
                            }
                            void loadProjects(fleet, node.key);
                          },
                        );
                      }}
                    >
                      Remove project
                    </MenuItem>
                  </Menu>
                }
              />
            ))}
          </RowList>
        )}
        {list
          .filter((project) => project.state.state === "clone_failed")
          .map((project) =>
            project.state.state === "clone_failed" ? (
              <Banner key={project.id} tone="error">
                {project.name}: {cloneFailureWords(project.state.reason)}{" "}
                {project.state.message}
              </Banner>
            ) : null,
          )}
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
      </SettingsSection>
      {chosen === null ? null : (
        <ProjectDetail
          key={chosen.id}
          node={node.key}
          label={label}
          project={chosen}
        />
      )}
    </SettingsPage>
  );
}
