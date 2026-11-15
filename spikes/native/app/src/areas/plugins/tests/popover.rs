use super::*;
use vessel_app_kit::popover::Dismissal;

fn popup_fixture() -> (Rc<RefCell<Registry>>, ContributionAddress) {
    fixture_controls(vec![Control {
        id: "trigger".parse().expect("identity"),
        content: ControlContent::Button {
            button: Button {
                title: "Form".parse().expect("caption"),
                availability: Availability::Enabled,
                behavior: ButtonBehavior::Popover {
                    surface: Surface {
                        controls: vec![
                            Control {
                                id: "link".parse().expect("identity"),
                                content: ControlContent::Markdown {
                                    text: "[Open](https://example.invalid/page)"
                                        .parse()
                                        .expect("Markdown"),
                                },
                            },
                            Control {
                                id: "inside".parse().expect("identity"),
                                content: ControlContent::TextField {
                                    label: "Input".parse().expect("caption"),
                                    submit: "act".parse().expect("method"),
                                },
                            },
                        ],
                    },
                },
            },
        },
    }])
}

#[gpui::test]
fn each_actual_popup_opening_retires_its_body_and_recreates_the_editor(cx: &mut TestAppContext) {
    use gpui::{Modifiers, px};
    cx.update(|cx| {
        vessel_app_kit::init(cx, SchemeChoice::Fixed(Scheme::Dark)).expect("kit");
    });
    let (registry, address) = popup_fixture();
    let owner = Rc::clone(&registry);
    let captured = Rc::new(RefCell::new(Vec::new()));
    let events = Rc::clone(&captured);
    let opened = Arc::new(std::sync::Mutex::new(Vec::new()));
    let fake = Arc::clone(&opened);
    let (view, cx) = cx.add_window_view(move |window, cx| {
        let view = SurfaceView::new(
            &owner,
            address,
            Arc::new(move |url, _, _| fake.lock().expect("fake opener").push(url.clone())),
            window,
            cx,
        )
        .expect("surface");
        cx.subscribe_self(move |_view, event: &SurfaceEvent, _cx| {
            events.borrow_mut().push(event.clone());
        })
        .detach();
        view
    });
    cx.run_until_parked();
    let trigger = cx
        .debug_bounds("plugin-surface-button")
        .expect("actual trigger");
    cx.simulate_click(
        trigger.inset(px(vessel_app_kit::theme::spacing::S2)).origin,
        Modifiers::none(),
    );
    cx.run_until_parked();
    let (old, lease) = view.read_with(cx, |view, _| {
        let opening = view
            .shown_popover
            .as_ref()
            .expect("actual shared popover opened");
        (opening.body.clone(), opening.lease.clone())
    });
    let handler = assert_live_and_offline_link(&registry, &old, &opened, &captured, cx);
    let inside: ContributionId = "inside".parse().expect("identity");
    let old_id = old.read_with(cx, |view, cx| view.fields[&inside].read(cx).editor.id());
    view.update_in(cx, |view, window, cx| {
        assert_eq!(view.retire_popover(&lease), Dismissal::Close);
        assert!(view.popover.close(&lease, window, cx));
        cx.notify();
    });
    cx.run_until_parked();
    cx.simulate_click(
        trigger.inset(px(vessel_app_kit::theme::spacing::S2)).origin,
        Modifiers::none(),
    );
    cx.run_until_parked();
    let fresh = view.read_with(cx, |view, _| {
        view.shown_popover
            .as_ref()
            .expect("second opening")
            .body
            .clone()
    });
    let fresh_id = fresh.read_with(cx, |view, cx| view.fields[&inside].read(cx).editor.id());
    assert_ne!(
        fresh_id, old_id,
        "a reopened control cannot inherit an old emitter identity"
    );
    assert_ne!(old.entity_id(), fresh.entity_id());
    assert_retired_input(&old, &inside, &handler, &opened, &captured, cx);
    view.update_in(cx, |view, window, cx| {
        assert_eq!(view.retire_popover(&lease), Dismissal::KeepOpen);
        assert!(
            !view.popover.close(&lease, window, cx),
            "old close cannot acknowledge new focus"
        );
    });
    fresh.update_in(cx, |view, window, cx| {
        view.fields[&inside].clone().update(cx, |field, cx| {
            field.editor.set_text("fresh literal", window, cx);
            field.editor.focus(window, cx);
        });
    });
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert!(captured.borrow().iter().any(|event| matches!(event, SurfaceEvent::Requested(request) if matches!(request.submission(), Submission::Field { value: FieldValue::Text(value), .. } if value.as_str() == "fresh literal"))));
    assert_eq!(
        view.read_with(cx, |view, _| view.subscriptions.len()),
        0,
        "opening subscriptions are held only by the current owner"
    );
    captured.borrow_mut().clear();
    view.update_in(cx, |view, window, cx| {
        view.queue_popover(
            "trigger".parse().expect("identity"),
            vec![],
            Some(trigger),
            window,
            cx,
        );
        // Hold the invalidation before the deferred open's first owner check.
        registry
            .borrow_mut()
            .offline(view.state.address().installation.node);
    });
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert!(view.shown_popover.is_none());
        assert!(!view.popover.is_open());
    });
    assert!(captured.borrow().iter().any(|event| matches!(
        event,
        SurfaceEvent::Rejected(SurfaceProblem::Catalog(Problem::Offline))
    )));
    assert!(
        !captured
            .borrow()
            .iter()
            .any(|event| matches!(event, SurfaceEvent::Requested(_)))
    );
}

