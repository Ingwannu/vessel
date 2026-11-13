//! The kit's own drawings, served to GPUI as assets: the window controls'
//! four glyphs, as SVG paths on a 10 by 10 grid (Windows 11's shapes:
//! a line, a square, two offset squares, a cross).
//!
//! Embedded as text, not files: four short paths need no bundle, and an
//! app built from the kit needs no asset directory to draw its chrome.

use std::borrow::Cow;

use gpui::{AssetSource, SharedString};

/// Vessel's mark, one colour (`fill="currentColor"`), embedded from the
/// design system's own file by path (`docs/design/brand/`, the owner's
/// paths, never copied): GPUI draws an SVG as a mask in the element's
/// text colour, so the one file serves every colour the brand README
/// asks for.
const MARK: &str = include_str!("../../../../docs/design/brand/vessel-mark-mono.svg");

/// The brand colour (`docs/design/brand/README.md`): the mark's colour on
/// light surfaces and its icon's tile; never the interface's.
pub const BRAND: u32 = 0x001a_5868;

/// A glyph the kit draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    /// Vessel's mark.
    Mark,
    /// The window's minimise button.
    Minimize,
    /// Maximise.
    Maximize,
    /// Restore, when maximised.
    Restore,
    /// Close.
    Close,
    /// An update: an arrow down into a tray (the title bar's update
    /// status at a phone's width).
    Update,
    /// Three lines: opens the sidebar's sheet at a phone's width.
    Menu,
    /// A picker's caret.
    ChevronDown,
    /// Up: the find bar's previous match.
    ChevronUp,
    /// A row that opens a list of its own (a provider's models).
    ChevronRight,
    /// Back, from such a list.
    ChevronLeft,
    /// The chosen row's mark.
    Check,
    /// A search row.
    Search,
    /// Send.
    ArrowUp,
    /// Stop.
    Stop,
    /// The permissions mode.
    Shield,
    /// The effort (thinking).
    Brain,
    /// An agent.
    Bot,
    /// A failure.
    Alert,
    /// A success alert: a check inside a circle, drawn on the kit's own grid.
    CircleCheck,
    /// An error alert: a cross inside a circle, drawn on the kit's own grid.
    CircleX,
    /// A folder.
    Folder,
    /// A machine.
    Machine,
    /// Three dots: a menu of more actions (the Session actions).
    More,
    /// An agent's feature, and the features (Paseo's `Settings2`: two
    /// sliders).
    Sliders,
    /// Settings (Paseo's `Settings`: a gear).
    Gear,
    /// Copy: two sheets, one over the other (a code block's copy).
    Copy,
    /// Appearance (a painter's palette, Paseo's `Palette`).
    Palette,
    /// Shortcuts (a keyboard, Paseo's `Keyboard`).
    Keyboard,
    /// Notifications (a bell, Paseo's `Bell`).
    Bell,
    /// Models (a spark, Paseo's `Sparkles`).
    Sparkles,
    /// A machine as Settings lists it (Paseo's `Server`).
    Server,
    /// Devices (a phone, Paseo's `Smartphone`).
    Smartphone,
    /// Sync (two arrows round).
    Refresh,
    /// About (an i in a circle, Paseo's `Info`).
    Info,
    /// Back (Settings' way back to the workspace, Paseo's `ArrowLeft`).
    ArrowLeft,
    /// Adding (a machine, Paseo's `Plus`).
    Plus,
    /// A branch (a new worktree, a starting ref: Paseo's `GitBranch`).
    Branch,
    /// A pull request (Paseo's `GitPullRequest`).
    PullRequest,
    /// Fast mode, off: a lightning bolt's outline.
    Lightning,
    /// Fast mode, on: the bolt filled.
    LightningFilled,
    /// Back to the default: an arrow turning back.
    Reset,
    /// A terminal session (a prompt's chevron and a line).
    Terminal,
    /// A conversation (a speech bubble).
    Message,
    /// The archive (a box with its lid).
    Archive,
    /// Delete for good.
    Trash,
    /// Pinned.
    Pin,
    /// Work going on: three quarters of a ring, turned by `spinner`.
    Spinner,
    /// A pencil: edit (a queued message).
    Pencil,
    /// An arrow turning up and right: send into the running turn (GOAT's
    /// steer, `CornerUpRight`).
    Steer,
    /// Six dots: a handle to drag a row by.
    Grip,
    /// An X on the line glyphs' grid: take a row out (a queued message;
    /// the window's `Close` fills its box and reads heavier beside them).
    Remove,
    /// A file (the Files tree, Paseo's `File`).
    File,
    /// The code panel's toggle (Paseo's `PanelRight`).
    PanelRight,
}

