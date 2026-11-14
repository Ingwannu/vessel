//! The agents' lists the controls draw (owner word 28): read from
//! `vessel_app_view::offers` (what is shown, when to ask), asked for
//! through the link, handed over from the link's file at connect. The
//! draft's model, effort and permissions controls, its model list's agent
//! rows and Settings › Agents' defaults read from here (`offered`,
//! `offer_options`) rather than keeping answers of their own; a running
//! session's controls stay its own report.

use std::time::Instant;

use vessel_app_link::{Command, KeptLists};
use vessel_app_view::offers::{Anchored, Arrival, OfferKey, Offered, Why};
use vessel_app_view::{Fleet, Reach};
use vessel_proto::{AgentCommand, HarnessRef, SessionOption};

use super::ControlsTarget;

use crate::shell::Shell;

impl Shell {
    /// What a control shows for `agent` on `machine` now.
    pub(crate) fn offered(&self, machine: &str, agent: &HarnessRef) -> Offered<'_> {
        self.areas
            .composer
            .offers
            .offered(&OfferKey::new(machine, agent.clone()))
    }

    /// `agent`'s list on `machine`, however current, if there is one.
    pub(crate) fn offer_options(
        &self,
        machine: &str,
        agent: &HarnessRef,
    ) -> Option<&[SessionOption]> {
        self.areas
            .composer
            .offers
            .options(&OfferKey::new(machine, agent.clone()))
    }

    /// Whether the store has anything for `agent` on `machine` (a list, or
    /// a request or an answer of this run).
    pub(crate) fn offers_known(&self, machine: &str, agent: &HarnessRef) -> bool {
        self.areas
            .composer
            .offers
            .known(&OfferKey::new(machine, agent.clone()))
    }

    /// Whether a request for `agent` on `machine` is in flight.
    pub(crate) fn offers_asking(&self, machine: &str, agent: &HarnessRef) -> bool {
        self.areas
            .composer
            .offers
            .asking(&OfferKey::new(machine, agent.clone()), Instant::now())
    }

    /// Ask `agent` on `machine` what it offers, for `why`, unless the
    /// store says not now (one request in flight, a recent answer).
    pub(crate) fn want_offers(&mut self, machine: String, agent: HarnessRef, why: Why) {
        let key = OfferKey::new(machine, agent);
        if self.areas.composer.offers.want(&key, why, Instant::now()) {
            tracing::debug!(machine = %key.machine, agent = ?key.agent, ?why, "asking an agent's options");
            self.command(Command::Options {
                machine: key.machine,
                harness: key.agent,
            });
        }
    }

    /// An agent's answer: laid over the list when one of the draft's
    /// lists is open, else taken as sent; a failure keeps the list there
    /// is, said once as not fresh. The draft's lists open above their
    /// control (`Side::Above`) or as a sheet from the bottom: anchored by
    /// their bottom, so a new row is laid at the top.
    pub(crate) fn offers_arrived(
        &mut self,
        machine: String,
        harness: HarnessRef,
        outcome: Result<Vec<SessionOption>, String>,
    ) {
        if let Err(why) = &outcome {
            tracing::info!(%machine, agent = ?harness, %why, "an agent's options could not be read");
        }
        let open = self.offer_list_open().then_some(Anchored::Bottom);
        let arrival = self.areas.composer.offers.answered(
            OfferKey::new(machine, harness),
            outcome,
            open,
            Instant::now(),
        );
        tracing::debug!(?arrival, ?open, "an agent's options arrived");
    }

    /// The link's kept lists for a machine that just connected: shown at
    /// once where there is nothing newer, then the options asked again
    /// (commands are a session's report: the next session of the agent
    /// refreshes them).
    pub(crate) fn offers_kept(&mut self, machine: &str, lists: Vec<KeptLists>) {
        for kept in lists {
            let agent = kept.harness;
            let ask = kept.options.is_some();
            let key = OfferKey::new(machine, agent.clone());
            if self
                .areas
                .composer
                .offers
                .kept(key, kept.options, kept.commands)
                == Arrival::Changed
            {
                tracing::debug!(%machine, ?agent, "a kept list shown");
            }
            if ask {
                self.want_offers(machine.to_owned(), agent, Why::Connect);
            }
        }
    }

    /// The `/` commands of the session on screen: what it reported, else
    /// what its agent's last session reported (kept, so the menu opens on
    /// a list before this session's report arrives); `None` when neither.
    pub(crate) fn session_commands(&self) -> Option<&[AgentCommand]> {
        let ControlsTarget::Session { machine, agent, .. } = self.controls_target() else {
            return None;
        };
        self.areas
            .conversation
            .timeline
            .facts()
            .commands
            .as_deref()
            .or_else(|| {
                self.areas
                    .composer
                    .offers
                    .commands(&OfferKey::new(machine, agent))
            })
    }

    /// Records of the session on screen arrived: when its agent's reported
    /// commands differ from the kept ones, the store takes them and the
    /// link keeps them on the device.
    pub(crate) fn offers_commands_seen(&mut self) {
        let ControlsTarget::Session { machine, agent, .. } = self.controls_target() else {
            return;
        };
        let Some(reported) = self.areas.conversation.timeline.facts().commands.clone() else {
            return;
        };
        let key = OfferKey::new(machine.clone(), agent.clone());
        if self.areas.composer.offers.commands_reported(key, &reported) {
            self.command(Command::KeepCommands {
                machine,
                harness: agent,
                commands: reported,
            });
        }
    }

    /// The fleet was read again: a machine online before and now whose
    /// agents changed has its lists asked again (a machine that connected
    /// is the link's hand-over, [`Shell::offers_kept`]; one that went
    /// offline cannot answer, and its lists stay as they are).
    pub(crate) fn offers_fleet(&mut self, earlier: &Fleet) {
        let changed: Vec<String> = self
            .fleet
            .machines
            .iter()
            .filter(|machine| {
                matches!(machine.reach, Reach::Online)
                    && earlier
                        .machines
                        .iter()
                        .find(|was| was.name == machine.name)
                        .is_some_and(|was| {
                            matches!(was.reach, Reach::Online) && was.agents != machine.agents
                        })
            })
            .map(|machine| machine.name.clone())
            .collect();
        for machine in changed {
            for agent in self.areas.composer.offers.agents_on(&machine) {
                self.want_offers(machine.clone(), agent, Why::AgentsChanged);
            }
        }
    }

    /// Whether one of the draft's option lists is open (an answer is then
    /// laid over what it shows, so nothing under the pointer moves).
    fn offer_list_open(&self) -> bool {
        let lists = &self.areas.draft.lists;
        lists.model.is_open()
            || lists.thinking.is_open()
            || lists.mode.is_open()
            || lists.features.is_open()
            || self.areas.draft.effort.open
    }
}
