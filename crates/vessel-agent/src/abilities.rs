//! What an agent can do, read from its `initialize` answer (protocol 10):
//! steer a running turn, resume a conversation, take images or audio.
//!
//! [Decision Log]
//! - Purpose: a control a client offers (steer, start again, attach an
//!   image) must be one the agent takes; the agent says so in its
//!   handshake, and nowhere else.
//! - Steering is not in ACP's capabilities: it is the steering extension,
//!   advertised in the answer's top-level `_meta` as `steering.supported`
//!   (Claude's adapter, `codex-acp` and `OpenCode` do; agent-orchestrator
//!   reads the same flag). `_meta` is open JSON, so it is decoded here into
//!   a small typed struct, once per agent start; anything else in it is
//!   ignored.

use agent_client_protocol::v1::schema as acp;
use serde::Deserialize;
use vessel_proto::AgentAbilities;

/// The part of `_meta` Vessel reads.
#[derive(Debug, Default, Deserialize)]
struct Extensions {
    #[serde(default)]
    steering: Option<Supported>,
}

/// An extension's `{ "supported": true }`.
#[derive(Debug, Default, Deserialize)]
struct Supported {
    #[serde(default)]
    supported: bool,
}

/// The abilities an `initialize` answer states.
pub(crate) fn of(initialized: &acp::InitializeResponse) -> AgentAbilities {
    let capabilities = &initialized.agent_capabilities;
    AgentAbilities {
        steer: steering(initialized.meta.as_ref()),
        resume: capabilities.load_session,
        image: capabilities.prompt_capabilities.image,
        audio: capabilities.prompt_capabilities.audio,
        // ACP's unstable `session/fork` (protocol 14, ADR 0052).
        fork: capabilities.session_capabilities.fork.is_some(),
    }
}

/// Whether `_meta` says `steering.supported: true`. A `_meta` of another
/// shape says nothing (false), as an agent without it does.
fn steering(meta: Option<&acp::Meta>) -> bool {
    let Some(meta) = meta else {
        return false;
    };
    serde_json::from_value::<Extensions>(serde_json::Value::Object(meta.clone()))
        .ok()
        .and_then(|extensions| extensions.steering)
        .is_some_and(|steering| steering.supported)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(json: &str) -> acp::InitializeResponse {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn claudes_answer_steers_resumes_and_takes_images() {
        // Trimmed from `claude-agent-acp` 0.100's `initialize` answer.
        let abilities = of(&answer(
            r#"{"protocolVersion":1,"agentCapabilities":{"loadSession":true,
                "promptCapabilities":{"image":true,"embeddedContext":true}},
                "_meta":{"steering":{"supported":true}}}"#,
        ));
        assert_eq!(
            abilities,
            AgentAbilities {
                steer: true,
                resume: true,
                image: true,
                audio: false,
                fork: false,
            }
        );
    }

    #[test]
    fn without_the_extension_there_is_no_steering() {
        // Gemini's: audio too, no `_meta`.
        let gemini = of(&answer(
            r#"{"protocolVersion":1,"agentCapabilities":{"loadSession":true,
                "promptCapabilities":{"image":true,"audio":true,"embeddedContext":true}}}"#,
        ));
        assert!(!gemini.steer);
        assert!(gemini.audio);
        for meta in [
            r#"{"steering":{"supported":false}}"#,
            r#"{"steering":true}"#,
            r#"{"other":{"supported":true}}"#,
        ] {
            let json = format!(r#"{{"protocolVersion":1,"_meta":{meta}}}"#);
            assert!(!of(&answer(&json)).steer, "{meta}");
        }
        assert_eq!(
            of(&answer(r#"{"protocolVersion":1}"#)),
            AgentAbilities::default()
        );
    }
}
