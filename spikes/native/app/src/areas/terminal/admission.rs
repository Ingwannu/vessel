//! Frontend admission precedes any draft/hydration change; link still owns authentication.
//! The opaque value carries validated metadata once, with no cache or connection lookup.
use super::{Binding, NodeName};
use crate::{areas::session::Open, shell::Shell};
use vessel_app_kit::ui::Context;
use vessel_proto::TerminalSize;

/// Admitted navigation metadata, consumed once to begin the corresponding presentation.
#[derive(Debug)]
pub(crate) struct Admission(Projection);
#[derive(Debug)]
enum Projection {
    Structured,
    Terminal { binding: Binding, machine: NodeName },
}
impl Shell {
    /// Validate the retained destination before a caller switches drafts or issues reads.
    pub(crate) fn terminal_admit(
        &mut self,
        open: &Open,
        cx: &mut Context<'_, Self>,
    ) -> Option<Admission> {
        if open.structured {
            return Some(Admission(Projection::Structured));
        }
        let bound = open
            .binding
            .as_ref()
            .filter(|binding| {
                binding.host.as_str() == open.machine && binding.address.session == open.session
            })
            .and_then(|binding| {
                binding
                    .host
                    .as_str()
                    .parse::<NodeName>()
                    .ok()
                    .map(|machine| (binding, machine))
            });
        if let Some((binding, machine)) = bound {
            Some(Admission(Projection::Terminal {
                binding: binding.clone(),
                machine,
            }))
        } else {
            self.notice = Some(self.words.sessions().navigation_refused().to_owned());
            cx.notify();
            None
        }
    }
    /// Consume the metadata; only link's synchronous node check can authorize the actual read.
    pub(crate) fn terminal_follow_admitted(
        &mut self,
        admission: Admission,
        cx: &mut Context<'_, Self>,
    ) {
        match admission.0 {
            Projection::Structured => self.terminal_closed(),
            Projection::Terminal { binding, machine } => {
                self.terminal_opened_bound(machine, binding, TerminalSize::default(), cx);
            }
        }
    }
    /// Legacy refresh entry: admission and effects remain in the same owner.
    pub(crate) fn terminal_follow(&mut self, open: &Open, cx: &mut Context<'_, Self>) -> bool {
        let Some(admission) = self.terminal_admit(open, cx) else {
            return false;
        };
        self.terminal_follow_admitted(admission, cx);
        true
    }
}
