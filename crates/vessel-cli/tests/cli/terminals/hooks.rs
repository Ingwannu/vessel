//! `vessel hooks` end to end (ADR 0089, owed at assignment): run inside a
//! Vessel terminal, as an agent's hook runs it, the command reports that
//! terminal's activity through the node's local endpoint, and the node's
//! terminal list shows it. A standalone Esc typed into the terminal ends a
//! turn's work, as Paseo's input fallback does
//! (`terminal/terminal.ts:180-182,1164-1166`).
//!
//! Unix only: the terminal runs `/bin/sh`.

use std::time::Duration;

use bytes::Bytes;
use vessel_client::NodeClient;
use vessel_proto::notifications::{TerminalActivity, TerminalAttentionReason};
use vessel_proto::terminals::{GetTerminal, SendTerminal};
use vessel_proto::{TerminalBytes, TerminalId, TerminalInput};

use super::start;
use crate::support::within;

/// Type `text` into the terminal.
async fn type_in(owner: &NodeClient, terminal: TerminalId, text: &str) {
    let _ack = owner
        .call(SendTerminal {
            terminal,
            input: TerminalInput::Data {
                data: TerminalBytes::new(Bytes::from(text.to_owned())),
            },
        })
        .await
        .unwrap();
}

/// Run `vessel hooks claude <event>` in the terminal, as Claude Code's
/// hook would.
async fn hook(owner: &NodeClient, terminal: TerminalId, event: &str) {
    let vessel = env!("CARGO_BIN_EXE_vessel").replace('\'', "'\\''");
    type_in(
        owner,
        terminal,
        &format!("'{vessel}' hooks claude {event} </dev/null\n"),
    )
    .await;
}

/// Wait until the terminal's activity is `wanted`; panics after 20 s with
/// what it was.
async fn until_activity(owner: &NodeClient, terminal: TerminalId, wanted: TerminalActivity) {
    let mut last = None;
    for _ in 0..200 {
        let info = owner
            .call(GetTerminal {
                terminal,
                changed_from: None,
            })
            .await
            .unwrap();
        last = info.attention.map(|attention| attention.activity);
        if last == Some(wanted) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("the terminal's activity stayed {last:?}, not {wanted:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_hook_in_a_vessel_terminal_reports_its_activity_and_esc_ends_a_turn() {
    within(async {
        let fixture = start().await;
        let session = fixture.run_detached().await;
        let opened = fixture
            .vessel(&["shell", "--session", &session, "--detached"], b"")
            .await;
        assert!(opened.ok, "{}", opened.stderr);
        let terminal: TerminalId = opened
            .stdout
            .split_whitespace()
            .next()
            .unwrap()
            .parse()
            .unwrap();
        let owner = fixture.owner().await;

        // A turn: working while the prompt runs, finished at its end.
        hook(&owner, terminal, "UserPromptSubmit").await;
        until_activity(&owner, terminal, TerminalActivity::Working).await;
        hook(&owner, terminal, "Stop").await;
        until_activity(
            &owner,
            terminal,
            TerminalActivity::Attention {
                reason: TerminalAttentionReason::Finished,
            },
        )
        .await;

        // An interrupted turn: Claude Code runs no `Stop` hook, so Esc alone
        // ends the work, and nothing is finished.
        hook(&owner, terminal, "UserPromptSubmit").await;
        until_activity(&owner, terminal, TerminalActivity::Working).await;
        type_in(&owner, terminal, "\x1b").await;
        until_activity(&owner, terminal, TerminalActivity::Unknown).await;
    })
    .await;
}