type Opened = Arc<std::sync::Mutex<Vec<AttachmentUrl>>>;
type Events = Rc<RefCell<Vec<SurfaceEvent>>>;

fn follow(handler: &vessel_app_kit::MarkdownLinkHandler, cx: &mut gpui::VisualTestContext) {
    let url: gpui::SharedString = "https://example.invalid/page".into();
    let click = gpui::ClickEvent::Mouse(gpui::MouseClickEvent::default());
    cx.update(|window, cx| handler(&url, &click, window, cx));
}

fn assert_live_and_offline_link(
    registry: &Rc<RefCell<Registry>>,
    old: &Entity<SurfaceView>,
    opened: &Opened,
    captured: &Events,
    cx: &mut gpui::VisualTestContext,
) -> vessel_app_kit::MarkdownLinkHandler {
    assert!(
        cx.debug_bounds("plugin-surface-markdown").is_some(),
        "actual popup Markdown rendered"
    );
    // Clone the exact Arc which the actual rendered Markdown passed to kit.
    // Direct invocation intentionally reaches it beyond the UI input fence.
    let handler = old.read_with(cx, |view, _| Arc::clone(&view.markdown_link));
    follow(&handler, cx);
    assert_eq!(
        opened.lock().expect("fake opener").len(),
        1,
        "live positive control"
    );
    opened.lock().expect("fake opener").clear();
    let node = old.read_with(cx, |view, _| view.state.address().installation.node);
    registry.borrow_mut().offline(node);
    follow(&handler, cx);
    cx.run_until_parked();
    assert!(opened.lock().expect("fake opener").is_empty());
    assert!(captured.borrow().iter().any(|event| matches!(
        event,
        SurfaceEvent::Rejected(SurfaceProblem::Catalog(Problem::Offline))
    )));
    let entry = registry.borrow().installed(node).expect("retained catalog")[0].clone();
    let snapshot = ClientCatalog::new(
        node,
        vec![
            ClientInstallation::new(
                entry.address().clone(),
                vessel_proto::PROTOCOL_VERSION,
                entry.document().clone(),
            )
            .expect("canonical installation"),
        ],
    )
    .expect("canonical snapshot");
    let ticket = registry
        .borrow_mut()
        .begin(node, HostSupport::Supported)
        .expect("reconnect");
    assert_eq!(
        registry.borrow_mut().apply(ticket, Ok(snapshot)),
        Ok(SnapshotApplied::Applied)
    );
    handler
}

fn assert_retired_input(
    old: &Entity<SurfaceView>,
    inside: &ContributionId,
    handler: &vessel_app_kit::MarkdownLinkHandler,
    opened: &Opened,
    captured: &Events,
    cx: &mut gpui::VisualTestContext,
) {
    let old_events = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&old_events);
    old.update_in(cx, |old, _window, cx| {
        cx.subscribe_self(move |_view, event: &SurfaceEvent, _cx| {
            observed.borrow_mut().push(event.clone());
        })
        .detach();
        let field = old.fields[&inside].clone();
        field.update(cx, |field, cx| {
            cx.emit(fields::Event::Submit {
                identity: inside.clone(),
                source: field.editor.id(),
                value: "retired queued input".parse(),
            });
        });
        // Retained old entities can exist externally, but cannot forward input.
        assert!(!old.inputs.is_live());
    });
    // The same pre-close Markdown handler must refuse after ancestor retirement,
    // even though its old entity/Arc are deliberately retained and reachable.
    follow(handler, cx);
    cx.run_until_parked();
    assert!(old_events.borrow().iter().any(|event| matches!(
        event,
        SurfaceEvent::Rejected(SurfaceProblem::Catalog(Problem::Stale))
    )));
    assert!(opened.lock().expect("fake opener").is_empty());
    assert!(
        !captured
            .borrow()
            .iter()
            .any(|event| matches!(event, SurfaceEvent::Requested(_)))
    );
}
