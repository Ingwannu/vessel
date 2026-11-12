// Moving a session to another profile of its agent (DESIGN.md §7.4): the
// conversation continues under the other account, between turns. A
// session keeps its model route: the node refuses a move to a profile on
// another route, or between a route and none (ADR 0015), and the list
// says so before it is asked (data/agent-choices.ts).

import { useState } from "react";

import { useFleet, useFleetState } from "../../context";
import { agentName } from "../../data/agents";
import { moveProfile } from "../../data/calls";
import { moveChoices } from "../../data/agent-choices";
import { shortId } from "../../data/format";
import { Button, Dialog, Field, Select } from "../../ui/kit";
import { toast } from "../../ui/toasts";
import type { SessionInfo } from "../../wire/proto";
import type { NodeKey } from "../../wire/bridge";

export function MoveProfile({
  node,
  session,
  close,
  moved,
}: {
  node: NodeKey;
  session: SessionInfo;
  close: () => void;
  moved: () => void;
}) {
  const fleet = useFleet();
  const profiles = useFleetState((state) => state.profiles[node]);
  const choices = (profiles ?? []).filter(
    (profile) => profile.harness === session.spec.harness,
  );
  const route = session.model?.route ?? null;
  const [chosen, setChosen] = useState<string>(
    route === null
      ? ""
      : (choices.find(
          (profile) =>
            profile.model === route && profile.name !== session.profile,
        )?.name ??
          session.profile ??
          ""),
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const move = () => {
    setBusy(true);
    void moveProfile(
      fleet,
      node,
      session.id,
      chosen === "" ? null : chosen,
    ).then((outcome) => {
      if (!outcome.ok) {
        setBusy(false);
        setError(outcome.problem.message);
        return;
      }
      toast(
        `Moved ${shortId(session.id)} to ${chosen === "" ? "the default profile" : chosen}`,
      );
      moved();
    });
  };
  return (
    <Dialog
      title="Move to another profile"
      onClose={close}
      footer={
        <>
          <Button variant="ghost" onClick={close}>
            Cancel
          </Button>
          <Button variant="primary" busy={busy} disabled={busy} onClick={move}>
            Move
          </Button>
        </>
      }
    >
      <p className="hint">
        The conversation continues under the other account, between turns. Only
        profiles of {agentName(session.spec.harness)} on this machine are
        offered
        {route === null
          ? "."
          : `, on route ${route}: a session keeps the route it started on.`}
      </p>
      <Field label="Profile" error={error}>
        <Select
          value={chosen}
          options={moveChoices(profiles ?? [], session.spec.harness, route)}
          onChange={setChosen}
        />
      </Field>
    </Dialog>
  );
}
