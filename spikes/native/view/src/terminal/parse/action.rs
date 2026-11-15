//! What a program's bytes mean, as types: the decoder's output and the
//! grid's input (ADR 0073). Past `vte`'s callbacks nothing is a string
//! match or a raw parameter list, except SGR's groups, which the
//! attributes slice reads.

/// One thing the program asked the terminal to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Draw a character at the cursor.
    Print(char),
    /// A C0 control.
    Control(Control),
    /// A control sequence (`ESC [`).
    Csi(Csi),
    /// An escape sequence that is not a CSI, OSC or DCS.
    Esc(Esc),
    /// An operating-system command (`ESC ]`).
    Osc(Osc),
    /// A device control string (`ESC P`), its data dropped: none is drawn
    /// and none is answered (ADR 0073's closed set).
    Dcs(Dcs),
}

/// The C0 controls the grid acts on; the rest are dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// BEL.
    Bell,
    /// BS.
    Backspace,
    /// HT.
    Tab,
    /// LF, VT and FF: one line down.
    LineFeed,
    /// CR.
    CarriageReturn,
    /// ENQ: an answerback request, never answered (ADR 0073).
    Enquiry,
    /// Any other C0 byte (SO, SI and the rest are not drawn).
    Other,
}

impl Control {
    pub(super) const fn of(byte: u8) -> Self {
        match byte {
            0x07 => Self::Bell,
            0x08 => Self::Backspace,
            0x09 => Self::Tab,
            0x0a..=0x0c => Self::LineFeed,
            0x0d => Self::CarriageReturn,
            0x05 => Self::Enquiry,
            _ => Self::Other,
        }
    }
}

/// A count a sequence carries: at least 1 (0 and absent mean 1, as in
/// VT100), at most `u16::MAX` (the parser saturates).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Count(u16);

impl Count {
    /// One.
    pub const ONE: Self = Self(1);

    pub(super) const fn of(raw: u16) -> Self {
        if raw == 0 { Self::ONE } else { Self(raw) }
    }

    /// Its value, at least 1.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// A 1-based position a sequence carries (row or column); 0 and absent
/// mean 1.
pub type Position = Count;

/// Which part of the screen or line an erase covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Erase {
    /// From the cursor to the end.
    ToEnd,
    /// From the start to the cursor.
    ToStart,
    /// All of it.
    All,
    /// The scrollback too (`CSI 3 J`).
    Scrollback,
}

/// A mode a program sets or resets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// IRM (4): insert instead of replace.
    Insert,
    /// LNM (20): LF also returns the carriage.
    NewLine,
    /// DECCKM (`?1`): cursor keys in application mode.
    CursorKeys,
    /// DECOM (`?6`): positions relative to the scroll region.
    Origin,
    /// DECAWM (`?7`): wrap at the right margin.
    AutoWrap,
    /// DECTCEM (`?25`): the cursor is shown.
    ShowCursor,
    /// `?47`, `?1047`: the alternate screen.
    AltScreen,
    /// `?1048`: save or restore the cursor.
    SaveCursor,
    /// `?1049`: save the cursor and use a cleared alternate screen.
    AltScreenSaveCursor,
    /// `?2004`: bracketed paste.
    BracketedPaste,
    /// Any other mode (mouse reports, focus events, blink): not kept.
    Other,
}

/// Set or reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Switch {
    /// `h`.
    Set,
    /// `l`.
    Reset,
}

/// A device-status request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// `CSI 5 n`: "are you there".
    Operating,
    /// `CSI 6 n`: where the cursor is.
    Cursor,
    /// `CSI ? 6 n`: where the cursor is, private form.
    PrivateCursor,
}

/// A device-attributes request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attributes {
    /// `CSI c`, `CSI 0 c`: primary, answered.
    Primary,
    /// `CSI > c` or `CSI = c` (secondary, tertiary) or a primary with
    /// another parameter: never answered.
    Other,
}

/// One SGR group: a parameter and its colon sub-parameters (`38:2:r:g:b`
/// is one group of five). At most `vte`'s 32 values in all.
pub type Group = Vec<u16>;

