//! The kit's own drawings, served to GPUI as assets: the window controls'
//! four glyphs, as SVG paths on a 10 by 10 grid (Windows 11's shapes:
//! a line, a square, two offset squares, a cross).
//!
//! Embedded as text, not files: the kit needs no separate asset bundle.
//! The inventory (the enum, its list and its paths) is here and the
//! drawings are in `drawing.rs`, one arm per glyph: the two stayed in one
//! file until the git row's glyphs took it past its listed size
//! (docs/QUALITY.md M3). A registration still cannot miss its vector: the
//! drawing's match has no catch-all.

use std::borrow::Cow;

use gpui::{AssetSource, SharedString};

mod drawing;
mod icon;

#[cfg(test)]
use icon::view_box;
pub use icon::{IconError, window_icon};

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
    /// Outward link arrow, consumed at12 by `ExternalLink`.
    ArrowUpRight,
    /// Stop.
    Stop,
    /// The permissions mode.
    Shield,
    /// A permissions mode that accepts edits (Paseo's `ShieldPlus`).
    ShieldPlus,
    /// A permissions mode that decides for itself (Paseo's `ShieldCheck`).
    ShieldCheck,
    /// A permissions mode that asks nothing (Paseo's `ShieldOff`).
    ShieldOff,
    /// A planning mode (Paseo's `ShieldEllipsis`).
    ShieldEllipsis,
    /// A permissions mode that asks before writing (Paseo's `ShieldAlert`).
    ShieldAlert,
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
    /// A task not started (Paseo's `Circle`): a ring on the same grid as
    /// [`Self::CircleCheck`].
    Circle,
    /// A task under way (Paseo's `CircleDot`): the ring with a dot.
    CircleDot,
    /// A task list (Paseo's `CheckSquare`): a check in a rounded square.
    SquareCheck,
    /// Scheduling: a calendar with a clock, on the shared 24-unit line grid.
    /// One asset serves navigation and empty states; no screen draws a second.
    CalendarClock,
    /// A folder.
    Folder,
    /// A project: a folder holding a branch (Paseo's `FolderGit2`, an
    /// original drawing on the shared 24-unit line grid).
    FolderGit,
    /// The agents a machine can run: three stacked boxes (Paseo's
    /// `Boxes`, an original drawing on the shared 24-unit line grid).
    Boxes,
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
    /// Diagnostics: an original stethoscope on the shared 24-unit line grid.
    Stethoscope,
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
    /// A draft pull request (Paseo's `GitPullRequestDraft`).
    PullRequestDraft,
    /// A closed pull request (Paseo's `GitPullRequestClosed`).
    PullRequestClosed,
    /// A merged pull request (Paseo's `GitMerge`).
    GitMerge,
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
    /// A delete action with the two interior bin strokes (legacy Trash stays).
    Trash2,
    /// Restore an archived item: the archive with its upward arrow.
    ArchiveRestore,
    /// A horizontal minus on the shared line grid.
    Minus,
    /// Create another branch, with the plus beside the branch's nodes.
    GitBranchPlus,
    /// A commit node on its history line.
    Commit,
    /// Push: an upward transfer arrow and an open tray.
    Upload,
    /// Pull: a downward transfer arrow and an open tray.
    Download,
    /// Undo a file change: a returning arrow, separate from Reset.
    Undo2,
    /// Fork a conversation: one way parting in two (Paseo's `Split`, its
    /// fork menu).
    Split,
    /// Pinned.
    Pin,
    /// A vertical kebab, for a row's actions (horizontal More stays unchanged).
    MoreVertical,
    /// Pause a recurring action.
    Pause,
    /// Resume an action.
    Play,
    /// Run again now: one clockwise rotation arrow.
    RotateCw,
    /// Original combined activity asset, retained for compatibility. Use the
    /// loading control to apply track alpha after monochrome mask correction.
    ActivityIndicator,
    /// Opaque activity track; its renderer applies the source alpha after masking.
    ActivityTrack,
    /// Opaque activity foreground, rotated independently of its invariant track.
    ActivityArc,
    /// Work going on: three quarters of a ring, turned by `spinner`.
    Spinner,
    /// The board (three lanes of cards).
    Board,
    /// The light theme (a sun).
    Sun,
    /// The dark theme (a moon).
    Moon,
    /// The system's theme (a screen).
    Monitor,
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
    /// A file with lines of text: an attached file's pill (Paseo's
    /// `FileText`).
    FileText,
    /// The code panel's toggle (Paseo's `PanelRight`).
    PanelRight,
    /// The desktop sidebar toggle, preserving the existing right-panel glyph.
    PanelLeft,
    /// The compact sidebar's two full bars and one short bar, painted 16 by 12.
    MenuCompact,
    /// Realtime voice microphone, source 2.5 stroke on the shared 24-unit grid.
    Mic,
    /// Realtime voice mute state; distinct from timeline `MicVocal`.
    MicOff,
    /// Filled stop square; the existing outline Stop remains unchanged.
    SquareFilled,
    /// A usage dial, on the shared24-unit line grid.
    Gauge,
    /// A catalog package with its add action.
    PackagePlus,
}

