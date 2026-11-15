use super::*;
use gpui::TestAppContext;
use vessel_app_kit::{Scheme, SchemeChoice};
use vessel_app_view::plugins::{HostSupport, SnapshotApplied, Submission};
use vessel_proto::plugins::{
    Action, Availability, Button, ButtonBehavior, ClientCatalog, ClientDocument,
    ClientInstallation, Contribution, ContributionContent, ContributionKind, Control,
    ControlContent, PluginAddress, RuntimeGeneration, Surface,
};

#[gpui::test]
fn markdown_links_use_the_typed_guard_and_recording_fake_opener(cx: &mut TestAppContext) {
    use gpui::{Modifiers, point, px};
    use std::sync::Mutex;
    cx.update(|cx| {
        vessel_app_kit::init(cx, SchemeChoice::Fixed(Scheme::Dark)).expect("kit");
    });
    for (markdown, accepted) in [
        ("[Open](https://example.invalid/page)", true),
        (
            "[Open](https://user:PRIVATE-LINK-CANARY@example.invalid/page)",
            false,
        ),
    ] {
        let (registry, address) = fixture_with_markdown(Some(markdown));
        let owner = Rc::clone(&registry);
        let opened = Arc::new(Mutex::new(Vec::new()));
        let fake = Arc::clone(&opened);
        let events = Rc::new(RefCell::new(Vec::new()));
        let observed = Rc::clone(&events);
        let (_view, cx) = cx.add_window_view(move |window, cx| {
            let view = SurfaceView::new(
                &owner,
                address,
                Arc::new(move |url, _, _| fake.lock().expect("fake opener").push(url.clone())),
                window,
                cx,
            )
            .expect("surface");
            cx.subscribe_self(move |_view, event: &SurfaceEvent, _cx| {
                observed.borrow_mut().push(event.clone())
            })
            .detach();
            view
        });
        cx.run_until_parked();
        let bounds = cx
            .debug_bounds("plugin-surface-markdown")
            .expect("real Markdown body");
        let inset = px(vessel_app_kit::theme::spacing::S2);
        cx.simulate_click(
            point(bounds.left() + inset, bounds.top() + inset),
            Modifiers::none(),
        );
        cx.run_until_parked();
        if accepted {
            let urls = opened.lock().expect("recorded opener");
            assert_eq!(urls.len(), 1);
            assert_eq!(urls[0].as_str(), "https://example.invalid/page");
        } else {
            assert!(opened.lock().expect("recorded opener").is_empty());
            assert!(
                events
                    .borrow()
                    .iter()
                    .any(|event| matches!(event, SurfaceEvent::LinkRejected))
            );
        }
    }
}
use vessel_proto::{NodeId, RawJson};

fn fixture() -> (Rc<RefCell<Registry>>, ContributionAddress) {
    fixture_with_markdown(None)
}

