//! Completed-tab protocol projection, not evidence of an actual engine tab.
use super::*;
use vessel_proto::browser::{
    BrowserHistory, BrowserIcon, BrowserLoad, BrowserProfile, BrowserUrl, BrowserViewport,
    NavigateArgs, NewTabArgs, ResizeArgs, TabArgs, ViewportSide, ViewportSize,
};
fn id() -> BrowserId {
    "00000000-0000-4000-8000-000000000001".parse().unwrap()
}
fn size(width: u32, height: u32) -> ViewportSize {
    ViewportSize {
        width: ViewportSide::try_from(width).unwrap(),
        height: ViewportSide::try_from(height).unwrap(),
    }
}
fn tab() -> BrowserTab {
    BrowserTab {
        browser_id: id(),
        workspace: vessel_proto::WorkspaceId::random(),
        url: "https://observed.example/redirected".parse().unwrap(),
        title: "Observed protocol fixture".to_owned().try_into().unwrap(),
        load: BrowserLoad::Ready,
        history: BrowserHistory::Empty,
        viewport: BrowserViewport::Fixed(size(1280, 800)),
        profile: BrowserProfile::General,
        icon: BrowserIcon::Globe,
    }
}
fn result(command: &BrowserCommand, tab: BrowserTab) -> Value {
    let reply = render(command, BrowserResult::Tab { tab }).unwrap();
    assert!(!reply.is_error.unwrap_or(false));
    reply.structured_content.unwrap()["result"].clone()
}
#[test]
fn create_and_navigate_are_flat_and_use_the_observed_url_not_the_request() {
    let requested: BrowserUrl = "https://requested.example/".parse().unwrap();
    let created = tab();
    let workspace = created.workspace;
    let output = result(
        &BrowserCommand::NewTab(NewTabArgs {
            url: Some(requested.clone()),
        }),
        created,
    );
    assert_eq!(output["browserId"], serde_json::to_value(id()).unwrap());
    assert_eq!(
        output["workspaceId"],
        serde_json::to_value(workspace).unwrap()
    );
    assert_eq!(output["url"], "https://observed.example/redirected");
    assert_eq!(output["command"], "new_tab");
    assert!(output.get("tab").is_none());
    let output = result(
        &BrowserCommand::Navigate(NavigateArgs {
            browser_id: id(),
            url: requested,
        }),
        tab(),
    );
    assert_eq!(output["url"], "https://observed.example/redirected");
    assert_eq!(output["command"], "navigate");
    assert!(output.get("tab").is_none());
}
#[test]
fn resize_returns_observed_dimensions_for_both_resident_viewport_modes() {
    for viewport in [
        BrowserViewport::Fixed(size(1280, 800)),
        BrowserViewport::Responsive(size(1280, 800)),
    ] {
        let mut completed = tab();
        completed.viewport = viewport;
        let output = result(
            &BrowserCommand::Resize(ResizeArgs {
                browser_id: id(),
                size: size(390, 844),
            }),
            completed,
        );
        assert_eq!(output["width"], 1280);
        assert_eq!(output["height"], 800);
        assert_eq!(output["command"], "resize");
        assert!(output.get("tab").is_none());
    }
}
#[test]
fn traversal_and_reload_project_only_the_completed_target_identity() {
    for command in [
        BrowserCommand::Back(TabArgs { browser_id: id() }),
        BrowserCommand::Forward(TabArgs { browser_id: id() }),
        BrowserCommand::Reload(TabArgs { browser_id: id() }),
    ] {
        let output = result(&command, tab());
        assert_eq!(output["browserId"], serde_json::to_value(id()).unwrap());
        assert_eq!(
            output["command"],
            command.tool().name().strip_prefix("browser_").unwrap()
        );
        assert_eq!(output.as_object().unwrap().len(), 2);
    }
}
