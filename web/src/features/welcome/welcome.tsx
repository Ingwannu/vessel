// The first screen while no machine is connected (track W2): Paseo's
// welcome ("Connect your computer to get started") and GOAT's empty
// state, one heading, one sentence, one primary action. Under it, when
// this machine's own Vessel is the one not running, the command that
// starts it.
//
// [Decision Log]
// - Purpose: an empty board says nothing about what to do first.
// - Constraint: the bridge stops dialling this machine's node when it
//   finds nothing to dial (no token until the node has run once); only a
//   call or a retry starts it again, so starting the node in a terminal is
//   not noticed by itself.
// - Choice: retry it when the page gets focus back (the person ran the
//   command elsewhere and came back) and on "Check again"; the index
//   leaves this screen as soon as any machine connects (`firstScreen`).
// - Alternative: retrying on a timer while the screen is up; it would
//   dial a node that is not there every few seconds for as long as the tab
//   stays open, for no one.

import "./welcome.css";

import { useEffect } from "react";

import { useFleet, useFleetState } from "../../context";
import { openAddMachine } from "../../data/add-machine";
import { retryMachine } from "../../data/book";
import { HERE } from "../../data/machines";
import { START_COMMAND } from "../../data/pairing";
import { hereIsDown } from "../../data/welcome";
import { BrandMark, Button, CommandLine } from "../../ui/kit";

export function Welcome() {
  const fleet = useFleet();
  const book = useFleetState((state) => state.book);
  const statuses = useFleetState((state) => state.statuses);
  const down = hereIsDown(book, statuses);
  useEffect(() => {
    if (!down) {
      return undefined;
    }
    const check = () => {
      if (document.visibilityState === "visible") {
        void retryMachine(fleet, HERE);
      }
    };
    window.addEventListener("focus", check);
    document.addEventListener("visibilitychange", check);
    return () => {
      window.removeEventListener("focus", check);
      document.removeEventListener("visibilitychange", check);
    };
  }, [down, fleet]);
  return (
    <div className="welcome">
      <div className="welcome-column">
        <span className="welcome-mark" aria-hidden="true">
          <BrandMark size={28} />
        </span>
        <h2 className="welcome-title">Welcome to Vessel</h2>
        <p className="welcome-lead">
          Connect a machine to run its agents from here.
        </p>
        <Button variant="primary" icon="plus" onClick={openAddMachine}>
          Add a machine
        </Button>
        {down ? (
          <div className="welcome-here">
            <p className="welcome-here-title">Or use this machine</p>
            <p className="welcome-here-text">
              Vessel is not running here. Start it in a terminal, then come
              back:
            </p>
            <CommandLine command={START_COMMAND} />
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                void retryMachine(fleet, HERE);
              }}
            >
              Check again
            </Button>
          </div>
        ) : null}
      </div>
    </div>
  );
}