/// Every glyph, for the asset source.
const ALL: [Glyph; 54] = [
    Glyph::Mark,
    Glyph::Minimize,
    Glyph::Maximize,
    Glyph::Restore,
    Glyph::Close,
    Glyph::Update,
    Glyph::Menu,
    Glyph::ChevronDown,
    Glyph::ChevronUp,
    Glyph::ChevronRight,
    Glyph::ChevronLeft,
    Glyph::Check,
    Glyph::Search,
    Glyph::ArrowUp,
    Glyph::Stop,
    Glyph::Shield,
    Glyph::Brain,
    Glyph::Bot,
    Glyph::Alert,
    Glyph::CircleCheck,
    Glyph::CircleX,
    Glyph::Folder,
    Glyph::Machine,
    Glyph::More,
    Glyph::Sliders,
    Glyph::Gear,
    Glyph::Copy,
    Glyph::Palette,
    Glyph::Keyboard,
    Glyph::Bell,
    Glyph::Sparkles,
    Glyph::Server,
    Glyph::Smartphone,
    Glyph::Refresh,
    Glyph::Info,
    Glyph::ArrowLeft,
    Glyph::Plus,
    Glyph::Branch,
    Glyph::PullRequest,
    Glyph::Lightning,
    Glyph::LightningFilled,
    Glyph::Reset,
    Glyph::Terminal,
    Glyph::Message,
    Glyph::Archive,
    Glyph::Trash,
    Glyph::Pin,
    Glyph::Spinner,
    Glyph::Pencil,
    Glyph::Steer,
    Glyph::Grip,
    Glyph::Remove,
    Glyph::File,
    Glyph::PanelRight,
];

/// A line glyph on Lucide's grid (24 units, a stroke of 1.5, round ends),
/// as Paseo's icons are drawn; the paths are the kit's own.
macro_rules! line_glyph {
    ($body:literal) => {
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">"#,
            $body,
            "</svg>"
        )
    };
}