/// Every glyph, for the asset source.
const ALL: [Glyph; 98] = [
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
    Glyph::ArrowUpRight,
    Glyph::Stop,
    Glyph::Shield,
    Glyph::ShieldPlus,
    Glyph::ShieldCheck,
    Glyph::ShieldOff,
    Glyph::ShieldEllipsis,
    Glyph::ShieldAlert,
    Glyph::Brain,
    Glyph::Bot,
    Glyph::Alert,
    Glyph::CircleCheck,
    Glyph::CircleX,
    Glyph::Circle,
    Glyph::CircleDot,
    Glyph::SquareCheck,
    Glyph::CalendarClock,
    Glyph::Folder,
    Glyph::FolderGit,
    Glyph::Boxes,
    Glyph::Machine,
    Glyph::More,
    Glyph::Sliders,
    Glyph::Gear,
    Glyph::Copy,
    Glyph::Palette,
    Glyph::Stethoscope,
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
    Glyph::PullRequestDraft,
    Glyph::PullRequestClosed,
    Glyph::GitMerge,
    Glyph::Lightning,
    Glyph::LightningFilled,
    Glyph::Reset,
    Glyph::Terminal,
    Glyph::Message,
    Glyph::Archive,
    Glyph::Trash,
    Glyph::Trash2,
    Glyph::ArchiveRestore,
    Glyph::Minus,
    Glyph::GitBranchPlus,
    Glyph::Commit,
    Glyph::Upload,
    Glyph::Download,
    Glyph::Undo2,
    Glyph::Split,
    Glyph::Pin,
    Glyph::MoreVertical,
    Glyph::Pause,
    Glyph::Play,
    Glyph::RotateCw,
    Glyph::ActivityIndicator,
    Glyph::ActivityTrack,
    Glyph::ActivityArc,
    Glyph::Spinner,
    Glyph::Board,
    Glyph::Sun,
    Glyph::Moon,
    Glyph::Monitor,
    Glyph::Pencil,
    Glyph::Steer,
    Glyph::Grip,
    Glyph::Remove,
    Glyph::File,
    Glyph::FileText,
    Glyph::PanelRight,
    Glyph::PanelLeft,
    Glyph::MenuCompact,
    Glyph::Mic,
    Glyph::MicOff,
    Glyph::SquareFilled,
    Glyph::Gauge,
    Glyph::PackagePlus,
];

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
            Self::ArrowUpRight => "kit/arrow-up-right.svg",
            Self::Stop => "kit/stop.svg",
            Self::Shield => "kit/shield.svg",
            Self::ShieldPlus => "kit/shield-plus.svg",
            Self::ShieldCheck => "kit/shield-check.svg",
            Self::ShieldOff => "kit/shield-off.svg",
            Self::ShieldEllipsis => "kit/shield-ellipsis.svg",
            Self::ShieldAlert => "kit/shield-alert.svg",
            Self::Brain => "kit/brain.svg",
            Self::Bot => "kit/bot.svg",
            Self::Alert => "kit/alert.svg",
            Self::CircleCheck => "kit/circle-check.svg",
            Self::CircleX => "kit/circle-x.svg",
            Self::Circle => "kit/circle.svg",
            Self::CircleDot => "kit/circle-dot.svg",
            Self::SquareCheck => "kit/square-check.svg",
            Self::CalendarClock => "kit/calendar-clock.svg",
            Self::Folder => "kit/folder.svg",
            Self::FolderGit => "kit/folder-git.svg",
            Self::Boxes => "kit/boxes.svg",
            Self::Machine => "kit/machine.svg",
            Self::More => "kit/more.svg",
            Self::Sliders => "kit/sliders.svg",
            Self::Gear => "kit/gear.svg",
            Self::Copy => "kit/copy.svg",
            Self::Palette => "kit/palette.svg",
            Self::Stethoscope => "kit/stethoscope.svg",
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
            Self::FileText => "kit/file-text.svg",
            Self::PanelRight => "kit/panel-right.svg",
            Self::PanelLeft => "kit/panel-left.svg",
            Self::MenuCompact => "kit/menu-compact.svg",
            Self::PullRequest => "kit/pull-request.svg",
            Self::PullRequestDraft => "kit/pull-request-draft.svg",
            Self::PullRequestClosed => "kit/pull-request-closed.svg",
            Self::GitMerge => "kit/git-merge.svg",
            Self::Lightning => "kit/lightning.svg",
            Self::LightningFilled => "kit/lightning-filled.svg",
            Self::Reset => "kit/reset.svg",
            Self::Terminal => "kit/terminal.svg",
            Self::Message => "kit/message.svg",
            Self::Archive => "kit/archive.svg",
            Self::Trash => "kit/trash.svg",
            Self::Trash2 => "kit/trash-2.svg",
            Self::ArchiveRestore => "kit/archive-restore.svg",
            Self::Minus => "kit/minus.svg",
            Self::GitBranchPlus => "kit/git-branch-plus.svg",
            Self::Commit => "kit/commit.svg",
            Self::Upload => "kit/upload.svg",
            Self::Download => "kit/download.svg",
            Self::Undo2 => "kit/undo-2.svg",
            Self::Split => "kit/split.svg",
            Self::Pin => "kit/pin.svg",
            Self::MoreVertical => "kit/more-vertical.svg",
            Self::Pause => "kit/pause.svg",
            Self::Play => "kit/play.svg",
            Self::RotateCw => "kit/rotate-cw.svg",
            Self::ActivityIndicator => "kit/activity-indicator.svg",
            Self::ActivityTrack => "kit/activity-track.svg",
            Self::ActivityArc => "kit/activity-arc.svg",
            Self::Spinner => "kit/spinner.svg",
            Self::Board => "kit/board.svg",
            Self::Sun => "kit/sun.svg",
            Self::Moon => "kit/moon.svg",
            Self::Monitor => "kit/monitor.svg",
            Self::Pencil => "kit/pencil.svg",
            Self::Steer => "kit/steer.svg",
            Self::Grip => "kit/grip.svg",
            Self::Remove => "kit/remove.svg",
            Self::Mic => "kit/mic.svg",
            Self::MicOff => "kit/mic-off.svg",
            Self::SquareFilled => "kit/square-filled.svg",
            Self::Gauge => "kit/gauge.svg",
            Self::PackagePlus => "kit/package-plus.svg",
        }
    }
}

