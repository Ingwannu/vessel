// Canonical declaration order is also ALL order; append future glyphs here.
// Documentation stays on each generated public variant. Vector ownership
// remains in drawing.rs, whose exhaustive match must handle every new glyph.
glyph_inventory! {
    /// Vessel's mark.
    Mark => "kit/mark.svg",
    /// The window's minimise button.
    Minimize => "kit/minimize.svg",
    /// Maximise.
    Maximize => "kit/maximize.svg",
    /// Restore, when maximised.
    Restore => "kit/restore.svg",
    /// Close.
    Close => "kit/close.svg",
    /// An update: an arrow down into a tray (the title bar's update
    /// status at a phone's width).
    Update => "kit/update.svg",
    /// Three lines: opens the sidebar's sheet at a phone's width.
    Menu => "kit/menu.svg",
    /// A picker's caret.
    ChevronDown => "kit/chevron-down.svg",
    /// Up: the find bar's previous match.
    ChevronUp => "kit/chevron-up.svg",
    /// A row that opens a list of its own (a provider's models).
    ChevronRight => "kit/chevron-right.svg",
    /// Back, from such a list.
    ChevronLeft => "kit/chevron-left.svg",
    /// The chosen row's mark.
    Check => "kit/check.svg",
    /// A search row.
    Search => "kit/search.svg",
    /// Send.
    ArrowUp => "kit/arrow-up.svg",
    /// Outward link arrow, consumed at12 by `ExternalLink`.
    ArrowUpRight => "kit/arrow-up-right.svg",
    /// Stop.
    Stop => "kit/stop.svg",
    /// The permissions mode.
    Shield => "kit/shield.svg",
    /// A permissions mode that accepts edits (Paseo's `ShieldPlus`).
    ShieldPlus => "kit/shield-plus.svg",
    /// A permissions mode that decides for itself (Paseo's `ShieldCheck`).
    ShieldCheck => "kit/shield-check.svg",
    /// A permissions mode that asks nothing (Paseo's `ShieldOff`).
    ShieldOff => "kit/shield-off.svg",
    /// A planning mode (Paseo's `ShieldEllipsis`).
    ShieldEllipsis => "kit/shield-ellipsis.svg",
    /// A permissions mode that asks before writing (Paseo's `ShieldAlert`).
    ShieldAlert => "kit/shield-alert.svg",
    /// The effort (thinking).
    Brain => "kit/brain.svg",
    /// An agent.
    Bot => "kit/bot.svg",
    /// A failure.
    Alert => "kit/alert.svg",
    /// A success alert: a check inside a circle, drawn on the kit's own grid.
    CircleCheck => "kit/circle-check.svg",
    /// An error alert: a cross inside a circle, drawn on the kit's own grid.
    CircleX => "kit/circle-x.svg",
    /// A task not started (Paseo's `Circle`): a ring on the same grid as
    /// [`Self::CircleCheck`].
    Circle => "kit/circle.svg",
    /// A task under way (Paseo's `CircleDot`): the ring with a dot.
    CircleDot => "kit/circle-dot.svg",
    /// A task list (Paseo's `CheckSquare`): a check in a rounded square.
    SquareCheck => "kit/square-check.svg",
    /// Scheduling: a calendar with a clock, on the shared 24-unit line grid.
    /// One asset serves navigation and empty states; no screen draws a second.
    CalendarClock => "kit/calendar-clock.svg",
    /// A folder.
    Folder => "kit/folder.svg",
    /// A project: a folder holding a branch (Paseo's `FolderGit2`, an
    /// original drawing on the shared 24-unit line grid).
    FolderGit => "kit/folder-git.svg",
    /// The agents a machine can run: three stacked boxes (Paseo's
    /// `Boxes`, an original drawing on the shared 24-unit line grid).
    Boxes => "kit/boxes.svg",
    /// A machine.
    Machine => "kit/machine.svg",
    /// Three dots: a menu of more actions (the Session actions).
    More => "kit/more.svg",
    /// An agent's feature, and the features (Paseo's `Settings2`: two
    /// sliders).
    Sliders => "kit/sliders.svg",
    /// Settings (Paseo's `Settings`: a gear).
    Gear => "kit/gear.svg",
    /// Copy: two sheets, one over the other (a code block's copy).
    Copy => "kit/copy.svg",
    /// Appearance (a painter's palette, Paseo's `Palette`).
    Palette => "kit/palette.svg",
    /// Diagnostics: an original stethoscope on the shared 24-unit line grid.
    Stethoscope => "kit/stethoscope.svg",
    /// Shortcuts (a keyboard, Paseo's `Keyboard`).
    Keyboard => "kit/keyboard.svg",
    /// Notifications (a bell, Paseo's `Bell`).
    Bell => "kit/bell.svg",
    /// Models (a spark, Paseo's `Sparkles`).
    Sparkles => "kit/sparkles.svg",
    /// A machine as Settings lists it (Paseo's `Server`).
    Server => "kit/server.svg",
    /// Devices (a phone, Paseo's `Smartphone`).
    Smartphone => "kit/smartphone.svg",
    /// Sync (two arrows round).
    Refresh => "kit/refresh.svg",
    /// About (an i in a circle, Paseo's `Info`).
    Info => "kit/info.svg",
    /// Back (Settings' way back to the workspace, Paseo's `ArrowLeft`).
    ArrowLeft => "kit/arrow-left.svg",
    /// Adding (a machine, Paseo's `Plus`).
    Plus => "kit/plus.svg",
    /// A branch (a new worktree, a starting ref: Paseo's `GitBranch`).
    Branch => "kit/branch.svg",
    /// A pull request (Paseo's `GitPullRequest`).
    PullRequest => "kit/pull-request.svg",
    /// A draft pull request (Paseo's `GitPullRequestDraft`).
    PullRequestDraft => "kit/pull-request-draft.svg",
    /// A closed pull request (Paseo's `GitPullRequestClosed`).
    PullRequestClosed => "kit/pull-request-closed.svg",
    /// A merged pull request (Paseo's `GitMerge`).
    GitMerge => "kit/git-merge.svg",
    /// Fast mode, off: a lightning bolt's outline.
    Lightning => "kit/lightning.svg",
    /// Fast mode, on: the bolt filled.
    LightningFilled => "kit/lightning-filled.svg",
    /// Back to the default: an arrow turning back.
    Reset => "kit/reset.svg",
    /// A terminal session (a prompt's chevron and a line).
    Terminal => "kit/terminal.svg",
    /// A conversation (a speech bubble).
    Message => "kit/message.svg",
    /// The archive (a box with its lid).
    Archive => "kit/archive.svg",
    /// Delete for good.
    Trash => "kit/trash.svg",
    /// A delete action with the two interior bin strokes (legacy Trash stays).
    Trash2 => "kit/trash-2.svg",
    /// Restore an archived item: the archive with its upward arrow.
    ArchiveRestore => "kit/archive-restore.svg",
    /// A horizontal minus on the shared line grid.
    Minus => "kit/minus.svg",
    /// Create another branch, with the plus beside the branch's nodes.
    GitBranchPlus => "kit/git-branch-plus.svg",
    /// A commit node on its history line.
    Commit => "kit/commit.svg",
    /// Push: an upward transfer arrow and an open tray.
    Upload => "kit/upload.svg",
    /// Pull: a downward transfer arrow and an open tray.
    Download => "kit/download.svg",
    /// Undo a file change: a returning arrow, separate from Reset.
    Undo2 => "kit/undo-2.svg",
    /// Fork a conversation: one way parting in two (Paseo's `Split`, its
    /// fork menu).
    Split => "kit/split.svg",
    /// Pinned.
    Pin => "kit/pin.svg",
    /// A vertical kebab, for a row's actions (horizontal More stays unchanged).
    MoreVertical => "kit/more-vertical.svg",
    /// Pause a recurring action.
    Pause => "kit/pause.svg",
    /// Resume an action.
    Play => "kit/play.svg",
    /// Run again now: one clockwise rotation arrow.
    RotateCw => "kit/rotate-cw.svg",
    /// Original combined activity asset, retained for compatibility. Use the
    /// loading control to apply track alpha after monochrome mask correction.
    ActivityIndicator => "kit/activity-indicator.svg",
    /// Opaque activity track; its renderer applies the source alpha after masking.
    ActivityTrack => "kit/activity-track.svg",
    /// Opaque activity foreground, rotated independently of its invariant track.
    ActivityArc => "kit/activity-arc.svg",
    /// Work going on: three quarters of a ring, turned by `spinner`.
    Spinner => "kit/spinner.svg",
    /// The board (three lanes of cards).
    Board => "kit/board.svg",
    /// The light theme (a sun).
    Sun => "kit/sun.svg",
    /// The dark theme (a moon).
    Moon => "kit/moon.svg",
    /// The system's theme (a screen).
    Monitor => "kit/monitor.svg",
    /// A pencil: edit (a queued message).
    Pencil => "kit/pencil.svg",
    /// An arrow turning up and right: send into the running turn (GOAT's
    /// steer, `CornerUpRight`).
    Steer => "kit/steer.svg",
    /// Six dots: a handle to drag a row by.
    Grip => "kit/grip.svg",
    /// An X on the line glyphs' grid: take a row out (a queued message;
    /// the window's `Close` fills its box and reads heavier beside them).
    Remove => "kit/remove.svg",
    /// A file (the Files tree, Paseo's `File`).
    File => "kit/file.svg",
    /// A file with lines of text: an attached file's pill (Paseo's
    /// `FileText`).
    FileText => "kit/file-text.svg",
    /// The code panel's toggle (Paseo's `PanelRight`).
    PanelRight => "kit/panel-right.svg",
    /// The desktop sidebar toggle, preserving the existing right-panel glyph.
    PanelLeft => "kit/panel-left.svg",
    /// The compact sidebar's two full bars and one short bar, painted 16 by 12.
    MenuCompact => "kit/menu-compact.svg",
    /// A usage dial, on the shared24-unit line grid.
    Gauge => "kit/gauge.svg",
    /// A catalog package with its add action.
    PackagePlus => "kit/package-plus.svg",
    /// A tool's generic activity (Lucide's `Wrench`).
    Wrench => "kit/wrench.svg",
    /// Reading a file (Lucide's `Eye`).
    Eye => "kit/eye.svg",
    /// Voice tool activity (Lucide's `MicVocal`).
    MicVocal => "kit/mic-vocal.svg",
    /// Context compaction (Lucide's `Scissors`).
    Scissors => "kit/scissors.svg",
}
