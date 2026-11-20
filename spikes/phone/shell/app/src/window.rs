//! The one web view: the page at the bridge's origin, kept there, and the
//! build's label shown on it.
//!
//! [Decision Log]
//! - Purpose: the web view shows the page and nothing else; no remote
//!   content is ever loaded into it (the brief), since the page holds the
//!   way to every node the phone has paired with.
//! - What keeps it so: (1) navigation is refused for any address but the
//!   bridge's own origin (scheme, the launch's host name and port), so a
//!   link in a conversation cannot take the web view elsewhere; (2) the
//!   page's content security policy, sent by the bridge with every
//!   response, keeps scripts, styles, images and connections on that
//!   origin; (3) the web view is granted no Tauri command (no capability in
//!   `tauri.conf.json`), so a page that went wrong still could not call
//!   into the app.
//! - Alternatives: opening links in the system browser (needs Tauri's
//!   opener plugin and a permission for it; left for when the owner asks,
//!   so today a link to elsewhere does nothing); Tauri's own asset
//!   protocol for the page (the page would then need a second transport
//!   to reach Rust, through IPC commands: the surface the brief asks to
//!   keep empty).
//! - The label: shown for a few seconds over the page when it loads, as a
//!   pill that takes no touches, in Pretendard (the owner's rule on type),
//!   so a capture or a report names the build. Injected by the web view
//!   (an initialisation script), not by the page, which other tracks hold.

use std::sync::{Arc, Mutex};

use tauri::webview::PageLoadEvent;
use tauri::{App, Url, WebviewUrl, WebviewWindowBuilder};

/// The window's label in Tauri.
const MAIN: &str = "main";

/// The origin the web view may show: the bridge's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Origin {
    host: String,
    port: u16,
}

impl Origin {
    /// The origin of `url` (the launch address), if it is the bridge's
    /// kind: `http`, a host name, a port.
    pub(crate) fn of(url: &Url) -> Option<Self> {
        if url.scheme() != "http" {
            return None;
        }
        Some(Self {
            host: url.host_str()?.to_owned(),
            port: url.port()?,
        })
    }

    /// Whether the web view may go to `url`.
    pub(crate) fn allows(&self, url: &Url) -> bool {
        Self::of(url).is_some_and(|origin| origin == *self)
    }
}

/// Whether `url` is the app's own start page (Tauri serves the APK's
/// `dist/` at `http://tauri.localhost/` on Android and Windows and at
/// `tauri://localhost/` elsewhere).
fn is_app_page(url: &Url) -> bool {
    match url.scheme() {
        "tauri" => url.host_str() == Some("localhost"),
        "http" | "https" => url.host_str() == Some("tauri.localhost") && url.port().is_none(),
        _ => false,
    }
}

/// Where the web view may go: the app's start page, or the bridge's
/// origin of the launch running now.
fn allowed(bridge: &Mutex<Option<Origin>>, to: &Url) -> bool {
    is_app_page(to)
        || bridge
            .lock()
            .ok()
            .and_then(|origin| origin.as_ref().map(|origin| origin.allows(to)))
            .unwrap_or(false)
}

/// Open the one window at the app's start page; each time the start page
/// loads, the launcher starts a launch and sends the web view to it
/// (`launch`).
///
/// # Errors
/// The web view could not be made.
pub(crate) fn open(app: &App, bridge: Arc<Mutex<Option<Origin>>>) -> tauri::Result<()> {
    let _window = WebviewWindowBuilder::new(app, MAIN, WebviewUrl::App("index.html".into()))
        .on_navigation(move |to| {
            let allowed = allowed(&bridge, to);
            if !allowed {
                // The scheme and host only: an address may carry a secret.
                tracing::warn!(scheme = to.scheme(), host = to.host_str().unwrap_or(""), "navigation refused");
            }
            allowed
        })
        .on_page_load(|webview, payload| {
            if matches!(payload.event(), PageLoadEvent::Finished) && is_app_page(payload.url()) {
                crate::launch::start_page_loaded(&webview, payload.url());
            }
        })
        .initialization_script(label_script(crate::LABEL))
        .build()?;
    Ok(())
}

