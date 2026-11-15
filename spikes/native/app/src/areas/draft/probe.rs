//! Asking a machine what the draft's agent offers, and taking the answer
//! (split from `mod.rs` for the file budget, docs/QUALITY.md M3).

use vessel_proto::{HarnessRef, SessionOption};

use crate::shell::Shell;

impl Shell {
    /// Ask the draft's machine what its agent offers: when not asked yet,
    /// or (`again`) to refresh an answer or retry a failure.
    pub(crate) fn probe(&mut self, again: bool) {
        let Some(key) = self.areas.draft.settled.clone() else {
            return;
        };
        self.probe_agent(key, again);
    }

    /// Ask what `key`'s agent offers on its machine: when nothing is
    /// known of it yet, or (`again`) when a control opens, as the store
    /// allows (one request in flight, none soon after an answer; track P's
    /// `areas::composer::offers`).
    pub(crate) fn probe_agent(&mut self, key: (String, HarnessRef), again: bool) {
        let (machine, harness) = key;
        if again || !self.offers_known(&machine, &harness) {
            self.want_offers(machine, harness, vessel_app_view::offers::Why::Open);
        }
    }

    /// An agent's options arrived; the store takes them (a failure to
    /// refresh keeps what was shown, said as not fresh).
    pub(crate) fn options_arrived(
        &mut self,
        machine: String,
        harness: HarnessRef,
        outcome: Result<Vec<SessionOption>, String>,
    ) {
        if let Ok(options) = &outcome {
            // Which ids an agent sends, for the effort table
            // (`view::effort::ROLES`): ids, categories and kinds only.
            let shape: Vec<String> = options
                .iter()
                .map(|option| match &option.kind {
                    vessel_proto::OptionKind::Select { values, .. } => format!(
                        "{} ({:?}, select of {})",
                        option.id.as_str(),
                        option.category,
                        values
                            .iter()
                            .map(|value| value.id.as_str())
                            .collect::<Vec<_>>()
                            .join("|")
                    ),
                    vessel_proto::OptionKind::Toggle { .. } => {
                        format!("{} ({:?}, toggle)", option.id.as_str(), option.category)
                    }
                    vessel_proto::OptionKind::Unknown => {
                        format!("{} ({:?}, unknown)", option.id.as_str(), option.category)
                    }
                })
                .collect();
            tracing::info!(agent = ?harness, options = %shape.join(", "), "an agent's options");
        }
        self.offers_arrived(machine, harness, outcome);
    }
}
