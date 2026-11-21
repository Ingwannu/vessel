//! Notices asked about before they are shown (ADR 0089
//! `notification.claim`; `vessel_app_view::notices::claims` holds the
//! Decision Log). A machine that admits claims hears its notices first, and
//! only what it lets through is told. Every other machine's notices are
//! told at once, as ADR 0114 does. While ADR 0089's protocol is unassigned,
//! no machine admits claims, so this path never runs.

use vessel_app_kit::ui::Context;
use vessel_app_link::Command;
use vessel_app_link::claims::{Answer, Ask};
use vessel_app_view::notices::Announce;
use vessel_app_view::notices::claims::{claims, shows};

use crate::shell::Shell;

impl Shell {
    /// Send a machine that admits claims its notices, one ask per machine,
    /// and give back the ones to tell now.
    pub(super) fn ask_before_telling(&self, told: Vec<Announce>) -> Vec<Announce> {
        let mut now = Vec::with_capacity(told.len());
        let mut asks: Vec<Ask> = Vec::new();
        for one in told {
            let machine = &one.notice.machine;
            let protocol = self
                .fleet
                .machines
                .iter()
                .find(|known| known.name == *machine)
                .and_then(|known| known.protocol);
            if !claims(protocol) {
                now.push(one);
                continue;
            }
            match asks.iter_mut().find(|ask| ask.machine == *machine) {
                Some(ask) => ask.told.push(one),
                None => asks.push(Ask {
                    machine: machine.clone(),
                    told: vec![one],
                }),
            }
        }
        for ask in asks {
            self.command(Command::Claims(ask));
        }
        now
    }

    /// A machine answered: tell what it admitted, and what its first inbox
    /// page did not hold (ADR 0114's path, the coordinator's rule).
    pub(crate) fn notices_claimed(&mut self, answer: Answer, cx: &mut Context<'_, Self>) {
        let told: Vec<Announce> = answer
            .asked
            .into_iter()
            .filter(|(_one, asked)| shows(*asked))
            .map(|(one, _asked)| one)
            .collect();
        if !told.is_empty() {
            self.tell(&told, cx);
        }
    }
}

#[cfg(test)]
mod tests;
