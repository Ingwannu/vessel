//! Shared kit controls display typed source disclosure before a grant mutation.
use super::rows::{Density, message};
use crate::shell::Shell;
use vessel_app_kit::{
    Tokens, outline_button, settings,
    theme::spacing,
    ui::{
        AnyElement, Context, FluentBuilder as _, IntoElement as _, ParentElement as _,
        StatefulInteractiveElement as _, Styled as _, div, px,
    },
};
use vessel_app_view::usage::{source_availability, source_disclosure};
use vessel_proto::{
    limits::ExternalLookup,
    usage_reports::{
        SetUsageSourceLookup, UsageReports, UsageSourceAvailability, UsageSourceConsent,
    },
};
pub(super) fn render(
    shell: &Shell,
    reports: &UsageReports,
    density: Density,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let text = shell.words.usage();
    let state = &shell.areas.usage;
    let receipt = super::read_states::context(shell);
    let compact = matches!(density, Density::Compact);
    let mut body = div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(spacing::S2));
    for source in reports.sources.as_slice() {
        let next = match source.consent {
            UsageSourceConsent::InheritedGlobal => continue,
            UsageSourceConsent::Off => ExternalLookup::On,
            UsageSourceConsent::Granted { version } if version != source.disclosure.version => {
                ExternalLookup::On
            }
            UsageSourceConsent::Granted { .. } => ExternalLookup::Off,
        };
        let label = match next {
            ExternalLookup::On => text.enable_source(),
            ExternalLookup::Off => text.disable_source(),
        };
        let enabled = source.availability != UsageSourceAvailability::Unsupported
            && state.policy_pending.is_none()
            && state.source_pending.is_none();
        let receipt = receipt.clone();
        let request = SetUsageSourceLookup {
            source: source.id.clone(),
            policy: next,
            disclosure_version: source.disclosure.version,
        };
        let action = (source.availability != UsageSourceAvailability::Unsupported).then(|| {
            outline_button(
                vessel_app_kit::ui::SharedString::from(format!(
                    "usage-source-policy-{}",
                    source.id
                )),
                label,
                enabled,
                tokens,
            )
            .when(enabled, |button| {
                button.on_click(cx.listener(move |shell, _, _, cx| {
                    if let Some(receipt) = &receipt
                        && super::read_states::current(shell, receipt)
                    {
                        shell.usage_source_policy(receipt.machine().clone(), request.clone());
                        cx.notify();
                    }
                }))
            })
            .into_any_element()
        });
        let copy = source_disclosure(&source.disclosure, shell.words);
        body = body.child(
            settings::card(tokens)
                .child(settings::row_at(
                    source.label.as_str().to_owned(),
                    Some(copy.into()),
                    None,
                    action,
                    true,
                    compact,
                    tokens,
                ))
                .child(message(
                    source_availability(source.availability, shell.words),
                    tokens,
                )),
        );
    }
    body.into_any_element()
}
