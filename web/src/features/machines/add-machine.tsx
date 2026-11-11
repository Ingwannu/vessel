// Adding a machine (track W2): one guided dialog over track E's ops,
// opened from Settings › Machines, the welcome screen and the command
// palette. The stages and what each row of the checklist says are
// `data/pairing.ts`'s decisions (its Decision Log says why this shape).
//
// The link is a credential until it is used: it lives in this dialog's
// state only, goes to `inspect` and `pair` and nowhere else, and is gone
// when the dialog closes.

import "./add-machine.css";

import { useState } from "react";

import { openNewSession, useFleet, useFleetState } from "../../context";
import { addMachine, closeAddMachine } from "../../data/add-machine";
import { inspectLink, type Offer, pairMachine } from "../../data/book";
import { nameProblem } from "../../data/machines";
import { nextStep } from "../../data/outcome";
import {
  checklistState,
  type Pairing,
  pairingChecks,
} from "../../data/pairing";
import { useStore } from "../../data/store";
import { Button, Dialog } from "../../ui/kit";
import { AddChecklist } from "./add-checklist";
import { AddConfirmStep } from "./add-confirm-step";
import { AddLinkStep } from "./add-link-step";

/** Mounted once, by the app's root; draws the dialog while it is open. */
export function AddMachineHost() {
  const open = useStore(addMachine, (value) => value);
  return open ? <AddMachineDialog onClose={closeAddMachine} /> : null;
}

type Stage =
  | { stage: "link"; checking: boolean; error: string | null }
  | { stage: "confirm"; offer: Offer }
  | { stage: "connect"; name: string; pairing: Pairing };

function AddMachineDialog({ onClose }: { onClose: () => void }) {
  const fleet = useFleet();
  const book = useFleetState((state) => state.book);
  const [link, setLink] = useState("");
  const [name, setName] = useState("");
  const [stage, setStage] = useState<Stage>({
    stage: "link",
    checking: false,
    error: null,
  });
  const key =
    stage.stage === "connect" && stage.pairing.kind === "paired"
      ? stage.pairing.node.key
      : null;
  const status = useFleetState((state) =>
    key === null ? undefined : state.statuses[key],
  );
  const harnesses = useFleetState((state) =>
    key === null ? undefined : state.harnesses[key],
  );
  const readError = useFleetState((state) =>
    key === null ? undefined : state.errors[key],
  );
  const nameError = nameProblem(name, book ?? []);

  const inspect = () => {
    if (link.trim() === "" || stage.stage !== "link" || stage.checking) {
      return;
    }
    setStage({ stage: "link", checking: true, error: null });
    void inspectLink(fleet, link).then((outcome) => {
      setStage(
        outcome.ok
          ? { stage: "confirm", offer: outcome.value }
          : {
              stage: "link",
              checking: false,
              error:
                outcome.problem.from === "op" &&
                outcome.problem.reason === "failed"
                  ? `This is not a pairing link Vessel can read: ${outcome.problem.message}`
                  : `${outcome.problem.message} ${nextStep(outcome.problem)}`,
            },
      );
    });
  };
  const connect = () => {
    if (stage.stage !== "confirm" || nameError !== null) {
      return;
    }
    const chosen = name;
    setStage({ stage: "connect", name: chosen, pairing: { kind: "pairing" } });
    void pairMachine(fleet, chosen, link).then((outcome) => {
      setStage({
        stage: "connect",
        name: chosen,
        pairing: outcome.ok
          ? { kind: "paired", node: outcome.value }
          : { kind: "refused", problem: outcome.problem },
      });
    });
  };

  switch (stage.stage) {
    case "link":
      return (
        <Dialog
          title="Add a machine"
          onClose={onClose}
          footer={
            <>
              <Button variant="ghost" onClick={onClose}>
                Cancel
              </Button>
              <Button
                variant="primary"
                busy={stage.checking}
                disabled={link.trim() === ""}
                onClick={inspect}
              >
                Continue
              </Button>
            </>
          }
        >
          <AddLinkStep
            link={link}
            onLink={(next) => {
              setLink(next);
              setStage({ stage: "link", checking: false, error: null });
            }}
            error={stage.error}
            onSubmit={inspect}
          />
        </Dialog>
      );
    case "confirm":
      return (
        <Dialog
          title="Connect to this machine?"
          onClose={onClose}
          footer={
            <>
              <Button
                variant="ghost"
                onClick={() => {
                  setStage({ stage: "link", checking: false, error: null });
                }}
              >
                Back
              </Button>
              <Button
                variant="primary"
                disabled={nameError !== null}
                onClick={connect}
              >
                Connect
              </Button>
            </>
          }
        >
          <AddConfirmStep
            offer={stage.offer}
            name={name}
            onName={setName}
            nameError={nameError}
            onSubmit={connect}
          />
        </Dialog>
      );
    case "connect": {
      const checks = pairingChecks(stage.pairing, status, harnesses, readError);
      const where = checklistState(checks);
      const refused = stage.pairing.kind === "refused";
      return (
        <Dialog
          title={
            where === "done"
              ? `${stage.name} is ready`
              : where === "stopped"
                ? `${stage.name} is not ready`
                : `Adding ${stage.name}`
          }
          onClose={onClose}
          footer={
            <>
              {refused ? (
                <Button
                  variant="ghost"
                  onClick={() => {
                    setLink("");
                    setStage({ stage: "link", checking: false, error: null });
                  }}
                >
                  Use another link
                </Button>
              ) : null}
              <Button
                variant={where === "done" ? "ghost" : "secondary"}
                onClick={onClose}
              >
                Close
              </Button>
              {where === "done" && key !== null ? (
                <Button
                  variant="primary"
                  onClick={() => {
                    onClose();
                    openNewSession(key);
                  }}
                >
                  Start a session there
                </Button>
              ) : null}
            </>
          }
        >
          <AddChecklist checks={checks} />
        </Dialog>
      );
    }
  }
}
