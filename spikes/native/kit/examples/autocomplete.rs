//! Visual check of `kit::autocomplete`, not an app route: the list as it
//! sits over a composer, in the scene `VESSEL_AUTOCOMPLETE_SCENE` names
//! (`commands`, `long`, `files`, `empty`, `loading`, `failed`, `scroll`;
//! default `commands`) and the theme `VESSEL_APP_THEME` names (`light`,
//! default dark). A click moves the choice, as in the app.
use gpui::{
    AppContext as _, Bounds, Context, IntoElement, ParentElement as _, Render, SharedString,
    Styled as _, Window, WindowOptions, div, point, px, size,
};
use vessel_app_kit::autocomplete::{
    AutocompleteRow, AutocompleteState, Listing, RowKind, autocomplete,
};
use vessel_app_kit::{Assets, Scheme, SchemeChoice, Tokens};

/// The window, as wide as a phone's composer card plus a margin.
const WIDTH: f32 = 420.0;
const HEIGHT: f32 = 400.0;

struct Sheet {
    scheme: Scheme,
    listing: Listing,
    chosen: Option<usize>,
    state: AutocompleteState,
}

fn command(name: &str, description: &str, hint: Option<&str>) -> AutocompleteRow {
    AutocompleteRow {
        label: SharedString::from(format!("/{name}")),
        detail: hint.map(|hint| SharedString::from(hint.to_owned())),
        description: (!description.is_empty()).then(|| SharedString::from(description.to_owned())),
        kind: RowKind::Command,
    }
}

fn file(path: &str) -> AutocompleteRow {
    AutocompleteRow {
        label: SharedString::from(path.to_owned()),
        detail: None,
        description: None,
        kind: RowKind::File,
    }
}

/// The rows of a scene, drawn order (the best nearest the composer, last).
fn scene(name: &str) -> (Listing, Option<usize>) {
    let rows = |rows: Vec<AutocompleteRow>| {
        let last = rows.len().checked_sub(1);
        (Listing::Rows(rows), last)
    };
    match name {
        "long" => rows(vec![
            command(
                "review-pull-request-with-every-check-the-repository-defines",
                "Reviews the pull request on this branch against the repository's own rules, runs every check it defines and writes the findings as comments a person can act on",
                Some("[pull request number or branch]"),
            ),
            command(
                "검토",
                "현재 브랜치의 변경을 저장소 규칙에 맞춰 검토하고, 고칠 점을 한국어로 정리합니다. 설명이 길면 한 줄에서 잘립니다.",
                Some("[브랜치 이름]"),
            ),
            command(
                "compact",
                "Clear conversation history but keep a summary in context",
                Some("<optional custom summarization instructions>"),
            ),
        ]),
        "files" => rows(vec![
            file("crates/vessel-workspace/src/find.rs"),
            file("spikes/native/app/src/areas/composer/running.rs"),
            file(
                "docs/plans/a-very-long-folder-name-that-does-not-fit/and-another-one-inside-it/the-file-at-the-end-of-it.md",
            ),
            file("README.md"),
        ]),
        "empty" => (Listing::Empty("No commands found".into()), None),
        "loading" => (Listing::Loading("Searching workspace...".into()), None),
        "failed" => (
            Listing::Failed("The files could not be searched: the machine did not answer.".into()),
            None,
        ),
        "scroll" => rows(
            (1..=30)
                .map(|index| {
                    command(
                        &format!("command-{index:02}"),
                        "One of thirty commands",
                        None,
                    )
                })
                .collect(),
        ),
        _ => rows(vec![
            command(
                "init",
                "Initialize a new CLAUDE.md file with codebase documentation",
                None,
            ),
            command("review", "Review a pull request", Some("[pr number]")),
            command(
                "pr-comments",
                "Get comments from a GitHub pull request",
                None,
            ),
            command(
                "compact",
                "Clear conversation history but keep a summary in context",
                Some("<optional custom summarization instructions>"),
            ),
        ]),
    }
}

impl Sheet {
    fn choose(&mut self, index: usize, _window: &mut Window, cx: &mut Context<'_, Self>) {
        self.chosen = Some(index);
        cx.notify();
    }
}

impl Render for Sheet {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(self.scheme);
        // The list sits 12 over where a composer's card would start, as
        // wide as the card.
        div()
            .size_full()
            .relative()
            .bg(tokens.bg_base)
            .child(
                div()
                    .absolute()
                    .left(px(16.0))
                    .right(px(16.0))
                    .bottom(px(72.0))
                    .child(autocomplete(
                        "example-autocomplete",
                        &self.listing,
                        self.chosen,
                        &self.state,
                        Self::choose,
                        vessel_app_kit::autocomplete::Motion::NONE,
                        &tokens,
                        cx,
                    )),
            )
            .child(
                div()
                    .absolute()
                    .left(px(16.0))
                    .right(px(16.0))
                    .bottom(px(16.0))
                    .h(px(44.0))
                    .rounded(px(16.0))
                    .border_1()
                    .border_color(tokens.neutral.border_accent)
                    .bg(tokens.neutral.surface1),
            )
    }
}

fn main() {
    let scheme = if std::env::var("VESSEL_APP_THEME").as_deref() == Ok("light") {
        Scheme::Light
    } else {
        Scheme::Dark
    };
    let name = std::env::var("VESSEL_AUTOCOMPLETE_SCENE").unwrap_or_default();
    gpui_platform::application()
        .with_assets(Assets)
        .run(move |cx| {
            if let Err(error) = vessel_app_kit::init(cx, SchemeChoice::Fixed(scheme)) {
                tracing::error!(%error, "autocomplete example fonts could not start");
                cx.quit();
                return;
            }
            let options = WindowOptions {
                window_bounds: Some(gpui::WindowBounds::Windowed(Bounds {
                    origin: point(px(0.0), px(0.0)),
                    size: size(px(WIDTH), px(HEIGHT)),
                })),
                ..WindowOptions::default()
            };
            let (listing, chosen) = scene(&name);
            if let Err(error) = cx.open_window(options, move |_window, cx| {
                cx.new(|_cx| Sheet {
                    scheme,
                    listing,
                    chosen,
                    state: AutocompleteState::new(),
                })
            }) {
                tracing::error!(%error, "autocomplete example window could not start");
                cx.quit();
            }
        });
}
