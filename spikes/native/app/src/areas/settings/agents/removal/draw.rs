//! All menu and modal furniture comes from the reviewed shared controls.
use super::{Context, HarnessRef, Rc, Shell, Window};
use vessel_app_kit::ui::{
    AnyElement, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, div, px,
};
use vessel_app_kit::{
    ButtonConfig, ButtonContent, ButtonKind, ButtonState, ControlSizing, DialogContent, DialogSize,
    Tokens, assets::Glyph, button_control, menu as menus,
};
use vessel_app_view::agent_status::removal::{can_remove, is_removing};
const MENU_WIDTH: f32 = 220.0;
pub(in crate::areas::settings::agents) fn trigger(
    agent: &HarnessRef,
    inventory: &vessel_app_view::agent_status::pages::Inventory,
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    if !can_remove(agent, inventory) {
        return None;
    }
    let machine = shell.areas.settings.agents.machine.clone()?;
    let state = &shell.areas.settings.agents.removal;
    let key = vessel_app_view::agent_status::agent_key(agent);
    let anchor = Rc::clone(state.anchors.borrow_mut().entry(key.clone()).or_default());
    let node = inventory.node;
    let target = agent.clone();
    let pending = is_removing(
        agent,
        super::super::page(shell)
            .and_then(|page| page.pending())
            .map(|pending| &pending.operation),
    );
    let availability = if pending {
        ButtonState::Disabled
    } else {
        ButtonState::Enabled
    };
    let open = state.menu.is_open()
        && state.target.as_ref().is_some_and(|target| {
            target.machine == machine && target.node == node && target.agent == *agent
        });
    Some(
        menus::trigger_presented(
            format!("agent-actions-{key}"),
            &anchor,
            menus::TriggerProfile::ProviderRow,
            ControlSizing::from_compact(compact),
            if open {
                menus::TriggerPresentation::Open(availability)
            } else {
                menus::TriggerPresentation::Closed(availability)
            },
            tokens,
        )
        .when(!pending, |button| {
            button.on_click(cx.listener(move |shell, _, window, cx| {
                shell.agent_open_actions(&machine, node, &target, &anchor, window, cx);
            }))
        })
        .into_any_element(),
    )
}
pub(in crate::areas::settings::agents) fn menu(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.settings.agents.removal;
    let target = state.target.as_ref()?;
    if !shell.agent_context(Some(&target.machine)) {
        return None;
    }
    let origin = Rc::clone(target);
    let words = shell.words.agent_catalog();
    let items = [menus::MenuItem::danger(words.remove())
        .with_leading_at(Glyph::Trash2, menus::LeadingSize::Regular)];
    menus::menu_with(
        menus::Content::new("agent-actions-menu", &items).with_width(px(MENU_WIDTH)),
        &state.menu,
        &menus::MenuHandlers {
            state: |shell: &mut Shell| &mut shell.areas.settings.agents.removal.menu,
            chose: Rc::new(move |shell, index, window, cx| {
                if index == 0 {
                    shell.agent_choose_removal(&origin, window, cx);
                }
            }),
        },
        tokens,
        compact,
        cx,
    )
}
pub(in crate::areas::settings::agents) fn dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.settings.agents.removal;
    if state.restore.replace(false) {
        state.dialog.dismissed(window, cx);
    }
    let target = state.confirmation.as_ref()?;
    if !shell.agent_context(Some(&target.machine)) {
        return None;
    }
    let words = shell.words.agent_catalog();
    let compact = vessel_app_kit::breakpoint::phone(window);
    let cancel = Rc::clone(target);
    let confirm = Rc::clone(target);
    let close = Rc::clone(target);
    let sizing = ControlSizing::from_compact(compact);
    let footer = vec![
        button_control(
            "agent-remove-cancel",
            ButtonContent::Label(words.cancel().into()),
            ButtonConfig::new(ButtonKind::Ghost, ButtonState::Enabled, sizing),
            tokens,
        )
        .on_click(cx.listener(move |shell, _, window, cx| {
            shell.agent_cancel_removal(&cancel, window, cx);
        }))
        .into_any_element(),
        button_control(
            "agent-remove-confirm",
            ButtonContent::Label(words.remove_confirm().into()),
            ButtonConfig::new(ButtonKind::Destructive, ButtonState::Enabled, sizing),
            tokens,
        )
        .on_click(cx.listener(move |shell, _, window, cx| {
            shell.agent_confirm_removal(&confirm, window, cx);
        }))
        .into_any_element(),
    ];
    Some(vessel_app_kit::dialog(
        &state.dialog,
        DialogContent {
            title: target.title.borrow().clone(),
            body: div().child(words.remove_help()).into_any_element(),
            footer,
            size: DialogSize::Normal,
        },
        tokens,
        window,
        compact,
        move |shell: &mut Shell, window, cx| shell.agent_cancel_removal(&close, window, cx),
        cx,
    ))
}
pub(in crate::areas::settings::agents) fn notice(
    shell: &Shell,
    tokens: &Tokens,
) -> Option<AnyElement> {
    let state = &shell.areas.settings.agents.removal;
    let words = shell.words.agent_catalog();
    let notice =
        vessel_app_view::agent_status::removal::notice(state.problem, state.word_error.as_ref())?;
    let message = words.removal_notice(notice);
    Some(
        vessel_app_kit::alert::alert(
            vessel_app_kit::alert::Content {
                title: Some(message.into()),
                description: None,
                action: None,
            },
            crate::badge::alert_variant(notice.variant()),
            vessel_app_kit::alert::Size::Small,
            tokens,
        )
        .into_any_element(),
    )
}
