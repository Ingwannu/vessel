//! Staged native surface renderer against one injected owner registry.
//! The node-backed Hub/panel wiring removes the parent cfg(test), retains that
//! ONE registry, and consumes typed request events through the real link.
mod draw;
mod fields;
mod menus;
#[cfg(test)]
mod tests;

use gpui::{AppContext as _, EventEmitter, Render, StatefulInteractiveElement as _};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::{Rc, Weak},
    sync::Arc,
};
use vessel_app_kit::{
    MenuState,
    ui::{
        App, Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _,
        Styled as _, Subscription, Window, div,
    },
};
use vessel_app_view::plugins::{
    FieldValue, Problem, Registry, SurfaceProblem, SurfaceRequest, SurfaceState,
};
use vessel_proto::plugins::{AttachmentUrl, ContributionAddress, ContributionId};

/// The owner supplies its core opener; tests always inject a recording fake.
type Opener = Arc<dyn Fn(&AttachmentUrl, &mut Window, &mut App) + Send + Sync>;

#[derive(Debug, Clone)]
enum SurfaceEvent {
    Requested(SurfaceRequest),
    Rejected(SurfaceProblem),
    LinkRejected,
}

struct SurfaceView {
    state: SurfaceState,
    registry: Weak<RefCell<Registry>>,
    fields: BTreeMap<ContributionId, Entity<fields::Field>>,
    _subscriptions: Vec<Subscription>,
    menu: MenuState,
    shown_menu: Option<menus::Shown>,
    opener: Opener,
}
impl EventEmitter<SurfaceEvent> for SurfaceView {}

impl SurfaceView {
    fn new(
        registry: &Rc<RefCell<Registry>>,
        address: ContributionAddress,
        opener: Opener,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Result<Self, SurfaceProblem> {
        let borrowed = registry.try_borrow().map_err(|_busy| Problem::Busy)?;
        let state = SurfaceState::new(&borrowed, address)?;
        drop(borrowed);
        let mut field_specs = Vec::new();
        fields::collect(state.surface(), &mut field_specs);
        let mut fields = BTreeMap::new();
        let mut subscriptions = Vec::new();
        for (identity, label) in field_specs {
            let field = cx.new(|cx| fields::Field::new(identity.clone(), label, window, cx));
            subscriptions.push(cx.subscribe_in(
                &field,
                window,
                |this, _field, event, window, cx| {
                    this.field_event(event, window, cx);
                },
            ));
            let _old = fields.insert(identity, field);
        }
        Ok(Self {
            state,
            registry: Rc::downgrade(registry),
            fields,
            _subscriptions: subscriptions,
            menu: MenuState::new(cx),
            shown_menu: None,
            opener,
        })
    }

    fn with_registry<T>(
        &self,
        work: impl FnOnce(&Registry) -> Result<T, SurfaceProblem>,
    ) -> Result<T, SurfaceProblem> {
        let registry = self.registry.upgrade().ok_or(Problem::Stale)?;
        let registry = registry.try_borrow().map_err(|_busy| Problem::Busy)?;
        work(&registry)
    }

    fn field_event(
        &mut self,
        event: &fields::Event,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let (identity, value, submit) = match event {
            fields::Event::Edit { identity, value } => (identity, value, false),
            fields::Event::Submit { identity, value } => (identity, value, true),
        };
        let result = value
            .as_ref()
            .map_err(|_invalid| SurfaceProblem::Value)
            .and_then(|value| self.state.edit(identity, FieldValue::Text(value.clone())));
        if let Err(problem) = result {
            // The actual editor must agree with its bounded retained draft.
            if let (Some(field), Some(FieldValue::Text(old))) =
                (self.fields.get(identity), self.state.value(identity))
            {
                field.update(cx, |field, cx| {
                    field.editor.set_text(old.as_str(), window, cx)
                });
            }
            cx.emit(SurfaceEvent::Rejected(problem));
        } else if submit {
            self.submit(identity, cx);
        }
        cx.notify();
    }

    fn submit(&self, identity: &ContributionId, cx: &mut Context<'_, Self>) {
        let request = self.with_registry(|registry| self.state.submit(registry, identity));
        self.publish(request, cx);
    }

    fn change(&mut self, identity: &ContributionId, value: FieldValue, cx: &mut Context<'_, Self>) {
        match self.state.edit(identity, value) {
            Ok(()) => self.submit(identity, cx),
            Err(problem) => cx.emit(SurfaceEvent::Rejected(problem)),
        }
        cx.notify();
    }

    fn publish(&self, request: Result<SurfaceRequest, SurfaceProblem>, cx: &mut Context<'_, Self>) {
        let checked = request.and_then(|request| {
            self.with_registry(|registry| self.state.revalidate(registry, &request))?;
            Ok(request)
        });
        match checked {
            Ok(request) => cx.emit(SurfaceEvent::Requested(request)),
            Err(problem) => cx.emit(SurfaceEvent::Rejected(problem)),
        }
    }
}

impl Render for SurfaceView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = vessel_app_kit::tokens(cx);
        let live = self
            .with_registry(|registry| self.state.check_current(registry))
            .is_ok();
        let body = draw::surface(self, self.state.surface(), &tokens, live, window, cx);
        let overlay = menus::overlay(self, &tokens, window, cx);
        div()
            .id("plugin-surface-root")
            .size_full()
            .min_w_0()
            .min_h_0()
            .overflow_y_scroll()
            .child(body)
            .children(overlay)
    }
}