/// The kit's assets, for `Application::with_assets`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        // The agents' marks are not here: `agent_mark` rasterises its own
        // embedded drawings at the display's pixels.
        let glyph = ALL.into_iter().find(|glyph| glyph.path() == path);
        Ok(glyph.map(|glyph| Cow::Borrowed(glyph.svg().as_bytes())))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(ALL
            .into_iter()
            .map(Glyph::path)
            .filter(|asset| asset.starts_with(path))
            .map(SharedString::from)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::{Assets, Glyph, MARK, view_box, window_icon};
    use gpui::AssetSource as _;

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

    #[test]
    fn calendar_clock_is_registered_and_renders_unclipped_at_24() {
        let glyph = Glyph::CalendarClock;
        assert_eq!(view_box(glyph.svg()), Some([0.0, 0.0, 24.0, 24.0]));
        assert_eq!(
            Assets.list(glyph.path()).unwrap(),
            [gpui::SharedString::from("kit/calendar-clock.svg")]
        );
        let bytes = Assets.load(glyph.path()).unwrap().unwrap();
        let tree = resvg::usvg::Tree::from_data(&bytes, &resvg::usvg::Options::default()).unwrap();
        let mut pixels = resvg::tiny_skia::Pixmap::new(24, 24).unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut pixels.as_mut(),
        );
        let image = image::RgbaImage::from_raw(24, 24, pixels.take()).unwrap();
        assert!(image.pixels().any(|pixel| pixel.0[3] > 0));
        for at in 0..24 {
            for (x, y) in [(at, 0), (at, 23), (0, at), (23, at)] {
                assert_eq!(image.get_pixel(x, y).0[3], 0);
            }
        }
    }

    #[test]
    fn archive_and_git_glyphs_are_registered_and_unclipped() {
        for glyph in [
            Glyph::Trash2,
            Glyph::ArchiveRestore,
            Glyph::Minus,
            Glyph::GitBranchPlus,
            Glyph::Commit,
            Glyph::Undo2,
            Glyph::Split,
        ] {
            assert_eq!(view_box(glyph.svg()), Some([0.0, 0.0, 24.0, 24.0]));
            let bytes = Assets.load(glyph.path()).unwrap().unwrap();
            let tree =
                resvg::usvg::Tree::from_data(&bytes, &resvg::usvg::Options::default()).unwrap();
            let mut pixels = resvg::tiny_skia::Pixmap::new(24, 24).unwrap();
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::identity(),
                &mut pixels.as_mut(),
            );
            let image = image::RgbaImage::from_raw(24, 24, pixels.take()).unwrap();
            assert!(image.pixels().any(|pixel| pixel.0[3] > 0), "{glyph:?}");
            for at in 0..24 {
                for (x, y) in [(at, 0), (at, 23), (0, at), (23, at)] {
                    assert_eq!(image.get_pixel(x, y).0[3], 0, "{glyph:?}");
                }
            }
        }
        assert_eq!(Glyph::Commit.path(), "kit/commit.svg");
    }
}

