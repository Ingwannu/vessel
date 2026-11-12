// Where the new-session page's session runs, and starting it there: the
// tray's place (`data/workspace/place.ts`), whether Send can go yet, the
// session's creation (in a typed folder, a project's folder, or after a
// worktree is made and set up, `start-worktree.ts`), and what the page
// shows meanwhile (the approval, setup's output, a refusal).

import { type ReactNode, useCallback, useState } from "react";

import { useFleet, useFleetState } from "../../context";
import { createSession } from "../../data/calls";
import { labelOf } from "../../data/nodes";
import type { Outcome } from "../../data/outcome";
import { useStore } from "../../data/store";
import {
  NO_PLACE,
  type Place,
  placeProblem,
  placeSpec,
  projectPlace,
} from "../../data/workspace/place";
import { projectIn, useProjects } from "../../data/workspace/project-store";
import { wantedPlace } from "../../data/workspace/wanted-place";
import { Spinner } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { SessionInfo, SessionSpec } from "../../wire/proto";
import { ApprovalDialog } from "../projects/approval-dialog";
import { SetupCard } from "../workspace/setup-card";
import { makeWorktree, type Step } from "./start-worktree";

/** The person's answers to an approval and to a failed setup. */
type Approval = "approve" | "skip" | "cancel";
type Failure = "retry" | "start" | "stop";

/** The answers being waited for. */
interface Asking {
  approval: ((choice: Approval) => void) | null;
  failure: ((choice: Failure) => void) | null;
}

const NOT_ASKING: Asking = { approval: null, failure: null };

/** Whether Send can go, as `data/new-session.ts`'s `sendable` says it. */
type Sendable = { ok: true } | { ok: false; reason: string | null };

export interface Starter {
  place: Place;
  setPlace: (place: Place) => void;
  /** `ready` with what the place adds: a project still cloning, a branch
   * name git refuses. */
  ready: (ready: Sendable) => Sendable;
  /** Create the session `spec` describes in the place; `null` when the
   * person stopped before it started (or the page said why). */
  create: (spec: SessionSpec) => Promise<Outcome<SessionInfo> | null>;
  /** What the page shows while it starts. */
  view: ReactNode;
}

export function useStarter(machine: NodeKey | null): Starter {
  const fleet = useFleet();
  const label = useFleetState((state) =>
    machine === null ? "" : labelOf(state, machine),
  );
  const projects = useProjects(fleet, machine);
  // A place is a machine's: another machine starts from no project.
  const [held, setHeld] = useState<{ machine: NodeKey | null; place: Place }>({
    machine,
    place: NO_PLACE,
  });
  // A project asked for from the sidebar, until the person chooses.
  const wanted = useStore(wantedPlace, (value) => value);
  const place =
    held.machine === machine
      ? held.place
      : wanted !== null && wanted.node === machine
        ? projectPlace(wanted.project)
        : NO_PLACE;
  const [step, setStep] = useState<Step>({ kind: "idle" });
  // Who waits on the person's answer to the question on screen: the
  // dialog's and the card's buttons answer it.
  const [asking, setAsking] = useState<Asking>(NOT_ASKING);

  const project =
    place.kind === "project" ? projectIn(projects, place.project) : null;
  const ready = (base: Sendable): Sendable => {
    if (!base.ok) {
      return base;
    }
    const problem = placeProblem(place, project);
    return problem === null ? base : { ok: false, reason: problem };
  };

  const create = useCallback(
    async (spec: SessionSpec): Promise<Outcome<SessionInfo> | null> => {
      if (machine === null) {
        return null;
      }
      setStep({ kind: "idle" });
      wantedPlace.set(null);
      const part = placeSpec(place);
      if (part !== null) {
        return createSession(fleet, machine, {
          ...spec,
          ...(place.kind === "project" ? { cwd: null } : {}),
          ...part,
        });
      }
      if (place.kind !== "project") {
        return null;
      }
      const made = await makeWorktree(fleet, machine, place, {
        show: setStep,
        approve: () =>
          new Promise<Approval>((resolve) => {
            setAsking({ approval: resolve, failure: null });
          }),
        failed: () =>
          new Promise<Failure>((resolve) => {
            setAsking({ approval: null, failure: resolve });
          }),
      });
      switch (made.kind) {
        case "stopped":
          return null;
        case "failed":
          setStep({ kind: "idle" });
          return { ok: false, problem: made.problem };
        case "ready":
          return createSession(fleet, machine, {
            ...spec,
            cwd: null,
            place: { kind: "worktree", worktree: made.worktree },
          });
      }
    },
    [fleet, machine, place],
  );

  const approve = (choice: Approval) => () => {
    asking.approval?.(choice);
    setAsking(NOT_ASKING);
  };
  const decide = (choice: Failure) => () => {
    asking.failure?.(choice);
    setAsking(NOT_ASKING);
  };
  const view: ReactNode =
    step.kind === "making" ? (
      <p className="new-session-note" role="status">
        <Spinner size={12} /> Making the worktree…
      </p>
    ) : step.kind === "refused" ? (
      <p className="new-session-error" role="alert">
        {step.message}
      </p>
    ) : step.kind === "approval" ? (
      <ApprovalDialog
        commands={step.commands}
        compared={step.compared}
        where={`in a new worktree of ${project?.name ?? "the project"} on ${label}`}
        busy={step.busy}
        onApprove={approve("approve")}
        onSkip={approve("skip")}
        onClose={approve("cancel")}
      />
    ) : step.kind === "setup" ? (
      <SetupCard
        branch={step.branch}
        setup={step.setup}
        log={step.log}
        commands={step.commands}
        {...(step.setup.state === "failed"
          ? {
              onRetry: decide("retry"),
              onStart: decide("start"),
              onStop: decide("stop"),
            }
          : {})}
      />
    ) : null;

  return {
    place,
    setPlace: (next) => {
      setStep({ kind: "idle" });
      setHeld({ machine, place: next });
    },
    ready,
    create,
    view,
  };
}
