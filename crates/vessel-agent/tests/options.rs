//! Protocol 9 against `fake-acp`: the options an agent reports, the
//! choices a session starts with, changes on a running session, the
//! agent's own changes, ACP's older modes, and probing.
//!
//! Tests wait on outputs, never on time; the timeout only turns a hang into
//! a failure.

use std::time::Duration;

use vessel_agent::{AgentSetup, Disconnect, InputError, OptionError, ProbeError};
use vessel_proto::{
    AgentEvent, ContentBlock, HarnessSessionId, OptionKind, SessionOption, StopReason,
};

use support::{Run, Seen, current, fake, select, setup, toggle, within};

#[cfg(test)]
mod support {
    use std::time::Duration;

    use vessel_agent::{
        AgentInput, AgentOutput, AgentOutputs, AgentProcess, AgentSetup, Disconnect, Signal,
    };
    use vessel_proc::CommandSpec;
    use vessel_proto::{
        AgentEvent, ChoiceValue, ContentBlock, OptionChoice, OptionId, OptionKind, OptionValueId,
        ResolvedMcpServers, SessionOption,
    };
    use vessel_testkit::Fake;

    const SAFETY_NET: Duration = Duration::from_secs(30);

    pub(super) async fn within<F: Future>(test: F) -> F::Output {
        vessel_testkit::within(SAFETY_NET, test)
            .await
            .expect("test timed out")
    }

    pub(super) fn fake() -> CommandSpec {
        CommandSpec::new(Fake::Acp.path().unwrap())
    }

    pub(super) fn setup(options: Vec<OptionChoice>) -> AgentSetup {
        AgentSetup {
            cwd: std::env::temp_dir(),
            model: None,
            resume: None,
            options,
            mcp_servers: ResolvedMcpServers::new(),
            handshake_timeout: vessel_agent::DEFAULT_HANDSHAKE_TIMEOUT,
            questions: false,
            fork: None,
            system_prompt: None,
            private_dir: None,
        }
    }

    pub(super) fn select(id: &str, value: &str) -> OptionChoice {
        OptionChoice {
            id: OptionId::new(id),
            value: ChoiceValue::Select {
                value: OptionValueId::new(value),
            },
        }
    }

    pub(super) fn toggle(id: &str, on: bool) -> OptionChoice {
        OptionChoice {
            id: OptionId::new(id),
            value: ChoiceValue::Toggle { on },
        }
    }

    /// One option's current value, by id.
    pub(super) fn current<'a>(options: &'a [SessionOption], id: &str) -> Option<&'a str> {
        options
            .iter()
            .find(|option| option.id.as_str() == id)
            .and_then(|option| match &option.kind {
                OptionKind::Select { current, .. } => Some(current.as_str()),
                OptionKind::Toggle { on: true } => Some("on"),
                OptionKind::Toggle { on: false } => Some("off"),
                OptionKind::Unknown => None,
            })
    }

    /// What a running agent reported, in order.
    #[derive(Debug, PartialEq, Eq)]
    pub(super) enum Seen {
        Options(Vec<SessionOption>),
        Event(AgentEvent),
    }

    pub(super) struct Run {
        pub(super) process: AgentProcess,
        pub(super) input: AgentInput,
        pub(super) outputs: AgentOutputs,
        pub(super) seen: Vec<Seen>,
        pub(super) disconnect: Option<Disconnect>,
    }

    impl Run {
        pub(super) fn start(command: &CommandSpec, setup: AgentSetup) -> Self {
            let (process, input, outputs) = AgentProcess::spawn(command, setup).unwrap();
            Self {
                process,
                input,
                outputs,
                seen: Vec::new(),
                disconnect: None,
            }
        }

        /// Read until `done` says so of what was just seen; `false` when the
        /// outputs ended first.
        pub(super) async fn until(&mut self, done: impl Fn(&Seen) -> bool) -> bool {
            while let Some(output) = self.outputs.next().await {
                let seen = match output {
                    AgentOutput::Event(AgentEvent::HarnessLog { .. })
                    | AgentOutput::Commands(_)
                    | AgentOutput::Usage(_)
                    | AgentOutput::Introduced(_) => continue,
                    AgentOutput::Event(event) => Seen::Event(event),
                    AgentOutput::Options(options) => Seen::Options(options),
                    AgentOutput::Disconnected(disconnect) => {
                        self.disconnect = Some(disconnect);
                        continue;
                    }
                };
                let stop = done(&seen);
                self.seen.push(seen);
                if stop {
                    return true;
                }
            }
            false
        }

        /// Read until ready (`true`) or the end (`false`).
        pub(super) async fn ready_or_end(&mut self) -> bool {
            self.until(|seen| matches!(seen, Seen::Event(AgentEvent::Ready { .. })))
                .await
        }

        pub(super) async fn ready(&mut self) {
            assert!(
                self.until(|seen| matches!(seen, Seen::Event(AgentEvent::Ready { .. })))
                    .await,
                "never ready: {:?} {:?}",
                self.seen,
                self.disconnect
            );
        }

        /// Every options report so far.
        pub(super) fn reports(&self) -> Vec<&[SessionOption]> {
            self.seen
                .iter()
                .filter_map(|seen| match seen {
                    Seen::Options(options) => Some(options.as_slice()),
                    Seen::Event(_) => None,
                })
                .collect()
        }

        /// The fake's own account of its values (prompt `options`).
        pub(super) async fn values(&mut self) -> String {
            self.input
                .prompt(vec![ContentBlock::Text {
                    text: "options".to_owned(),
                }])
                .await
                .unwrap();
            let mut reply = String::new();
            assert!(
                self.until(|seen| matches!(seen, Seen::Event(AgentEvent::TurnEnded { .. })))
                    .await
            );
            for seen in &self.seen {
                if let Seen::Event(AgentEvent::MessageChunk {
                    content: ContentBlock::Text { text },
                }) = seen
                    && text.starts_with("options ")
                {
                    text.clone_into(&mut reply);
                }
            }
            reply
        }

        pub(super) async fn stop(mut self) {
            drop(self.process.signal(Signal::Kill));
            let _code = self.process.wait().await;
        }
    }
}