/// A control sequence, typed. One that is not listed is `Unsupported`
/// and is dropped (the parser's rule, not a dropped command).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Csi {
    /// CUU `A`.
    Up(Count),
    /// CUD `B`, VPR `e`.
    Down(Count),
    /// CUF `C`, HPR `a`.
    Forward(Count),
    /// CUB `D`.
    Back(Count),
    /// CNL `E`.
    NextLine(Count),
    /// CPL `F`.
    PreviousLine(Count),
    /// CHA `G`, HPA `` ` ``.
    Column(Position),
    /// CUP `H`, HVP `f`: row, column.
    Goto(Position, Position),
    /// VPA `d`.
    Row(Position),
    /// ED `J`.
    EraseDisplay(Erase),
    /// EL `K`.
    EraseLine(Erase),
    /// IL `L`.
    InsertLines(Count),
    /// DL `M`.
    DeleteLines(Count),
    /// ICH `@`.
    InsertChars(Count),
    /// DCH `P`.
    DeleteChars(Count),
    /// ECH `X`.
    EraseChars(Count),
    /// SU `S`.
    ScrollUp(Count),
    /// SD `T`.
    ScrollDown(Count),
    /// CHT `I`: forward tab stops.
    TabForward(Count),
    /// CBT `Z`: back tab stops.
    TabBack(Count),
    /// TBC `g`: 0 clears the stop at the cursor, 3 every stop.
    ClearTabs(TabClear),
    /// DECSTBM `r`: top and bottom, 1-based; absent is the whole screen.
    ScrollRegion(Option<Position>, Option<Position>),
    /// SM, RM, DECSET, DECRST: each mode named in the sequence.
    Modes(Switch, Vec<Mode>),
    /// SGR `m`.
    Graphic(Vec<Group>),
    /// DSR `n`.
    Status(Status),
    /// DA `c`.
    Attributes(Attributes),
    /// SCOSC `s` (no parameters).
    SaveCursor,
    /// SCORC `u`.
    RestoreCursor,
    /// DECSCUSR `SP q`: the cursor's shape (kept by nobody: the app draws
    /// Paseo's bar).
    CursorStyle,
    /// Window operations `t` (title and size reports among them), DECRQM
    /// `$ p` and any other query: never answered (ADR 0073).
    RefusedQuery,
    /// Anything else.
    Unsupported,
}

/// What TBC clears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabClear {
    /// The stop at the cursor.
    Here,
    /// Every stop.
    All,
}

/// An escape sequence, typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Esc {
    /// IND `ESC D`.
    Index,
    /// NEL `ESC E`.
    NextLine,
    /// HTS `ESC H`.
    SetTab,
    /// RI `ESC M`.
    ReverseIndex,
    /// DECSC `ESC 7`.
    SaveCursor,
    /// DECRC `ESC 8`.
    RestoreCursor,
    /// RIS `ESC c`.
    Reset,
    /// DECKPAM `ESC =`, DECKPNM `ESC >`, charset designations and the
    /// rest: nothing the grid draws.
    Other,
}

/// An operating-system command, typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Osc {
    /// OSC 0 or 2: the window's title, from at most `OSC_LIMIT` bytes as
    /// lossy UTF-8 (so at most three times that); OSC 1, the icon's name,
    /// is dropped.
    Title(String),
    /// OSC 10, 11 or 12 whose argument is exactly `?`: answered with the
    /// colour drawn (ADR 0073).
    ColourQuery(ColourSlot),
    /// OSC 52 with `?` (a clipboard read): never answered.
    ClipboardRead,
    /// Anything else (colour sets, hyperlinks, clipboard writes, odd
    /// arguments): dropped.
    Other,
}

/// Which colour an OSC colour query names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColourSlot {
    /// OSC 10.
    Foreground,
    /// OSC 11.
    Background,
    /// OSC 12.
    Cursor,
}

/// A device control string, by what it asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dcs {
    /// DECRQSS `DCS $ q`: never answered.
    RequestSetting,
    /// XTGETTCAP `DCS + q`: never answered.
    TermcapQuery,
    /// Sixel and the rest: not drawn.
    #[default]
    Other,
}