#[gpui::test]
fn presented_button_dispatch_rechecks_offline_owner_before_any_request_event(
    cx: &mut TestAppContext,
) {
    use gpui::{Modifiers, point, px};
    cx.update(|cx| {
        vessel_app_kit::init(cx, SchemeChoice::Fixed(Scheme::Dark)).expect("kit");
    });
    let (registry, address) = fixture();
    let node = address.installation.node;
    let owner = Rc::clone(&registry);
    let captured = Rc::new(RefCell::new(Vec::new()));
    let events = Rc::clone(&captured);
    let (view, cx) = cx.add_window_view(move |window, cx| {
        let view = SurfaceView::new(
            &owner,
            address,
            Arc::new(|_, _, _| panic!("no browser in fixture")),
            window,
            cx,
        )
        .expect("surface");
        cx.subscribe_self(move |_view, event: &SurfaceEvent, _cx| {
            events.borrow_mut().push(event.clone())
        })
        .detach();
        view
    });
    cx.run_until_parked();
    let bounds = cx
        .debug_bounds("plugin-surface-button")
        .expect("real kit button");
    let inset = px(vessel_app_kit::theme::spacing::S2);
    let press = point(bounds.left() + inset, bounds.top() + inset);
    cx.simulate_click(press, Modifiers::none());
    cx.run_until_parked();
    assert!(captured.borrow().iter().any(|event| matches!(event, SurfaceEvent::Requested(request) if matches!(request.submission(), Submission::Action(_)))));
    captured.borrow_mut().clear();
    // Invoke the same captured dispatch without letting toolkit hover/pressed
    // invalidation repaint it disabled first. The earlier real click proves
    // this is the handler wired to the actual kit button.
    registry.borrow_mut().offline(node);
    view.update_in(cx, |view, window, cx| {
        let control: ContributionId = "button".parse().expect("button");
        let selected = view
            .state
            .surface()
            .controls
            .iter()
            .find(|item| item.id == control)
            .expect("declared button");
        let ControlContent::Button { button } = &selected.content else {
            panic!("button");
        };
        let behavior = button.behavior.clone();
        view.press(&control, &behavior, press, window, cx);
    });
    cx.run_until_parked();
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
fn fixture_with_markdown(markdown: Option<&str>) -> (Rc<RefCell<Registry>>, ContributionAddress) {
    let installation = PluginAddress {
        node: NodeId::random(),
        plugin: "fixture".parse().expect("id"),
        generation: RuntimeGeneration::random(),
    };
    let address = ContributionAddress {
        installation: installation.clone(),
        kind: ContributionKind::Screen,
        contribution: "home".parse().expect("id"),
    };
    let mut controls = vec![
        Control {
            id: "text".parse().expect("id"),
            content: ControlContent::Text {
                text: "Hello 안녕하세요".parse().expect("text"),
            },
        },
        Control {
            id: "button".parse().expect("id"),
            content: ControlContent::Button {
                button: Button {
                    title: "Run".parse().expect("title"),
                    availability: Availability::Enabled,
                    behavior: ButtonBehavior::Action {
                        action: Action::Invoke {
                            method: "act".parse().expect("method"),
                            input: RawJson::from_text("{}").expect("json"),
                        },
                    },
                },
            },
        },
        Control {
            id: "field".parse().expect("id"),
            content: ControlContent::TextField {
                label: "Input".parse().expect("label"),
                submit: "act".parse().expect("method"),
            },
        },
    ];
    if let Some(markdown) = markdown {
        controls.insert(
            0,
            Control {
                id: "markdown".parse().expect("id"),
                content: ControlContent::Markdown {
                    text: markdown.parse().expect("bounded Markdown"),
                },
            },
        );
    }
    let document = ClientDocument::new(
        vec!["act".parse().expect("method")],
        vec![Contribution {
            id: address.contribution.clone(),
            title: "Home".parse().expect("title"),
            content: ContributionContent::Screen {
                surface: Surface { controls },
            },
        }],
        vec![],
    )
    .expect("graph");
    let mut registry = Registry::default();
    let ticket = registry
        .begin(installation.node, HostSupport::Supported)
        .expect("begin");
    let entry = ClientInstallation::new(installation, vessel_proto::PROTOCOL_VERSION, document)
        .expect("canonical installation");
    let catalog = ClientCatalog::new(ticket.node(), vec![entry]).expect("canonical catalog");
    assert_eq!(
        registry.apply(ticket, Ok(catalog)),
        Ok(SnapshotApplied::Applied)
    );
    (Rc::new(RefCell::new(registry)), address)
}

#[gpui::test]
fn real_native_field_captures_bounded_text_and_refuses_submit_after_owner_goes_offline(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| {
        vessel_app_kit::init(cx, SchemeChoice::Fixed(Scheme::Dark)).expect("kit");
    });
    let (registry, address) = fixture();
    let owner = Rc::clone(&registry);
    let captured = Rc::new(RefCell::new(Vec::new()));
    let events = Rc::clone(&captured);
    let (view, cx) = cx.add_window_view(move |window, cx| {
        let view = SurfaceView::new(
            &owner,
            address,
            Arc::new(|_, _, _| panic!("no browser in fixture")),
            window,
            cx,
        )
        .expect("surface");
        cx.subscribe_self(move |_view, event: &SurfaceEvent, _cx| {
            events.borrow_mut().push(event.clone())
        })
        .detach();
        view
    });
    cx.run_until_parked();
    let id: ContributionId = "field".parse().expect("field");
    view.update_in(cx, |view, window, cx| {
        let field = view.fields.get(&id).expect("real field").clone();
        field.update(cx, |field, cx| {
            field.editor.set_text("retained literal", window, cx);
            field.editor.focus(window, cx);
        });
    });
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert!(captured.borrow().iter().any(|event| matches!(event, SurfaceEvent::Requested(request) if matches!(request.submission(), Submission::Field { value: FieldValue::Text(text), .. } if text.as_str() == "retained literal"))));
    registry
        .borrow_mut()
        .offline(view.read_with(cx, |view, _| view.state.address().installation.node));
    captured.borrow_mut().clear();
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
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

#[gpui::test]
fn a_replaced_editor_cannot_submit_a_queued_event_to_the_same_control_identity(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| {
        vessel_app_kit::init(cx, SchemeChoice::Fixed(Scheme::Dark)).expect("kit");
    });
    let (registry, address) = fixture();
    let owner = Rc::clone(&registry);
    let captured = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&captured);
    let (view, cx) = cx.add_window_view(move |window, cx| {
        let view = SurfaceView::new(
            &owner,
            address,
            Arc::new(|_, _, _| panic!("no browser in fixture")),
            window,
            cx,
        )
        .expect("surface");
        cx.subscribe_self(move |_view, event: &SurfaceEvent, _cx| {
            observed.borrow_mut().push(event.clone())
        })
        .detach();
        view
    });
    let id: ContributionId = "field".parse().expect("control");
    view.update_in(cx, |view, window, cx| {
        let old = view.fields.get(&id).expect("old actual editor").clone();
        let current = cx.new(|cx| {
            fields::Field::new(id.clone(), "Input".parse().expect("caption"), window, cx)
        });
        view._subscriptions.push(cx.subscribe_in(
            &current,
            window,
            |view, _field, event, window, cx| view.field_event(event, window, cx),
        ));
        let previous = view.fields.insert(id.clone(), current);
        assert!(previous.is_some());
        // Keep the real old entity/subscription alive and deliver its queued
        // submission through EventEmitter after its map slot has been replaced.
        old.update(cx, |field, cx| {
            cx.emit(fields::Event::Submit {
                identity: id.clone(),
                source: field.editor.id(),
                value: "old queued literal".parse(),
            });
        });
        cx.notify();
    });
    cx.run_until_parked();
    assert!(captured.borrow().iter().any(|event| matches!(
        event,
        SurfaceEvent::Rejected(SurfaceProblem::Catalog(Problem::Stale))
    )));
    assert!(
        !captured
            .borrow()
            .iter()
            .any(|event| matches!(event, SurfaceEvent::Requested(_)))
    );
    view.read_with(cx, |view, _| {
        let Some(FieldValue::Text(text)) = view.state.value(&id) else {
            panic!("retained text");
        };
        assert_eq!(text.as_str(), "");
    });
    captured.borrow_mut().clear();
    view.update_in(cx, |view, window, cx| {
        view.fields
            .get(&id)
            .expect("current actual editor")
            .clone()
            .update(cx, |field, cx| {
                field.editor.set_text("current literal", window, cx);
                field.editor.focus(window, cx);
            });
    });
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert!(captured.borrow().iter().any(|event| matches!(
        event,
        SurfaceEvent::Requested(request) if matches!(request.submission(),
            Submission::Field { value: FieldValue::Text(text), .. } if text.as_str() == "current literal")
    )));
}