/// The options come once before `Ready`, as they stand after the choices
/// (each applied in order, one already in effect skipped), and the agent
/// runs with them.
#[tokio::test]
async fn choices_are_applied_in_the_handshake_and_reported_before_ready() {
    within(async {
        let mut run = Run::start(
            &fake(),
            setup(vec![
                select("mode", "plan"),
                select("effort", "low"),
                select("model", "fake-small"),
                toggle("fast", true),
            ]),
        );
        run.ready().await;
        let [
            Seen::Options(options),
            Seen::Event(AgentEvent::Ready { .. }),
        ] = run.seen.as_slice()
        else {
            panic!("one report, then ready: {:?}", run.seen);
        };
        let values: Vec<_> = ["mode", "model", "effort", "fast"]
            .iter()
            .map(|id| current(options, id))
            .collect();
        assert_eq!(
            values,
            [Some("plan"), Some("fake-small"), Some("low"), Some("on")]
        );
        let Some(OptionKind::Select { values, .. }) = options
            .iter()
            .find(|option| option.id.as_str() == "effort")
            .map(|option| &option.kind)
        else {
            panic!("effort is a select");
        };
        let groups: Vec<_> = values.iter().map(|value| value.group.as_deref()).collect();
        assert_eq!(groups, [Some("Quick"), Some("Deep")]);
        assert_eq!(
            run.values().await,
            "options mode=plan model=fake-small effort=low fast=true"
        );
        run.stop().await;
    })
    .await;
}

/// An agent that offers nothing still reports, with an empty list: a
/// client tells "nothing offered" from "not known yet" by it.
#[tokio::test]
async fn an_agent_offering_nothing_reports_an_empty_list() {
    within(async {
        let mut run = Run::start(&fake().env("FAKE_ACP_OPTIONS", "none"), setup(Vec::new()));
        run.ready().await;
        let empty: &[SessionOption] = &[];
        assert_eq!(run.reports(), [empty]);
        assert_eq!(run.input.options().await.unwrap(), []);
        run.stop().await;
    })
    .await;
}

/// A choice the agent does not offer, or one it refuses, fails the
/// handshake, so the session never runs in a mode nobody asked for.
#[tokio::test]
async fn a_choice_not_offered_or_refused_fails_the_handshake() {
    within(async {
        for (choice, says) in [
            (select("mode", "yolo"), "option `mode` has no value `yolo`"),
            (
                select("sandbox", "on"),
                "the agent offers no option `sandbox`",
            ),
            (select("mode", "locked"), "the agent refused option `mode`"),
        ] {
            let mut run = Run::start(&fake(), setup(vec![choice]));
            assert!(!run.ready_or_end().await, "never ready");
            let Some(Disconnect::Handshake { message, .. }) = &run.disconnect else {
                panic!("a handshake failure: {:?}", run.disconnect);
            };
            assert!(message.contains(says), "{message}");
            assert!(run.reports().is_empty(), "nothing reported");
            run.stop().await;
        }
    })
    .await;
}