#[cfg(test)]
mod header_glyph_tests {
    use super::{Assets, Glyph, view_box};
    use gpui::AssetSource as _;

    #[test]
    fn compact_menu_has_the_reference_painted_extents_and_gaps_at_16_by_12() {
        assert_eq!(
            view_box(Glyph::MenuCompact.svg()),
            Some([0.0, 0.0, 16.0, 12.0])
        );
        let bytes = Assets.load(Glyph::MenuCompact.path()).unwrap().unwrap();
        let tree = resvg::usvg::Tree::from_data(&bytes, &resvg::usvg::Options::default()).unwrap();
        let boxes: Vec<_> = tree
            .root()
            .children()
            .iter()
            .map(|node| {
                let rect = node.abs_bounding_box();
                // usvg's rounded-rect cubic extremum at x=0 can be
                // -2.75e-32. Normalize only sub-EPSILON zero-origin noise;
                // all sizes/y positions and native raster gaps stay exact.
                let x = if rect.x().abs() < f32::EPSILON {
                    0.0
                } else {
                    rect.x()
                };
                [x, rect.y(), rect.width(), rect.height()]
            })
            .collect();
        assert_eq!(
            boxes,
            [
                [0.0, 0.0, 16.0, 1.5],
                [0.0, 5.25, 16.0, 1.5],
                [0.0, 10.5, 8.0, 1.5]
            ]
        );
        let mut pixels = resvg::tiny_skia::Pixmap::new(16, 12).unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut pixels.as_mut(),
        );
        let image = image::RgbaImage::from_raw(16, 12, pixels.take()).unwrap();
        for y in [0, 1, 5, 6, 10, 11] {
            assert!(image.get_pixel(4, y).0[3] > 0);
        }
        for y in [2, 3, 4, 7, 8, 9] {
            for x in 0..16 {
                assert_eq!(image.get_pixel(x, y).0[3], 0);
            }
        }
        for y in [0, 1, 5, 6] {
            assert!(image.get_pixel(15, y).0[3] > 0);
        }
        for y in [10, 11] {
            for x in 8..16 {
                assert_eq!(image.get_pixel(x, y).0[3], 0);
            }
        }
        assert_eq!(Glyph::Menu.path(), "kit/menu.svg");
        assert_eq!(Glyph::PanelRight.path(), "kit/panel-right.svg");
    }
    #[test]
    fn left_panel_is_registered_on_the_same_line_grid() {
        assert_eq!(
            view_box(Glyph::PanelLeft.svg()),
            Some([0.0, 0.0, 24.0, 24.0])
        );
        let bytes = Assets.load(Glyph::PanelLeft.path()).unwrap().unwrap();
        let tree = resvg::usvg::Tree::from_data(&bytes, &resvg::usvg::Options::default()).unwrap();
        let mut pixels = resvg::tiny_skia::Pixmap::new(24, 24).unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut pixels.as_mut(),
        );
        let image = image::RgbaImage::from_raw(24, 24, pixels.take()).unwrap();
        assert!(image.pixels().any(|pixel| pixel.0[3] > 0));
        for at in 0..24 {
            for (x, y) in [(at, 0), (at, 23), (0, at), (23, at)] {
                assert_eq!(image.get_pixel(x, y).0[3], 0);
            }
        }
    }
}