impl Glyph {
    /// Its asset path.
    #[must_use]
    pub const fn path(self) -> &'static str {
        match self {
            Self::Mark => "kit/mark.svg",
            Self::Minimize => "kit/minimize.svg",
            Self::Maximize => "kit/maximize.svg",
            Self::Restore => "kit/restore.svg",
            Self::Close => "kit/close.svg",
            Self::Update => "kit/update.svg",
            Self::Menu => "kit/menu.svg",
            Self::ChevronDown => "kit/chevron-down.svg",
            Self::ChevronUp => "kit/chevron-up.svg",
            Self::ChevronRight => "kit/chevron-right.svg",
            Self::ChevronLeft => "kit/chevron-left.svg",
            Self::Check => "kit/check.svg",
            Self::Search => "kit/search.svg",
            Self::ArrowUp => "kit/arrow-up.svg",
            Self::Stop => "kit/stop.svg",
            Self::Shield => "kit/shield.svg",
            Self::Brain => "kit/brain.svg",
            Self::Bot => "kit/bot.svg",
            Self::Alert => "kit/alert.svg",
            Self::CircleCheck => "kit/circle-check.svg",
            Self::CircleX => "kit/circle-x.svg",
            Self::Folder => "kit/folder.svg",
            Self::Machine => "kit/machine.svg",
            Self::More => "kit/more.svg",
            Self::Sliders => "kit/sliders.svg",
            Self::Gear => "kit/gear.svg",
            Self::Copy => "kit/copy.svg",
            Self::Palette => "kit/palette.svg",
            Self::Keyboard => "kit/keyboard.svg",
            Self::Bell => "kit/bell.svg",
            Self::Sparkles => "kit/sparkles.svg",
            Self::Server => "kit/server.svg",
            Self::Smartphone => "kit/smartphone.svg",
            Self::Refresh => "kit/refresh.svg",
            Self::Info => "kit/info.svg",
            Self::ArrowLeft => "kit/arrow-left.svg",
            Self::Plus => "kit/plus.svg",
            Self::Branch => "kit/branch.svg",
            Self::File => "kit/file.svg",
            Self::PanelRight => "kit/panel-right.svg",
            Self::PullRequest => "kit/pull-request.svg",
            Self::Lightning => "kit/lightning.svg",
            Self::LightningFilled => "kit/lightning-filled.svg",
            Self::Reset => "kit/reset.svg",
            Self::Terminal => "kit/terminal.svg",
            Self::Message => "kit/message.svg",
            Self::Archive => "kit/archive.svg",
            Self::Trash => "kit/trash.svg",
            Self::Pin => "kit/pin.svg",
            Self::Spinner => "kit/spinner.svg",
            Self::Pencil => "kit/pencil.svg",
            Self::Steer => "kit/steer.svg",
            Self::Grip => "kit/grip.svg",
            Self::Remove => "kit/remove.svg",
        }
    }

    #[expect(clippy::too_many_lines, reason = "one arm per glyph, each its drawing")]
    const fn svg(self) -> &'static str {
        match self {
            Self::Mark => MARK,
            Self::Minimize => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M0 5.5h10" stroke="black" stroke-width="1"/></svg>"#
            }
            Self::Maximize => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect x="0.5" y="0.5" width="9" height="9" fill="none" stroke="black" stroke-width="1"/></svg>"#
            }
            Self::Restore => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect x="0.5" y="2.5" width="7" height="7" fill="none" stroke="black" stroke-width="1"/><path d="M2.5 2.5V0.5h7v7h-2" fill="none" stroke="black" stroke-width="1"/></svg>"#
            }
            Self::Close => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M0.5 0.5l9 9M9.5 0.5l-9 9" stroke="black" stroke-width="1"/></svg>"#
            }
            Self::Update => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path d="M6 1.5v6.5M3.2 5.4L6 8.2l2.8-2.8M1.5 10.5h9" fill="none" stroke="black" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>"#
            }
            Self::Menu => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path d="M1.5 3h9M1.5 6h9M1.5 9h9" fill="none" stroke="black" stroke-width="1.2" stroke-linecap="round"/></svg>"#
            }
            Self::ChevronDown => line_glyph!(r#"<path d="M6 9l6 6 6-6"/>"#),
            Self::ChevronUp => line_glyph!(r#"<path d="M6 15l6-6 6 6"/>"#),
            Self::ChevronRight => line_glyph!(r#"<path d="M9 6l6 6-6 6"/>"#),
            Self::ChevronLeft => line_glyph!(r#"<path d="M15 6l-6 6 6 6"/>"#),
            Self::Check => line_glyph!(r#"<path d="M20 6L9 17l-5-5"/>"#),
            Self::Search => {
                line_glyph!(r#"<circle cx="11" cy="11" r="7"/><path d="M20 20l-3.6-3.6"/>"#)
            }
            Self::ArrowUp => line_glyph!(r#"<path d="M12 19V5M5 12l7-7 7 7"/>"#),
            Self::Stop => line_glyph!(r#"<rect x="6" y="6" width="12" height="12" rx="1.5"/>"#),
            Self::Shield => {
                line_glyph!(r#"<path d="M12 3l7 3v5.5c0 4.6-3 8.1-7 9.5-4-1.4-7-4.9-7-9.5V6z"/>"#)
            }
            Self::Brain => line_glyph!(
                r#"<path d="M12 5.5a3 3 0 0 0-5.6-1.2A3 3 0 0 0 4 8.5a3.2 3.2 0 0 0 .8 5.8A3 3 0 0 0 9 19a3 3 0 0 0 3-1.5z"/><path d="M12 5.5a3 3 0 0 1 5.6-1.2A3 3 0 0 1 20 8.5a3.2 3.2 0 0 1-.8 5.8A3 3 0 0 1 15 19a3 3 0 0 1-3-1.5"/>"#
            ),
            Self::Bot => line_glyph!(
                r#"<rect x="4.5" y="8" width="15" height="11" rx="2.5"/><path d="M12 8V4.5M9.5 12.5v1.5M14.5 12.5v1.5"/>"#
            ),
            Self::Alert => {
                line_glyph!(r#"<path d="M12 4l9 16H3z"/><path d="M12 10v4M12 17v.01"/>"#)
            }
            Self::CircleCheck => {
                line_glyph!(r#"<circle cx="12" cy="12" r="9"/><path d="M7.5 12l3 3 6-6"/>"#)
            }
            Self::CircleX => {
                line_glyph!(r#"<circle cx="12" cy="12" r="9"/><path d="M9 9l6 6M15 9l-6 6"/>"#)
            }
            Self::Folder => line_glyph!(
                r#"<path d="M3 7.5A2.5 2.5 0 0 1 5.5 5H9l2 2h7.5A2.5 2.5 0 0 1 21 9.5v7a2.5 2.5 0 0 1-2.5 2.5h-13A2.5 2.5 0 0 1 3 16.5z"/>"#
            ),
            Self::Machine => line_glyph!(
                r#"<rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8 20h8M12 16v4"/>"#
            ),
            Self::More => line_glyph!(
                r#"<circle cx="5" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/>"#
            ),
            Self::Sliders => line_glyph!(
                r#"<path d="M19.5 7H10M14 17H4.5"/><circle cx="7" cy="7" r="2.75"/><circle cx="17" cy="17" r="2.75"/>"#
            ),
            Self::Gear => line_glyph!(
                r#"<circle cx="12" cy="12" r="3"/><path d="M12 2.75v2.5M12 18.75v2.5M2.75 12h2.5M18.75 12h2.5M5.46 5.46l1.77 1.77M16.77 16.77l1.77 1.77M5.46 18.54l1.77-1.77M16.77 7.23l1.77-1.77"/><circle cx="12" cy="12" r="6.25"/>"#
            ),
            Self::Pencil => line_glyph!(
                r#"<path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L8 18l-4 1 1-4z"/><path d="M14.5 5.5l3 3"/>"#
            ),
            Self::Remove => line_glyph!(r#"<path d="M18 6L6 18M6 6l12 12"/>"#),
            Self::Steer => {
                line_glyph!(r#"<path d="M15 14l5-5-5-5"/><path d="M4 20v-7a4 4 0 0 1 4-4h12"/>"#)
            }
            Self::Grip => line_glyph!(
                r#"<circle cx="9" cy="6" r="1"/><circle cx="15" cy="6" r="1"/><circle cx="9" cy="12" r="1"/><circle cx="15" cy="12" r="1"/><circle cx="9" cy="18" r="1"/><circle cx="15" cy="18" r="1"/>"#
            ),
            Self::Copy => line_glyph!(
                r#"<rect x="8" y="8" width="13" height="13" rx="2"/><path d="M16 8V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h3"/>"#
            ),
            Self::Palette => line_glyph!(
                r#"<path d="M12 3a9 9 0 1 0 0 18c1.1 0 2-.9 2-2 0-.5-.2-1-.5-1.3-.3-.4-.5-.8-.5-1.3 0-1.1.9-2 2-2h2.4a4.6 4.6 0 0 0 4.6-4.6C21 5.9 17 3 12 3z"/><circle cx="7.5" cy="11" r="1"/><circle cx="10" cy="7" r="1"/><circle cx="15" cy="7.5" r="1"/>"#
            ),
            Self::Keyboard => line_glyph!(
                r#"<rect x="2.5" y="6" width="19" height="12" rx="2"/><path d="M6.5 10h.01M10 10h.01M14 10h.01M17.5 10h.01M8 14h8"/>"#
            ),
            Self::Bell => line_glyph!(
                r#"<path d="M6 16v-5a6 6 0 0 1 12 0v5l1.5 2h-15z"/><path d="M10 20.5a2 2 0 0 0 4 0"/>"#
            ),
            Self::Sparkles => line_glyph!(
                r#"<path d="M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8z"/><path d="M19 15v4M17 17h4"/>"#
            ),
            Self::Server => line_glyph!(
                r#"<rect x="3" y="4" width="18" height="7" rx="2"/><rect x="3" y="13" width="18" height="7" rx="2"/><path d="M7 7.5h.01M7 16.5h.01"/>"#
            ),
            Self::Smartphone => line_glyph!(
                r#"<rect x="6.5" y="2.5" width="11" height="19" rx="2.5"/><path d="M11 18h2"/>"#
            ),
            Self::Refresh => line_glyph!(
                r#"<path d="M20 11a8 8 0 0 0-14.5-4.5L4 8M4 4v4h4"/><path d="M4 13a8 8 0 0 0 14.5 4.5L20 16M20 20v-4h-4"/>"#
            ),
            Self::Info => {
                line_glyph!(r#"<circle cx="12" cy="12" r="9"/><path d="M12 11v5M12 8v.01"/>"#)
            }
            Self::ArrowLeft => line_glyph!(r#"<path d="M19 12H5M12 19l-7-7 7-7"/>"#),
            Self::Plus => line_glyph!(r#"<path d="M12 5v14M5 12h14"/>"#),
            Self::Branch => line_glyph!(
                r#"<path d="M6 3v12"/><circle cx="18" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M18 9a9 9 0 0 1-9 9"/>"#
            ),
            Self::Lightning => {
                line_glyph!(r#"<path d="M13.25 2.75 4.75 13.5h6.75l-1 7.75 8.5-10.75h-6.75z"/>"#)
            }
            Self::LightningFilled => concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="black" stroke="black" stroke-width="1.5" stroke-linejoin="round">"#,
                r#"<path d="M13.25 2.75 4.75 13.5h6.75l-1 7.75 8.5-10.75h-6.75z"/>"#,
                "</svg>"
            ),
            Self::Reset => line_glyph!(
                r#"<path d="M4.75 12a7.25 7.25 0 1 0 2.12-5.13L4.75 9"/><path d="M4.75 4.75V9H9"/>"#
            ),
            Self::Spinner => {
                line_glyph!(r#"<path d="M12 3.75A8.25 8.25 0 1 1 3.75 12" stroke-width="2"/>"#)
            }
            Self::File => line_glyph!(
                r#"<path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/>"#
            ),
            Self::PanelRight => line_glyph!(
                r#"<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M15 3v18"/>"#
            ),
            Self::PullRequest => line_glyph!(
                r#"<circle cx="18" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><path d="M13 6h3a2 2 0 0 1 2 2v7M6 9v12"/>"#
            ),
            // The page's own drawings (`web/src/ui/kit/icon.css`, on a
            // 16 grid at a stroke of 1), at one and a half times.
            Self::Terminal => line_glyph!(r#"<path d="m6 7.5 4.5 4.5-4.5 4.5M12 16.5h6"/>"#),
            Self::Message => line_glyph!(r#"<path d="M4.5 6.75h15v10.5H9l-4.5 3v-13.5z"/>"#),
            Self::Archive => line_glyph!(
                r#"<rect x="3" y="4.5" width="18" height="5.25" rx="1.5"/><path d="M4.8 9.75v7.5A2.25 2.25 0 0 0 7.05 19.5h9.9a2.25 2.25 0 0 0 2.25-2.25v-7.5"/><path d="M9.75 13.95h4.5"/>"#
            ),
            Self::Trash => line_glyph!(
                r#"<path d="M3.75 6h16.5M18 6v13.5a2.25 2.25 0 0 1-2.25 2.25h-7.5A2.25 2.25 0 0 1 6 19.5V6M9 6V4.125A1.875 1.875 0 0 1 10.875 2.25h2.25A1.875 1.875 0 0 1 15 4.125V6"/>"#
            ),
            Self::Pin => line_glyph!(
                r#"<path d="M8.25 3.75h7.5"/><path d="M9.75 3.75v6L6 13.5v2.25h12V13.5l-3.75-3.75v-6"/><path d="M12 15.75V21"/>"#
            ),
        }
    }
}

/// The kit's assets, for `Application::with_assets`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        let glyph = ALL.into_iter().find(|glyph| glyph.path() == path);
        Ok(glyph
            .map(|glyph| Cow::Borrowed(glyph.svg().as_bytes()))
            .or_else(|| crate::agent_mark::load(path).map(Cow::Borrowed)))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(ALL
            .into_iter()
            .map(Glyph::path)
            .filter(|asset| asset.starts_with(path))
            .map(SharedString::from)
            .chain(
                crate::agent_mark::asset_paths()
                    .filter(|asset| asset.starts_with(path))
                    .map(SharedString::from),
            )
            .collect())
    }
}

/// The window's icon (X11 reads it; Windows and macOS take theirs from the
/// executable and the bundle, the release track's): the mark in white on
/// a rounded tile of the brand colour, `size` pixels square.
///
/// # Errors
/// The mark could not be drawn (only a broken build could cause it).
pub fn window_icon(size: u32) -> Result<image::RgbaImage, IconError> {
    // The tile is drawn in the mark's own units, read from its view box
    // (the owner redraws the mark; nothing here assumes its numbers), the
    // mark inset by a tenth of its box on each side.
    let view = view_box(MARK).ok_or(IconError::Mark)?;
    let paths = MARK
        .split_once("<path")
        .map(|(_, rest)| format!("<path{rest}"))
        .ok_or(IconError::Mark)?
        .replace("currentColor", "#ffffff")
        .replace("</svg>", "");
    let tile = tile_svg(view, &paths);
    let tree = resvg::usvg::Tree::from_str(&tile, &resvg::usvg::Options::default())
        .map_err(|_bad| IconError::Mark)?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).ok_or(IconError::Size)?;
    let scale = size_scale(size, tree.size().width());
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    image::RgbaImage::from_raw(size, size, pixmap.take()).ok_or(IconError::Size)
}

/// A drawing's view box: x, y, width, height.
fn view_box(svg: &str) -> Option<[f32; 4]> {
    let (_, rest) = svg.split_once("viewBox=\"")?;
    let (text, _) = rest.split_once('"')?;
    let mut numbers = text.split_whitespace().map(str::parse::<f32>);
    let mut next = || numbers.next()?.ok();
    Some([next()?, next()?, next()?, next()?])
}

/// The window icon's drawing: a rounded tile of the brand colour a fifth
/// larger than the mark's view box, the mark's `paths` on it.
#[expect(
    clippy::float_arithmetic,
    reason = "a view box's units are fractional; this is a drawing's geometry"
)]
fn tile_svg(view: [f32; 4], paths: &str) -> String {
    let [x, y, width, height] = view;
    let (inset_x, inset_y) = (width / 10.0, height / 10.0);
    let (left, top) = (x - inset_x, y - inset_y);
    let (wide, high) = (width + 2.0 * inset_x, height + 2.0 * inset_y);
    // The tile's corner, as the first icon's: 180 of 840.
    let corner = wide * 180.0 / 840.0;
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{left} {top} {wide} {high}"><rect x="{left}" y="{top}" width="{wide}" height="{high}" rx="{corner}" fill="#{BRAND:06x}"/>{paths}</svg>"##
    )
}

/// How much to scale a drawing `width` wide to `size` pixels.
#[expect(
    clippy::cast_precision_loss,
    clippy::as_conversions,
    clippy::float_arithmetic,
    reason = "an icon's size is a few hundred pixels: exact in f32, and the ratio is what resvg takes"
)]
fn size_scale(size: u32, width: f32) -> f32 {
    size as f32 / width
}

/// The icon could not be made.
#[derive(Debug, thiserror::Error)]
pub enum IconError {
    /// The mark's file is not as expected.
    #[error("the mark could not be read")]
    Mark,
    /// The size is not a drawable one.
    #[error("not a drawable icon size")]
    Size,
}

#[cfg(test)]
mod tests {
    use super::{MARK, view_box, window_icon};

    #[test]
    fn the_icon_is_drawn_from_the_marks_own_view_box() {
        assert_eq!(view_box(MARK), Some([170.0, 170.0, 1660.0, 1660.0]));
        let icon = window_icon(64).unwrap();
        // The tile's rounded corner is clear; its edge, inside the inset,
        // is the brand colour.
        assert_eq!(icon.get_pixel(0, 0).0[3], 0);
        let edge = icon.get_pixel(32, 2).0;
        assert_eq!((edge[0], edge[1], edge[2]), (0x1a, 0x58, 0x68));
    }
}