/// A running session's option changes: the agent's answer is the reply
/// and a report; a value in effect changes nothing; a value not offered
/// is refused before the agent hears of it; the agent's refusal is the
/// error.
#[tokio::test]
async fn options_change_on_a_running_session() {
    within(async {
        let mut run = Run::start(&fake(), setup(Vec::new()));
        run.ready().await;
        let changed = run.input.set_option(select("mode", "plan")).await.unwrap();
        assert_eq!(current(&changed, "mode"), Some("plan"));
        assert!(
            run.until(|seen| matches!(seen, Seen::Options(options) if current(options, "mode") == Some("plan")))
                .await
        );
        let same = run.input.set_option(select("mode", "plan")).await.unwrap();
        assert_eq!(same, changed, "in effect already");

        let error = run
            .input
            .set_option(select("mode", "yolo"))
            .await
            .unwrap_err();
        assert!(
            matches!(error, InputError::Option(OptionError::NotOffered { .. })),
            "{error:?}"
        );
        let error = run
            .input
            .set_option(toggle("mode", true))
            .await
            .unwrap_err();
        assert!(
            matches!(error, InputError::Option(OptionError::WrongKind { .. })),
            "{error:?}"
        );
        let error = run
            .input
            .set_option(select("mode", "locked"))
            .await
            .unwrap_err();
        assert!(matches!(error, InputError::Refused { .. }), "{error:?}");
        assert_eq!(
            current(&run.input.options().await.unwrap(), "mode"),
            Some("plan"),
            "a refusal changes nothing"
        );
        // Two reports: after the handshake, after the change.
        assert_eq!(run.reports().len(), 2);
        run.stop().await;
    })
    .await;
}

/// The agent changes its own mode mid-turn and says so: a new report.
#[tokio::test]
async fn the_agents_own_change_is_reported() {
    within(async {
        let mut run = Run::start(&fake(), setup(Vec::new()));
        run.ready().await;
        run.input
            .prompt(vec![ContentBlock::Text {
                text: "switch plan".to_owned(),
            }])
            .await
            .unwrap();
        assert!(
            run.until(|seen| matches!(
                seen,
                Seen::Event(AgentEvent::TurnEnded {
                    reason: StopReason::EndTurn,
                    usage: None,
                    ..
                })
            ))
            .await
        );
        let reports = run.reports();
        assert_eq!(reports.len(), 2, "{reports:?}");
        assert_eq!(
            reports.last().and_then(|options| current(options, "mode")),
            Some("plan")
        );
        run.stop().await;
    })
    .await;
}

/// An agent with ACP's older modes and no mode option gets one made from
/// them; choosing it is `session/set_mode`, and the agent's
/// `current_mode_update` is a report.
#[tokio::test]
async fn older_modes_are_a_mode_option() {
    within(async {
        let command = fake().env("FAKE_ACP_OPTIONS", "modes");
        let mut run = Run::start(&command, setup(vec![select("mode", "plan")]));
        run.ready().await;
        let first = run.reports().first().map(|options| options.to_vec());
        assert_eq!(
            first.as_deref().and_then(|options| current(options, "mode")),
            Some("plan"),
            "chosen at creation with set_mode"
        );
        let back = run
            .input
            .set_option(select("mode", "default"))
            .await
            .unwrap();
        assert_eq!(current(&back, "mode"), Some("default"));
        run.input
            .prompt(vec![ContentBlock::Text {
                text: "switch plan".to_owned(),
            }])
            .await
            .unwrap();
        assert!(
            run.until(|seen| matches!(seen, Seen::Options(options) if current(options, "mode") == Some("plan")))
                .await
        );
        run.stop().await;
    })
    .await;
}

/// A resumed session gets its choices after the load.
#[tokio::test]
async fn a_resumed_session_gets_its_choices_after_the_load() {
    within(async {
        let mut run = Run::start(
            &fake(),
            AgentSetup {
                resume: Some(HarnessSessionId::new("fake-session-9")),
                ..setup(vec![select("mode", "plan")])
            },
        );
        run.ready().await;
        assert_eq!(
            run.reports()
                .first()
                .and_then(|options| current(options, "mode")),
            Some("plan")
        );
        run.stop().await;
    })
    .await;
}

/// A probe answers what the agent offers by default and stops it.
#[tokio::test]
async fn a_probe_reads_what_the_agent_offers() {
    within(async {
        let probed = vessel_agent::probe(&fake(), setup(Vec::new()))
            .await
            .unwrap();
        let ids: Vec<_> = probed
            .options
            .iter()
            .map(|option| option.id.as_str())
            .collect();
        assert_eq!(ids, ["mode", "model", "effort", "fast"]);
        let none = vessel_agent::probe(&fake().env("FAKE_ACP_OPTIONS", "none"), setup(Vec::new()))
            .await
            .unwrap();
        assert!(none.options.is_empty());
    })
    .await;
}

