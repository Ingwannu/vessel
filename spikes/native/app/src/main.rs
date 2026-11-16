//! Control-only native audit surface, never a product App route or node client.
//! Synthetic input is compared privately; only closed observations reach disk.
use gpui::prelude::*;
use gpui::{
    Bounds, ClipboardItem, Context, KeyDownEvent, Render, Window, WindowOptions, div, point, px,
    size,
};
use kit::field::{FieldId, SecretEdit, SecretField, SecretFieldContent, SecretPresence, TextField};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use vessel_app_kit::{
    self as kit, Assets, Editability, Scheme, SchemeChoice, Tokens, Type, TypeExt as _,
    theme::spacing,
};
const CANARY: &str = "synthetic-secure-entry-canary";
const SENTINEL: &str = "clipboard-sentinel";
const REFERENCE: &str = "ordinary-clipboard-reference";
const WINDOW_WIDTH: f32 = 900.0;
const WINDOW_HEIGHT: f32 = 620.0;
#[derive(Clone, Copy, Debug, Default)]
enum Observation {
    #[default]
    Unobserved,
    Passed,
    Failed,
}
impl Observation {
    fn of(value: bool) -> Self {
        if value { Self::Passed } else { Self::Failed }
    }
    fn word(self) -> &'static str {
        match self {
            Self::Unobserved => "unobserved",
            Self::Passed => "passed",
            Self::Failed => "failed",
        }
    }
}
#[derive(Clone, Copy)]
enum Slot {
    Absent,
    Stored,
}
#[derive(Clone, Copy)]
enum EventKind {
    Change,
    Submit,
}
#[derive(Clone, Copy, Default)]
enum SubmitPhase {
    #[default]
    Ordinary,
    AfterReadOnly,
}
#[derive(Default)]
struct Metrics {
    replaced: Observation,
    submitted: Observation,
    redacted: Observation,
    copy: Observation,
    cut: Observation,
    erased: Observation,
    held_edits: u32,
    readonly_retained: Observation,
    reference_copy: Observation,
}
struct Probe {
    absent: SecretField,
    stored: SecretField,
    reference: TextField,
    slot: Slot,
    scheme: Scheme,
    editability: Editability,
    phase: SubmitPhase,
    clipboard_step: Slot,
    metrics: Metrics,
    output: PathBuf,
}
impl Probe {
    fn new(
        scheme: Scheme,
        output: PathBuf,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let field = |presence, window: &mut Window, cx: &mut Context<'_, Self>| {
            SecretField::new(
                SecretFieldContent {
                    label: "API key audit".into(),
                    empty: "Enter replacement".into(),
                    set: "Set".into(),
                    presence,
                },
                Self::submit,
                Self::changed,
                window,
                cx,
            )
        };
        let absent = field(SecretPresence::Absent, window, cx);
        let stored = field(SecretPresence::Set, window, cx);
        let reference = TextField::new("", |_, _, _, _| {}, window, cx);
        reference.set_text(REFERENCE, window, cx);
        absent.focus(window, cx);
        Self {
            absent,
            stored,
            reference,
            slot: Slot::Absent,
            scheme,
            editability: Editability::Editable,
            phase: SubmitPhase::Ordinary,
            clipboard_step: Slot::Absent,
            metrics: Metrics::default(),
            output,
        }
    }
    fn field(&self) -> &SecretField {
        match self.slot {
            Slot::Absent => &self.absent,
            Slot::Stored => &self.stored,
        }
    }
    fn observe(&mut self, source: FieldId, edit: SecretEdit, kind: EventKind) {
        if source != self.absent.id() && source != self.stored.id() {
            return;
        }
        self.metrics.redacted = Observation::of(!format!("{edit:?}").contains(CANARY));
        match edit {
            SecretEdit::Unchanged => {
                self.metrics.erased = Observation::Passed;
                if let EventKind::Submit = kind {
                    match self.phase {
                        SubmitPhase::Ordinary => self.metrics.submitted = Observation::Failed,
                        SubmitPhase::AfterReadOnly => {
                            self.metrics.readonly_retained = Observation::Failed;
                            self.phase = SubmitPhase::Ordinary;
                        }
                    }
                }
            }
            SecretEdit::Replace(value) => {
                let redacted = !format!("{value:?}").contains(CANARY);
                self.metrics.redacted = Observation::of(redacted);
                let exact = value.into_value() == CANARY;
                match kind {
                    EventKind::Submit => match self.phase {
                        SubmitPhase::Ordinary => self.metrics.submitted = Observation::of(exact),
                        SubmitPhase::AfterReadOnly => {
                            self.metrics.readonly_retained = Observation::of(exact);
                            self.phase = SubmitPhase::Ordinary;
                        }
                    },
                    EventKind::Change => self.metrics.replaced = Observation::of(exact),
                }
            }
        }
    }
    fn submit(
        &mut self,
        id: FieldId,
        edit: SecretEdit,
        _: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.observe(id, edit, EventKind::Submit);
        cx.notify();
    }
    fn changed(&mut self, id: FieldId, edit: SecretEdit, cx: &mut Context<'_, Self>) {
        if self.editability == Editability::ReadOnly {
            self.metrics.held_edits = self.metrics.held_edits.saturating_add(1);
        }
        self.observe(id, edit, EventKind::Change);
        cx.notify();
    }
    fn key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<'_, Self>) {
        match event.keystroke.key.as_str() {
            "f1" => {
                self.slot = Slot::Absent;
                self.absent.focus(window, cx);
            }
            "f4" => {
                self.slot = Slot::Stored;
                self.stored.focus(window, cx);
            }
            "f5" => self.reference.focus(window, cx),
            "f11" => {
                self.metrics.reference_copy = Observation::of(
                    cx.read_from_clipboard()
                        .and_then(|item| item.text())
                        .is_some_and(|text| text == REFERENCE),
                );
            }
            "f2" => cx.write_to_clipboard(ClipboardItem::new_string(CANARY.to_owned())),
            "f3" => cx.write_to_clipboard(ClipboardItem::new_string(SENTINEL.to_owned())),
            "f6" => {
                let kept = cx
                    .read_from_clipboard()
                    .and_then(|item| item.text())
                    .is_some_and(|text| text == SENTINEL);
                match self.clipboard_step {
                    Slot::Absent => {
                        self.metrics.copy = Observation::of(kept);
                        self.clipboard_step = Slot::Stored;
                    }
                    Slot::Stored => self.metrics.cut = Observation::of(kept),
                }
            }
            "f7" => self.hold(Editability::ReadOnly, cx),
            "f8" => self.hold(Editability::Editable, cx),
            "f9" => self.field().clear(window, cx),
            "f10" => {
                if let Err(error) = self.record() {
                    tracing::error!(%error, "audit status could not be written");
                }
            }
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }
    fn hold(&mut self, mode: Editability, cx: &mut Context<'_, Self>) {
        self.editability = mode;
        if mode == Editability::ReadOnly {
            self.phase = SubmitPhase::AfterReadOnly;
            self.metrics.readonly_retained = Observation::Unobserved;
        }
        self.absent.set_editability(mode, cx);
        self.stored.set_editability(mode, cx);
    }
    fn record(&self) -> std::io::Result<()> {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let mut out = options.open(&self.output)?;
        for (name, value) in [
            ("replacement", self.metrics.replaced),
            ("submit", self.metrics.submitted),
            ("debug_redacted", self.metrics.redacted),
            ("copy_sentinel", self.metrics.copy),
            ("cut_sentinel", self.metrics.cut),
            ("erased_unchanged", self.metrics.erased),
            ("readonly_retained", self.metrics.readonly_retained),
            ("ordinary_copy_reference", self.metrics.reference_copy),
        ] {
            writeln!(out, "{name}\t{}", value.word())?;
        }
        writeln!(out, "held_edits\t{}", self.metrics.held_edits)?;
        out.sync_all()
    }
}
impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(self.scheme);
        let compact = kit::breakpoint::phone(window);
        div().id("secure-field-native-audit").size_full().flex().flex_col().gap(px(spacing::S4))
            .p(px(spacing::S6)).bg(tokens.neutral.surface0).type_(Type::Body).text_color(tokens.neutral.foreground)
            .on_key_down(cx.listener(Self::key))
            .child("Secure field audit — synthetic input only")
            .child("No stored credential loaded")
            .child(self.absent.element(&tokens, compact, cx))
            .child("Stored presence (replacement initially empty)")
            .child(self.stored.element(&tokens, compact, cx))
            .child("Ordinary copy reference (nonsecret negative control)")
            .child(self.reference.element(&tokens, compact, cx))
            .child(div().type_(Type::Meta).whitespace_normal().child("F1 absent · F4 stored · F2 seed clipboard · F3 sentinel · F6 check · F7 hold · F8 edit · F9 clear · F10 record"))
            .child(format!("replacement: {} · submit: {}", self.metrics.replaced.word(), self.metrics.submitted.word()))
            .child(format!("copy: {} · cut: {}", self.metrics.copy.word(), self.metrics.cut.word()))
            .child(format!("redacted: {} · held edits: {}", self.metrics.redacted.word(), self.metrics.held_edits))
    }
}
fn output() -> Option<PathBuf> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--evidence")) {
        return None;
    }
    let path = PathBuf::from(args.next()?);
    (path.is_absolute() && path.parent().is_some_and(Path::is_dir) && args.next().is_none())
        .then_some(path)
}
fn main() {
    let Some(output) = output() else {
        return;
    };
    let scheme = if std::env::var("VESSEL_APP_THEME").as_deref() == Ok("light") {
        Scheme::Light
    } else {
        Scheme::Dark
    };
    gpui_platform::application()
        .with_assets(Assets)
        .run(move |cx| {
            if let Err(error) = kit::init(cx, SchemeChoice::Fixed(scheme)) {
                tracing::error!(%error, "audit initialization failed");
                cx.quit();
                return;
            }
            let options = WindowOptions {
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Vessel secure field audit".into()),
                    ..gpui::TitlebarOptions::default()
                }),
                window_bounds: Some(gpui::WindowBounds::Windowed(Bounds {
                    origin: point(px(0.0), px(0.0)),
                    size: size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)),
                })),
                ..WindowOptions::default()
            };
            if let Err(error) = cx.open_window(options, |window, cx| {
                cx.new(|cx| Probe::new(scheme, output, window, cx))
            }) {
                tracing::error!(%error, "audit window could not open");
                cx.quit();
            }
        });
}
