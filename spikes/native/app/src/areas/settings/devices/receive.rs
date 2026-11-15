//! Queue ingress and subsequent frame/callback checks share the SAME weak
//! actor lease. Copied Fleet facts are not proof that an admission is active.
//! No guard is minted in App and no queued reply keeps a transport alive.
use super::{ConnectionContext, Shell};
use vessel_app_kit::ui::Context;
use vessel_app_link::{
    DeliveryGuard,
    devices::{Answer, Received},
};

pub(super) struct Receipt {
    context: ConnectionContext,
    guard: DeliveryGuard,
}
impl Receipt {
    pub(super) fn validate(
        &self,
        context: &ConnectionContext,
    ) -> Result<(), vessel_app_view::connection::Refusal> {
        self.guard.validate(context)
    }
}
impl Shell {
    /// Fence a rendered/captured control before the next Fleet event arrives.
    pub(super) fn devices_validate(&mut self) {
        let retired = self
            .areas
            .settings
            .devices
            .receipt
            .as_ref()
            .and_then(|receipt| {
                receipt
                    .validate(&receipt.context)
                    .err()
                    .map(|_why| receipt.context.tag())
            });
        if let Some(tag) = retired {
            self.devices_retired(&tag);
        }
    }
    pub(crate) fn devices_answer(&mut self, answer: Answer, cx: &mut Context<'_, Self>) {
        if self
            .areas
            .settings
            .devices
            .page
            .check_context(answer.context())
            .is_err()
        {
            // In particular an old answer cannot retire a successor's writes.
            return;
        }
        match answer.receive() {
            Received::Completed {
                context,
                guard,
                outcome,
            } => {
                // No await/callback occurs between the actor's validation and
                // installation. Keep only its weak proof for later callbacks.
                self.areas.settings.devices.receipt = Some(Receipt {
                    context: context.clone(),
                    guard,
                });
                self.devices_outcome(&context, outcome, cx);
            }
            Received::Refused { context, failure } => {
                // No request was sent, so settle it as a definite refusal.
                let unavailable = matches!(
                    failure.problem(),
                    vessel_app_view::devices::Problem::TargetChanged
                        | vessel_app_view::devices::Problem::Offline
                );
                self.devices_outcome(&context, failure.into_outcome(), cx);
                if unavailable {
                    self.devices_retired(&context.tag());
                }
            }
            Received::Retired {
                context,
                failure: _,
            } => {
                // A dispatched write may already have taken effect. Drop its
                // returned data and retain input plus an unconfirmed notice.
                self.devices_retired(&context.tag());
            }
        }
    }
}

/// The lost result is independent of the successor's current access/read state.
pub(super) fn unconfirmed(
    shell: &Shell,
    tokens: &vessel_app_kit::Tokens,
) -> Option<vessel_app_kit::ui::AnyElement> {
    use vessel_app_kit::ui::{IntoElement as _, ParentElement as _, Styled as _, div, px};
    let machine = super::super::choices::chosen_machine(shell)?;
    let operation = shell.areas.settings.devices.page.unconfirmed(machine)?;
    Some(
        div()
            .mt(px(vessel_app_kit::theme::spacing::S3))
            .child(vessel_app_kit::alert::alert(
                vessel_app_kit::alert::Content {
                    title: Some(shell.words.device_authority().uncertain_title().into()),
                    description: Some(
                        div()
                            .child(shell.t.lang.devices().unconfirmed(operation, shell.words))
                            .into_any_element(),
                    ),
                    action: None,
                },
                crate::badge::alert_variant(operation.notice()),
                vessel_app_kit::alert::Size::Small,
                tokens,
            ))
            .into_any_element(),
    )
}