/// What a real agent sends reaches the node as it was sent: the app's
/// pickers are checked against these shapes (`FAKE_ACP_OPTIONS=real`, from
/// the owner's capture of app build 0.0.0.0.5 with Claude Code): many
/// values, descriptions of several lines, long and Korean names, an on/off
/// option with no name.
#[tokio::test]
async fn a_real_agents_shapes_arrive_whole() {
    within(async {
        let probed =
            vessel_agent::probe(&fake().env("FAKE_ACP_OPTIONS", "real"), setup(Vec::new()))
                .await
                .unwrap();
        let ids: Vec<_> = probed
            .options
            .iter()
            .map(|option| option.id.as_str())
            .collect();
        assert_eq!(
            ids,
            ["mode", "model", "effort", "fast_mode", "ultracode", "agent"]
        );
        let values = |id: &str| match probed
            .options
            .iter()
            .find(|option| option.id.as_str() == id)
            .map(|option| &option.kind)
        {
            Some(OptionKind::Select { values, .. }) => values.clone(),
            other => panic!("{id} is not a select: {other:?}"),
        };
        assert_eq!(values("model").len(), 16);
        let agents = values("agent");
        assert_eq!(agents.len(), 16);
        assert!(
            agents
                .iter()
                .filter_map(|value| value.description.as_deref())
                .any(|description| description.lines().count() > 3),
            "a subagent's description keeps its lines"
        );
        assert!(
            agents
                .iter()
                .any(|value| value.name == "한국어 검토 에이전트")
        );
        let fast = probed
            .options
            .iter()
            .find(|option| option.id.as_str() == "fast_mode")
            .unwrap();
        assert!(fast.name.is_empty(), "the on/off option has no name");
        assert!(matches!(fast.kind, OptionKind::Toggle { on: false }));
    })
    .await;
}

/// A probe of an agent that cannot start says why, with what it wrote; one
/// stuck in the handshake ends at the deadline.
///
/// The agent's stderr is read by a task of its own, apart from its ACP
/// stream, so its last line can arrive after the disconnect: a probe that
/// stopped reading at the disconnect lost it now and then (4 runs in 80
/// under load, the line empty). The probe reads on until the stderr pump
/// has ended (`probe`), which this test pins.
#[tokio::test]
async fn a_failed_probe_says_why_and_never_hangs() {
    within(async {
        let early = vessel_agent::probe(&fake().env("FAKE_ACP_EXIT_EARLY", "1"), setup(Vec::new()))
            .await
            .unwrap_err();
        let ProbeError::Disconnected { log, .. } = &early else {
            panic!("the agent's end: {early:?}");
        };
        assert_eq!(log, &["fake-acp: not logged in"]);
        assert!(
            early
                .to_string()
                .contains("the agent said: fake-acp: not logged in"),
            "{early}"
        );

        let stuck = vessel_agent::probe(
            &fake().env("FAKE_ACP_HANG", "initialize"),
            AgentSetup {
                handshake_timeout: Duration::from_millis(300),
                ..setup(Vec::new())
            },
        )
        .await
        .unwrap_err();
        assert!(
            matches!(
                stuck,
                ProbeError::Disconnected {
                    disconnect: Disconnect::Handshake { .. },
                    ..
                } | ProbeError::TimedOut { .. }
            ),
            "{stuck:?}"
        );
    })
    .await;
}

/// Commands can arrive after Ready; a catalog probe must observe both without a prompt.
#[tokio::test]
async fn a_command_probe_waits_for_the_catalog_and_validates_selected_options() {
    within(async {
        let catalog =
            vessel_agent::probe_catalog_with(vessel_agent::Driver::Acp, &fake(), setup(Vec::new()))
                .await
                .unwrap();
        let names: Vec<_> = catalog
            .commands
            .unwrap()
            .into_iter()
            .map(|command| command.name.to_string())
            .collect();
        assert_eq!(names, ["compact", "review"]);
        let invalid = setup(vec![select("mode", "not-offered")]);
        assert!(matches!(
            vessel_agent::probe_catalog_with(vessel_agent::Driver::Acp, &fake(), invalid).await,
            Err(ProbeError::Disconnected { .. })
        ));
    })
    .await;
}

#[tokio::test]
async fn unsupported_native_catalogs_refuse_before_spawning_the_command() {
    for driver in [
        vessel_agent::Driver::ClaudeStream,
        vessel_agent::Driver::OpencodeHttp,
    ] {
        let command = vessel_proc::CommandSpec::new("must-not-run-catalog-provider");
        assert!(
            matches!(vessel_agent::probe_catalog_with(driver,&command,setup(Vec::new())).await,
            Err(ProbeError::CatalogUnsupported { driver: refused }) if refused == driver)
        );
    }
}
