// New session (DESIGN.md §7.5): each field filters the next. The dialog
// stays open until the node answers; a failure shows inline.
//
// The Model field lists the harness's own provider and the node's model
// routes (`model.list`). A route reaches a session only through a profile
// naming it (data/models.ts), so the two fields move together: picking a
// route picks a profile of this harness on it, and picking a routed
// profile shows its route.

import { useNavigate } from "@tanstack/react-router";
import { type SyntheticEvent, useState } from "react";

import { closeNewSession, useFleet, useFleetState } from "../context";
import {
  harnessOptions,
  nodeOptions,
  profileOptions,
  routeOptions,
} from "../data/choices";
import { profileOn } from "../data/models";
import {
  Button,
  Dialog,
  Field,
  FieldGroup,
  Segmented,
  Select,
  TextField,
} from "../ui/kit";
import type { HarnessKind, SessionSpec } from "../wire/proto";

type Mode = "structured" | "terminal";

export function NewSessionDialog({
  node: preselected,
}: {
  node: string | null;
}) {
  const fleet = useFleet();
  const navigate = useNavigate();
  const book = useFleetState((state) => state.book);
  const statuses = useFleetState((state) => state.statuses);
  const allHarnesses = useFleetState((state) => state.harnesses);
  const allProfiles = useFleetState((state) => state.profiles);
  const allModels = useFleetState((state) => state.models);
  const nodes = book ?? [];
  const connected = nodes.filter(
    (node) => statuses[node.key]?.state === "connected",
  );
  const [node, setNode] = useState<string>(
    preselected !== null &&
      connected.some((candidate) => candidate.key === preselected)
      ? preselected
      : ((connected.find((candidate) => candidate.default) ?? connected[0])
          ?.key ?? ""),
  );
  const harnesses = allHarnesses[node] ?? [];
  const [harness, setHarness] = useState<HarnessKind | "">(
    harnesses[0]?.kind ?? "",
  );
  const chosenHarness =
    harnesses.find((candidate) => candidate.kind === harness) ?? harnesses[0];
  const structuredAvailable = chosenHarness?.structured.state === "available";
  const profiles = (allProfiles[node] ?? []).filter(
    (profile) => profile.harness === chosenHarness?.kind,
  );
  const routes = allModels[node];
  const [profile, setProfile] = useState("");
  const chosenProfile = profiles.find(
    (candidate) => candidate.name === profile,
  );
  const route = chosenProfile?.model ?? "";
  const chosenRoute = routes?.find((candidate) => candidate.name === route);
  const [cwd, setCwd] = useState("");
  const [mode, setMode] = useState<Mode>("structured");
  const effectiveMode: Mode = structuredAvailable ? mode : "terminal";
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const create = (event?: SyntheticEvent) => {
    event?.preventDefault();
    if (chosenHarness === undefined || busy) {
      return;
    }
    const spec: SessionSpec =
      effectiveMode === "structured"
        ? {
            harness: chosenHarness.kind,
            mode: "structured",
            // A routed harness is pinned to its route's models; an
            // unrouted one uses its own default.
            model: null,
          }
        : {
            harness: chosenHarness.kind,
            mode: "terminal",
            args: [],
            size: { cols: 120, rows: 36 },
          };
    if (cwd.trim() !== "") {
      spec.cwd = cwd.trim();
    }
    if (profile !== "") {
      spec.profile = profile;
    }
    setBusy(true);
    setError(null);
    fleet.bridge.call(node, "session.create", { spec }).then(
      (info) => {
        closeNewSession();
        void fleet.refreshSessions(node);
        void navigate({
          to: "/sessions/$node/$id",
          params: { node, id: info.id },
        });
      },
      (failure: unknown) => {
        setBusy(false);
        setError(failure instanceof Error ? failure.message : String(failure));
      },
    );
  };

  const routeHint =
    chosenRoute !== undefined
      ? `Through profile ${profile}, pinned to ${chosenRoute.model}${
          chosenRoute.small_model === undefined ||
          chosenRoute.small_model === null
            ? ""
            : ` and ${chosenRoute.small_model}`
        }.`
      : route !== ""
        ? `Profile ${profile} names route ${route}, which the node does not offer.`
        : routes === null
          ? "This node predates model routing."
          : routes !== undefined && routes.length > 0
            ? "A route runs the harness on another provider's model, through a profile naming it."
            : "The node's models.toml defines model routes; it has none.";
  return (
    <Dialog
      title="New session"
      onClose={closeNewSession}
      footer={
        <>
          <Button variant="ghost" onClick={closeNewSession}>
            Cancel
          </Button>
          <Button
            variant="primary"
            type="submit"
            form="new-session"
            busy={busy}
            disabled={busy || chosenHarness === undefined}
          >
            {busy ? "Creating…" : "Create"}
          </Button>
        </>
      }
    >
      <form id="new-session" onSubmit={create}>
        <p className="hint">
          Each field filters the next. Nodes that are not connected cannot take
          sessions.
        </p>
        <div className="grid-2">
          <Field label="Node">
            <Select
              value={node}
              options={nodeOptions(nodes, statuses)}
              onChange={(next) => {
                setNode(next);
                setHarness("");
                setProfile("");
              }}
            />
          </Field>
          <Field label="Harness">
            <Select
              value={chosenHarness?.kind ?? null}
              options={harnessOptions(harnesses)}
              placeholder="no harness found"
              disabled={harnesses.length === 0}
              onChange={(next) => {
                setHarness(next);
                setProfile("");
              }}
            />
          </Field>
          <Field label="Profile">
            <Select
              value={profile}
              options={profileOptions(profiles)}
              onChange={setProfile}
            />
          </Field>
          <Field label="Model" hint={routeHint}>
            <Select
              value={route}
              options={routeOptions(routes ?? [], profiles)}
              onChange={(next) => {
                setProfile(
                  next === "" ? "" : (profileOn(profiles, next)?.name ?? ""),
                );
              }}
            />
          </Field>
        </div>
        <Field label="Working directory">
          <TextField
            mono
            placeholder="the node's home directory"
            value={cwd}
            onChange={setCwd}
          />
        </Field>
        <FieldGroup
          label="Mode"
          hint={
            chosenHarness?.structured.state === "unavailable"
              ? `Structured is unavailable: ${chosenHarness.structured.reason}`
              : "Structured is a timeline (ACP). Terminal is the harness TUI in a PTY."
          }
          error={error}
        >
          <Segmented
            label="Mode"
            value={effectiveMode}
            onChange={setMode}
            options={[
              {
                value: "structured",
                label: "Structured",
                icon: "message",
                disabled: !structuredAvailable,
              },
              { value: "terminal", label: "Terminal", icon: "terminal" },
            ]}
          />
        </FieldGroup>
      </form>
    </Dialog>
  );
}
