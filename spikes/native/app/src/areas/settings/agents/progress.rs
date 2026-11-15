//! Installation output belongs to its job, rather than the selected row.
use super::{note, page};
use crate::shell::Shell;
use std::hash::{DefaultHasher, Hash as _, Hasher as _};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{Context, Div, ParentElement as _, Styled as _, Window, px};
use vessel_app_kit::{
    Tokens,
    output::{self, Lines, Start},
    settings,
};
use vessel_app_view::agent_status::{
    install::Limit,
    pages::{InstallProgress, Operation},
};
use vessel_proto::HarnessRef;

pub(super) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<Div> {
    let page = page(shell)?;
    let words = shell.t.lang.agents();
    let (log, message) = match page.install() {
        InstallProgress::None => {
            if let Some(ticket) = page.pending()
                && let Operation::Install(_) = &ticket.operation
            {
                return Some(note(words.install_starting(), tokens));
            }
            return page
                .issue()
                .filter(|issue| matches!(issue.operation, Operation::Install(_)))
                .map(|issue| super::problem(&issue.source, shell, tokens));
        }
        InstallProgress::Following(log) => (log, words.install_following().to_owned()),
        InstallProgress::Finished { log, completion } => {
            (log, words.install_state(completion.state()))
        }
        InstallProgress::Unavailable(log) => (log, words.install_unavailable().to_owned()),
    };
    let variant = vessel_app_view::agent_status::notices::installation(page.install())?;
    let mut card = settings::card(tokens)
        .p(px(spacing::S4))
        .flex()
        .flex_col()
        .gap(px(spacing::S3))
        .child(vessel_app_kit::alert::alert(
            vessel_app_kit::alert::Content {
                title: Some(message.into()),
                description: None,
                action: None,
            },
            crate::badge::alert_variant(variant),
            vessel_app_kit::alert::Size::Small,
            tokens,
        ));
    card = card.child(super::detail::widgets::command_box(
        "running-install-command",
        &log.job.command,
        tokens,
        window,
        cx,
    ));
    if log.transcript.is_empty() {
        card = card.child(note(words.install_empty(), tokens));
    } else {
        let text = log.transcript.text();
        let mut hash = DefaultHasher::new();
        text.hash(&mut hash);
        card = card.child(output::lines_box(
            format!("agent-install-{}", log.job.id),
            hash.finish(),
            || Lines::text(&text),
            Start::End,
            tokens,
            window,
            cx,
        ));
    }
    if log.transcript.limit() == Limit::Clipped {
        card = card.child(note(words.install_clipped(), tokens));
    }
    if let Some(issue) = page
        .issue()
        .filter(|issue| matches!(issue.operation, Operation::Install(_)))
    {
        card = card.child(super::problem(&issue.source, shell, tokens));
    }
    let name = vessel_app_view::agent_name(&HarnessRef::Known(log.job.agent));
    Some(settings::section(words.install_title(&name), card, tokens))
}
