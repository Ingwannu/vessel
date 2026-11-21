import { useEffect, useRef, useState } from "react";

import { useFleet } from "../../context";
import type { ListedCommands } from "../../data/workspace/command-changes";
import { approveCommands } from "../../data/workspace/projects";
import {
  listScripts,
  runScript,
  type ScriptAction,
  scriptRefusal,
  stopScript,
} from "../../data/workspace/scripts";
import { rememberOpenedTerminal } from "../../data/workspace/terminals";
import type { NodeKey } from "../../wire/bridge";
import type {
  CommandsDigest,
  ProjectId,
  ScriptList,
  ScriptPlace,
  TerminalId,
} from "../../wire/proto";

interface Approval {
  commands: ListedCommands;
  action: ScriptAction;
}

const REFRESH_MS = 2_000;

export function useScripts({
  node,
  project,
  place,
  open,
  activity,
  onTerminal,
}: {
  node: NodeKey;
  project: ProjectId;
  place: ScriptPlace;
  open: boolean;
  activity: string;
  onTerminal: (terminal: TerminalId) => void;
}) {
  const fleet = useFleet();
  const [snapshot, setSnapshot] = useState<ScriptList | null>(null);
  const [listError, setListError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [approval, setApproval] = useState<Approval | null>(null);
  const [revision, refresh] = useState(0);
  const [busy, setBusy] = useState(false);
  const mounted = useRef(false);
  const pending = useRef(false);

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  useEffect(() => {
    let current = true;
    let timer: number | undefined;
    const read = async () => {
      const result = await listScripts(fleet, node, place);
      if (!current) return;
      if (result.ok) setSnapshot(result.value);
      setListError(result.ok ? null : result.problem.message);
      // The legacy list is unary. The closed control refreshes only on
      // workspace terminal changes, and the open list follows job exits.
      if (open)
        timer = window.setTimeout(() => {
          void read();
        }, REFRESH_MS);
    };
    void read();
    return () => {
      current = false;
      window.clearTimeout(timer);
    };
  }, [activity, fleet, node, open, place, revision]);

  const launch = async (action: ScriptAction, approved?: CommandsDigest) => {
    const result = await runScript(fleet, node, place, action);
    if (!mounted.current) return;
    if (!result.ok) {
      setError(result.problem.message);
      return;
    }
    const outcome = result.value;
    switch (outcome.outcome) {
      case "started":
        setApproval(null);
        if (action.kind === "run" || action.terminal !== outcome.terminal.id) {
          rememberOpenedTerminal(outcome.terminal.id);
        }
        onTerminal(outcome.terminal.id);
        return;
      case "needs_approval":
        if (outcome.commands.state !== "listed") {
          setError("The machine could not provide the commands for approval.");
        } else if (outcome.commands.digest === approved) {
          setError(
            "The run still requires approval. If the project changed, stop the old run and start the current script.",
          );
        } else {
          setApproval({ commands: outcome.commands, action });
        }
        return;
      case "refused":
        setError(scriptRefusal(outcome.reason));
    }
  };

  const perform = async (work: () => Promise<void>) => {
    if (pending.current) return;
    pending.current = true;
    setBusy(true);
    setError(null);
    try {
      await work();
    } finally {
      pending.current = false;
      if (mounted.current) {
        setBusy(false);
        refresh((value) => value + 1);
      }
    }
  };

  return {
    snapshot,
    listError,
    error,
    approval,
    busy,
    clearApproval: () => {
      if (!pending.current) setApproval(null);
    },
    run: (action: ScriptAction) => {
      void perform(() => launch(action));
    },
    stop: (terminal: TerminalId) => {
      void perform(async () => {
        const result = await stopScript(fleet, node, terminal);
        if (!result.ok && mounted.current) setError(result.problem.message);
      });
    },
    approve: () => {
      if (approval === null) return;
      void perform(async () => {
        const result = await approveCommands(
          fleet,
          node,
          project,
          approval.commands.digest,
          true,
        );
        if (!mounted.current) return;
        if (result.ok) {
          setApproval(null);
          await launch(approval.action, approval.commands.digest);
        } else {
          setError(result.problem.message);
        }
      });
    },
  };
}
