use super::*;
use crate::browser::DevicePreset;
use vessel_proto::browser::{BrowserFailure, BrowserIcon, BrowserId, BrowserViewport, PageText};
use vessel_proto::{SessionId, WorkspaceId};

fn viewport(width: u32, height: u32) -> ViewportSize {
    ViewportSize {
        width: ViewportSide::try_from(width).unwrap(),
        height: ViewportSide::try_from(height).unwrap(),
    }
}

fn tab() -> BrowserTab {
    BrowserTab {
        browser_id: BrowserId::random(),
        workspace: WorkspaceId::random(),
        url: "https://fixture.test/".parse().unwrap(),
        title: PageText::try_from("Local fixture".to_owned()).unwrap(),
        load: BrowserLoad::Ready,
        history: BrowserHistory::Empty,
        viewport: BrowserViewport::Responsive(viewport(390, 844)),
        profile: BrowserProfile::General,
        icon: BrowserIcon::Globe,
    }
}

#[test]
fn browser_controls_use_the_observed_target_and_loading_state() {
    let mut observed = tab();
    let mut pane = GeneralPane::new(observed.clone(), BrowserRevision::ZERO).unwrap();
    assert!(pane.navigation_request(NavigationAction::Back).is_none());
    assert!(pane.navigation_request(NavigationAction::Stop).is_none());

    observed.load = BrowserLoad::Loading;
    observed.history = BrowserHistory::Back;
    assert!(pane.observe(observed.clone(), BrowserRevision::ZERO.next().unwrap()));
    assert!(pane.navigation_request(NavigationAction::Reload).is_none());
    let stop = pane.navigation_request(NavigationAction::Stop).unwrap();
    assert_eq!(stop.workspace, observed.workspace);
    assert!(stop.opening_session.is_none());
    assert_eq!(
        stop.command,
        BrowserControlCommand::Stop {
            browser_id: observed.browser_id.clone(),
        }
    );
}

#[test]
fn late_or_foreign_descriptors_cannot_replace_a_resident_tab() {
    let observed = tab();
    let revision = BrowserRevision::ZERO.next().unwrap();
    let mut pane = GeneralPane::new(observed.clone(), revision).unwrap();
    let mut foreign = observed.clone();
    foreign.workspace = WorkspaceId::random();
    assert!(!pane.observe(foreign, revision));
    let mut old = observed.clone();
    old.load = BrowserLoad::Failed(BrowserFailure::Closed);
    assert!(!pane.observe(old, BrowserRevision::ZERO));
    let mut preview = observed.clone();
    preview.profile = BrowserProfile::Preview {
        opening_session: SessionId::random(),
    };
    assert!(GeneralPane::new(preview.clone(), revision).is_none());
    assert!(!pane.observe(preview, revision));
    assert_eq!(pane.tab(), &observed);
    assert_eq!(pane.revision(), revision);
}

#[test]
fn addresses_and_device_sizes_produce_only_typed_general_commands() {
    let pane = GeneralPane::new(tab(), BrowserRevision::ZERO).unwrap();
    assert!(pane.address_request("mailto:fixture@example.test").is_err());
    let control = pane.address_request("fixture.test/a").unwrap();
    let BrowserControlCommand::Drive(BrowserCommand::Navigate(args)) = control.command else {
        panic!("expected typed navigation");
    };
    assert_eq!(args.url.as_str(), "https://fixture.test/a");

    let current_bounds = viewport(618, 578);
    for (choice, expected) in [
        (Viewport::Responsive, current_bounds),
        (Viewport::Device(DevicePreset::Iphone14), viewport(390, 844)),
    ] {
        let request = pane.resize_request(choice, current_bounds).unwrap();
        let BrowserControlCommand::Drive(BrowserCommand::Resize(args)) = request.command else {
            panic!("expected typed viewport");
        };
        assert_eq!(args.size, expected);
    }
}