/// The script that shows `label` over the page for a few seconds. The
/// label is numbers and dots or `dev` (checked by `build.rs`), so it is
/// put into the script as it is.
fn label_script(label: &str) -> String {
    format!(
        r##"(() => {{
  if (window.top !== window) return;
  const show = () => {{
    const pill = document.createElement("div");
    pill.textContent = "Vessel {label}";
    pill.setAttribute("aria-hidden", "true");
    const style = pill.style;
    style.position = "fixed";
    style.left = "50%";
    style.top = "calc(env(safe-area-inset-top, 0px) + 8px)";
    style.transform = "translateX(-50%)";
    style.zIndex = "2147483647";
    style.pointerEvents = "none";
    style.padding = "4px 12px";
    style.borderRadius = "999px";
    style.background = "rgba(18, 18, 18, 0.88)";
    style.color = "#FAFAFA";
    style.font = '500 13px/1.4 "Pretendard Variable", Pretendard, Geist, system-ui, sans-serif';
    style.transition = "opacity 0.4s";
    document.body.appendChild(pill);
    setTimeout(() => {{
      style.opacity = "0";
      setTimeout(() => pill.remove(), 600);
    }}, 4000);
  }};
  if (document.body) show();
  else document.addEventListener("DOMContentLoaded", show, {{ once: true }});
}})();"##
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(text: &str) -> Url {
        text.parse().unwrap()
    }

    #[test]
    fn only_the_bridges_own_origin_is_allowed() {
        let launch = url("http://vessel-0123abcd.localhost:40123/#bootstrap=00");
        let origin = Origin::of(&launch).unwrap();
        assert!(origin.allows(&url("http://vessel-0123abcd.localhost:40123/sessions")));
        assert!(origin.allows(&launch));
        for elsewhere in [
            "https://vessel-0123abcd.localhost:40123/",
            "http://vessel-0123abcd.localhost:40124/",
            "http://vessel-ffff.localhost:40123/",
            "http://127.0.0.1:40123/",
            "https://example.com/",
            "javascript:alert(1)",
            "data:text/html,hi",
            "file:///etc/hosts",
        ] {
            assert!(!origin.allows(&url(elsewhere)), "{elsewhere}");
        }
    }

    #[test]
    fn the_start_page_and_the_current_launch_are_allowed_and_nothing_else() {
        let bridge = Mutex::new(None);
        assert!(allowed(&bridge, &url("http://tauri.localhost/index.html")));
        assert!(allowed(&bridge, &url("tauri://localhost/index.html#failed")));
        assert!(!allowed(&bridge, &url("http://tauri.localhost:8080/")));
        assert!(!allowed(&bridge, &url("http://vessel-01.localhost:4000/")));
        *bridge.lock().unwrap() = Origin::of(&url("http://vessel-01.localhost:4000/#bootstrap=0"));
        assert!(allowed(&bridge, &url("http://vessel-01.localhost:4000/sessions")));
        assert!(!allowed(&bridge, &url("http://vessel-02.localhost:4000/")));
        assert!(!allowed(&bridge, &url("https://example.com/")));
    }

    #[test]
    fn an_address_without_a_port_or_host_is_not_the_bridges() {
        assert!(Origin::of(&url("http://vessel.localhost/")).is_none());
        assert!(Origin::of(&url("https://vessel.localhost:1/")).is_none());
        assert!(Origin::of(&url("data:text/html,hi")).is_none());
    }

    #[test]
    fn the_label_script_names_the_build_and_puts_pretendard_first() {
        let script = label_script("0.0.0.0.1");
        assert!(script.contains("Vessel 0.0.0.0.1"));
        let font = script.find("Pretendard").unwrap();
        assert!(font < script.find("sans-serif").unwrap());
        assert!(script.contains("pointerEvents = \"none\""));
    }
}
