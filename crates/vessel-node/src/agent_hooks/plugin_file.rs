//! `OpenCode`'s plugin file (Paseo's `opencode/opencode-plugin.js`): a whole
//! file of Vessel's own, which reports `OpenCode`'s events through
//! `vessel hooks opencode <event>`.
//!
//! [Decision Log]
//! - Purpose: `OpenCode` has no hook commands in its configuration; a plugin
//!   in its `plugins` folder sees its events.
//! - Choice: Paseo's plugin, event for event. It differs in three ways:
//!   - the names are Vessel's (`VESSEL_TERMINAL_ACTIVITY_ID`, the file
//!     `vessel-terminal-activity.js`);
//!   - it runs `$VESSEL_HOOK_CLI` when set, as the hook commands of the other
//!     agents do (Paseo's plugin always runs `paseo`);
//!   - only Vessel's exact text is Vessel's: installing over another text at
//!     the same path, or removing one, leaves it as it is, with one logged
//!     line ([`Outcome::Invalid`]). Paseo replaces or removes the file at its
//!     path whatever it holds. If the text ever changes, [`ours`] must keep
//!     recognising the old text, so the new version replaces it.

use std::path::Path;

use super::files::{self, Outcome};

/// The plugin's path under `OpenCode`'s home.
const FILE: [&str; 2] = ["plugins", "vessel-terminal-activity.js"];

/// The plugin (Paseo's `OPENCODE_PLUGIN_SOURCE`, with Vessel's names).
/// `OpenCode` 1 calls `server()`; `OpenCode` 2 calls `setup()`. Reports are
/// queued in order, so an older busy report cannot overwrite idle.
const SOURCE: &str = r#"const V1_STATUS_EVENTS = {
  busy: "session.status.busy",
  retry: "session.status.retry",
  idle: "session.status.idle",
};

const V2_EVENTS = {
  "session.execution.started": "session.status.busy",
  "session.execution.succeeded": "session.status.idle",
  "session.execution.failed": "session.status.idle",
  "session.execution.interrupted": "session.status.idle",
  "permission.asked": "permission.asked",
  "permission.replied": "permission.replied",
};

function vesselEventForV1(event) {
  const type = event.type;
  if (type === "permission.asked") return "permission.asked";
  if (type === "permission.replied") return "permission.replied";
  if (type !== "session.status") return null;
  return V1_STATUS_EVENTS[event.properties.status.type] ?? null;
}

let pendingHook = Promise.resolve();

function runVesselHook(event) {
  if (!process.env.VESSEL_TERMINAL_ACTIVITY_ID) return;
  pendingHook = pendingHook.then(async () => {
    try {
      const cli = process.env.VESSEL_HOOK_CLI || "vessel";
      const child = Bun.spawn([cli, "hooks", "opencode", event], {
        stdin: "ignore",
        stdout: "ignore",
        stderr: "ignore",
      });
      await child.exited;
    } catch {}
  });
  return pendingHook;
}

export default {
  id: "vessel-terminal-activity",
  server() {
    return {
      event: async ({ event }) => {
        const vesselEvent = vesselEventForV1(event);
        if (vesselEvent) await runVesselHook(vesselEvent);
      },
    };
  },
  setup(ctx) {
    const controller = new AbortController();
    void (async () => {
      for await (const event of ctx.event.subscribe({ signal: controller.signal })) {
        const vesselEvent = V2_EVENTS[event.type];
        if (vesselEvent) await runVesselHook(vesselEvent);
      }
    })().catch(() => {});
    return () => controller.abort();
  },
};
"#;

/// Whether the plugin text is exactly Vessel's.
fn ours(text: &str) -> bool {
    text == SOURCE
}

/// Install (`on`) or remove the plugin under `home`.
pub(super) fn apply(home: &Path, on: bool) -> Outcome {
    let path = FILE
        .iter()
        .fold(home.to_path_buf(), |path, part| path.join(part));
    let current = match std::fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Outcome::Failed(path, error),
    };
    match (on, current) {
        (true, None) => files::replace(&path, SOURCE.as_bytes()),
        (true, Some(text)) if ours(&text) => Outcome::Unchanged,
        (false, None) => Outcome::Unchanged,
        (false, Some(text)) if ours(&text) => files::remove(&path),
        (_, Some(_)) => Outcome::Invalid(path),
    }
}

#[cfg(test)]
pub(super) const fn source() -> &'static str {
    SOURCE
}
