//! Common-input checks precede any prompt and are repeated for later-ready targets.
use super::endpoint::{Endpoint, READ};
use super::worker::Entry;
use super::{ComparisonInput, Failure, FailureKind, Phase, TargetStatus};
use vessel_proto::conversation::GetConversation;
use vessel_proto::{AttachmentKind, Conversation};

pub(super) async fn shared(
    endpoint: &mut Endpoint,
    entries: &[Entry],
    input: &ComparisonInput,
) -> Result<(), Failure> {
    let ready: Vec<_> = entries
        .iter()
        .filter(|entry| matches!(entry.report.status, TargetStatus::Ready))
        .collect();
    if ready.is_empty() {
        return Err(Failure::new(Phase::Validate, FailureKind::Input));
    }
    if let Some(source) = &input.source
        && !ready
            .iter()
            .any(|entry| entry.report.resources.session == Some(source.session))
    {
        return Err(Failure::new(Phase::Validate, FailureKind::Source));
    }
    for entry in ready {
        target(endpoint, entry, input).await?;
    }
    Ok(())
}
pub(super) async fn target(
    endpoint: &mut Endpoint,
    entry: &Entry,
    input: &ComparisonInput,
) -> Result<(), Failure> {
    let session = entry
        .report
        .resources
        .session
        .ok_or_else(|| Failure::new(Phase::Validate, FailureKind::Source))?;
    let conversation = endpoint
        .call(
            Phase::Validate,
            GetConversation { session },
            super::endpoint::after(READ),
        )
        .await?;
    check(&conversation, input)
}
fn check(conversation: &Conversation, input: &ComparisonInput) -> Result<(), Failure> {
    let invalid = || Failure::new(Phase::Validate, FailureKind::Input);
    // A missing handshake report cannot silently assume media capabilities.
    let abilities = conversation.abilities.ok_or_else(invalid)?;
    if let Some(source) = &input.source {
        let limits = &conversation.limits;
        if u32::try_from(source.files.len()).map_err(|source| {
            let mut failure = Failure::count(Phase::Validate, source);
            failure.kind = FailureKind::Input;
            failure
        })? > limits.per_prompt
        {
            return Err(invalid());
        }
        let mut inline = 0_u64;
        let mut total = 0_u64;
        for file in &source.files {
            if file.kind != AttachmentKind::of(&file.mime) || file.size > file.kind.max_size(limits)
            {
                return Err(invalid());
            }
            let supported = match file.kind {
                AttachmentKind::Image => abilities.image,
                AttachmentKind::Audio => abilities.audio,
                AttachmentKind::File => true,
            };
            if !supported {
                return Err(invalid());
            }
            total = total.checked_add(file.size).ok_or_else(invalid)?;
            if file.kind.is_inline() {
                inline = inline.checked_add(file.size).ok_or_else(invalid)?;
            }
        }
        if inline > limits.inline_per_prompt || total > limits.per_session {
            return Err(invalid());
        }
    }
    Ok(())
}