#[cfg(test)]
mod transfer_glyph_tests {
    use super::{Assets, Glyph, view_box};
    use gpui::AssetSource as _;

    #[test]
    fn transfer_glyphs_load_and_paint_unclipped_at_menu_and_action_sizes() {
        for glyph in [Glyph::Upload, Glyph::Download] {
            assert_eq!(view_box(glyph.svg()), Some([0.0, 0.0, 24.0, 24.0]));
            let bytes = Assets.load(glyph.path()).unwrap().unwrap();
            let tree =
                resvg::usvg::Tree::from_data(&bytes, &resvg::usvg::Options::default()).unwrap();
            for size in [14_u16, 16] {
                let side = u32::from(size);
                let scale = f32::from(size).mul_add(24.0_f32.recip(), 0.0);
                let mut pixels = resvg::tiny_skia::Pixmap::new(side, side).unwrap();
                resvg::render(
                    &tree,
                    resvg::tiny_skia::Transform::from_scale(scale, scale),
                    &mut pixels.as_mut(),
                );
                let image = image::RgbaImage::from_raw(side, side, pixels.take()).unwrap();
                assert!(image.pixels().any(|pixel| pixel.0[3] > 0));
                let edge = side.checked_sub(1).unwrap();
                for at in 0..side {
                    for (x, y) in [(at, 0), (at, edge), (0, at), (edge, at)] {
                        assert_eq!(image.get_pixel(x, y).0[3], 0);
                    }
                }
            }
        }
        assert_eq!(Glyph::ArrowUp.path(), "kit/arrow-up.svg");
        assert_eq!(Glyph::Update.path(), "kit/update.svg");
    }
}

#[cfg(test)]
mod diagnostics_glyph_tests {
    use super::{Assets, Glyph, view_box};
    use gpui::AssetSource as _;
    #[test]
    fn stethoscope_loads_and_clears_edges_at_both_sidebar_sizes() {
        let glyph = Glyph::Stethoscope;
        assert_eq!(glyph.path(), "kit/stethoscope.svg");
        assert_eq!(view_box(glyph.svg()), Some([0.0, 0.0, 24.0, 24.0]));
        assert_eq!(
            Assets.list(glyph.path()).unwrap(),
            vec![gpui::SharedString::from(glyph.path())]
        );
        let bytes = Assets.load(glyph.path()).unwrap().unwrap();
        let tree = resvg::usvg::Tree::from_data(&bytes, &resvg::usvg::Options::default()).unwrap();
        for size in [16_u16, 20, 24] {
            let side = u32::from(size);
            let mut pixels = resvg::tiny_skia::Pixmap::new(side, side).unwrap();
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::from_scale(
                    f32::from(size) / 24.0,
                    f32::from(size) / 24.0,
                ),
                &mut pixels.as_mut(),
            );
            let image = image::RgbaImage::from_raw(side, side, pixels.take()).unwrap();
            assert!(image.pixels().any(|pixel| pixel.0[3] > 0));
            let edge = side.checked_sub(1).unwrap();
            for at in 0..side {
                for (x, y) in [(at, 0), (at, edge), (0, at), (edge, at)] {
                    assert_eq!(image.get_pixel(x, y).0[3], 0);
                }
            }
        }
    }
}

#[cfg(test)]
mod settings_nav_glyph_tests {
    use super::{Assets, Glyph, view_box};
    use gpui::AssetSource as _;

    /// Settings' list draws these at 16 (pointer) and 20 (compact), as
    /// Paseo's `densityIconSize("md")`; each paints and keeps its rim clear.
    #[test]
    fn project_and_provider_glyphs_paint_unclipped_at_both_list_sizes() {
        for (glyph, path) in [
            (Glyph::FolderGit, "kit/folder-git.svg"),
            (Glyph::Boxes, "kit/boxes.svg"),
        ] {
            assert_eq!(glyph.path(), path);
            assert_eq!(view_box(glyph.svg()), Some([0.0, 0.0, 24.0, 24.0]));
            let bytes = Assets.load(glyph.path()).unwrap().unwrap();
            let tree =
                resvg::usvg::Tree::from_data(&bytes, &resvg::usvg::Options::default()).unwrap();
            for size in [16_u16, 20, 24] {
                let side = u32::from(size);
                let scale = f32::from(size).mul_add(24.0_f32.recip(), 0.0);
                let mut pixels = resvg::tiny_skia::Pixmap::new(side, side).unwrap();
                resvg::render(
                    &tree,
                    resvg::tiny_skia::Transform::from_scale(scale, scale),
                    &mut pixels.as_mut(),
                );
                let image = image::RgbaImage::from_raw(side, side, pixels.take()).unwrap();
                assert!(
                    image.pixels().any(|pixel| pixel.0[3] > 0),
                    "{path} at {size}"
                );
                let edge = side.checked_sub(1).unwrap();
                for at in 0..side {
                    for (x, y) in [(at, 0), (at, edge), (0, at), (edge, at)] {
                        assert_eq!(image.get_pixel(x, y).0[3], 0, "{path} at {size}");
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod activity_indicator_asset_tests {
    use super::{Assets, Glyph, view_box};
    use gpui::AssetSource as _;
    fn paint(dimension: u16, degrees: f32) -> image::RgbaImage {
        let bytes = Assets
            .load(Glyph::ActivityIndicator.path())
            .unwrap()
            .unwrap();
        let tree = resvg::usvg::Tree::from_data(&bytes, &resvg::usvg::Options::default()).unwrap();
        let side = u32::from(dimension);
        let mut pixels = resvg::tiny_skia::Pixmap::new(side, side).unwrap();
        let scale = f32::from(dimension).mul_add(32.0_f32.recip(), 0.0);
        let transform = resvg::tiny_skia::Transform::from_scale(scale, scale).pre_concat(
            resvg::tiny_skia::Transform::from_rotate_at(degrees, 16.0, 16.0),
        );
        resvg::render(&tree, transform, &mut pixels.as_mut());
        image::RgbaImage::from_raw(side, side, pixels.take()).unwrap()
    }
    #[test]
    fn activity_profile_keeps_alpha_and_foreground_arc_at_each_quarter_phase() {
        assert_eq!(
            Glyph::ActivityIndicator.path(),
            "kit/activity-indicator.svg"
        );
        assert_eq!(
            view_box(Glyph::ActivityIndicator.svg()),
            Some([0.0, 0.0, 32.0, 32.0])
        );
        assert_eq!(view_box(Glyph::Spinner.svg()), Some([0.0, 0.0, 24.0, 24.0]));
        for (phase, foreground, background) in [
            (0.0, (25, 25), (16, 2)),
            (90.0, (6, 25), (29, 16)),
            (180.0, (6, 6), (16, 29)),
            (270.0, (25, 6), (2, 16)),
        ] {
            let image = paint(32, phase);
            assert_eq!(image.get_pixel(foreground.0, foreground.1).0[3], 255);
            assert_eq!(image.get_pixel(background.0, background.1).0[3], 51);
            assert_eq!(image.get_pixel(16, 16).0[3], 0);
            assert_eq!(image.get_pixel(0, 0).0[3], 0);
        }
        for side in [10_u16, 16, 20, 36] {
            let image = paint(side, 0.0);
            assert_eq!(image.dimensions(), (u32::from(side), u32::from(side)));
            assert!(image.pixels().any(|pixel| pixel.0[3] == 255));
            assert!(image.pixels().any(|pixel| pixel.0[3] == 51));
        }
    }
}

#[cfg(test)]
mod added_navigation_tests;
#[cfg(test)]
mod voice_tests;
