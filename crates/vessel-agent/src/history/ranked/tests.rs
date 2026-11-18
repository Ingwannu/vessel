//! The ranked listing over a scripted agent, on a paused clock: what it
//! answers, what it leaves out and counts, and Paseo's budgets (a load at
//! most 30 s, all loads within 60 s of the first page, a close at most
//! 2 s), measured on tokio's clock rather than waited for.

use std::collections::HashMap;
use std::path::PathBuf;

use agent_client_protocol::v1::schema as acp;

use super::*;

const ACCOUNT: &str = "claude-code/default";

fn conversation(id: &str, at: Option<&str>) -> HistoryEntry {
    HistoryEntry {
        session: HarnessSessionId::new(id),
        cwd: PathBuf::from("/work/repo"),
        title: Some(format!("title {id}")),
        updated_at: at.map(str::to_owned),
        first_prompt: None,
        last_prompt: None,
    }
}

fn said(first: &str, last: &str) -> Prompts {
    Prompts {
        first: Some(first.to_owned()),
        last: Some(last.to_owned()),
        said: true,
    }
}

/// An agent whose pages and loads are written out: page `n` is asked with
/// cursor `n` (the first without); a load takes its time and answers its
/// prompts (one second and `first <id>` / `last <id>` unless written).
#[derive(Default)]
struct Script {
    pages: Vec<(Vec<HistoryEntry>, Option<String>)>,
    loads: HashMap<String, (Duration, Option<Prompts>)>,
    close_takes: Duration,
    asked: Vec<Option<String>>,
    loaded: Vec<String>,
    forgotten: Vec<String>,
    closed: Vec<String>,
}

impl Script {
    fn pages(pages: Vec<(Vec<HistoryEntry>, Option<&str>)>) -> Self {
        Self {
            pages: pages
                .into_iter()
                .map(|(entries, next)| (entries, next.map(str::to_owned)))
                .collect(),
            ..Self::default()
        }
    }

    fn loading(mut self, id: &str, takes: u64, prompts: Option<Prompts>) -> Self {
        let _earlier = self
            .loads
            .insert(id.to_owned(), (Duration::from_secs(takes), prompts));
        self
    }
}

impl Source for Script {
    async fn page(
        &mut self,
        cursor: Option<String>,
    ) -> Result<(Vec<HistoryEntry>, Option<String>), super::super::Unlisted> {
        self.asked.push(cursor.clone());
        let at = cursor.map_or(0, |cursor| cursor.parse::<usize>().unwrap());
        Ok(self.pages[at].clone())
    }

    async fn load(&mut self, entry: &HistoryEntry) -> Option<Prompts> {
        let id = entry.session.as_str().to_owned();
        self.loaded.push(id.clone());
        let (takes, prompts) = self.loads.get(&id).cloned().unwrap_or_else(|| {
            (
                Duration::from_secs(1),
                Some(said(&format!("first {id}"), &format!("last {id}"))),
            )
        });
        tokio::time::sleep(takes).await;
        prompts
    }

    fn forget(&mut self, entry: &HistoryEntry) {
        self.forgotten.push(entry.session.as_str().to_owned());
    }

    async fn close(&mut self, entry: &HistoryEntry) {
        self.closed.push(entry.session.as_str().to_owned());
        tokio::time::sleep(self.close_takes).await;
    }
}

fn ids(page: &RankedPage) -> Vec<&str> {
    page.entries
        .iter()
        .map(|entry| entry.session.as_str())
        .collect()
}

/// `elapsed` is `want` on the paused clock (whose timers fire on whole
/// milliseconds).
fn took(elapsed: Duration, want: Duration) {
    assert!(
        elapsed >= want && elapsed <= want.saturating_add(Duration::from_millis(5)),
        "took {elapsed:?}, not {want:?}"
    );
}

fn asked(limit: usize, query: Option<&str>) -> Ranked {
    Ranked::new(limit, query, Previews::default(), ACCOUNT.to_owned())
}

#[tokio::test(start_paused = true)]
async fn the_newest_not_imported_are_answered_with_their_prompts_and_the_rest_counted() {
    let mut agent = Script::pages(vec![
        (
            vec![
                conversation("a", Some("t3")),
                conversation("b", Some("t2")),
                conversation("c", Some("t1")),
            ],
            Some("1"),
        ),
        (vec![conversation("d", None)], None),
    ]);
    let ask = asked(2, None).without([HarnessSessionId::new("b")].into());
    let page = rank(&mut agent, &ask).await.unwrap();
    assert_eq!(ids(&page), ["a", "c"]);
    assert_eq!(page.already_imported, 1, "b is on the node");
    assert_eq!(page.entries[0].first_prompt.as_deref(), Some("first a"));
    assert_eq!(page.entries[0].last_prompt.as_deref(), Some("last a"));
    assert_eq!(agent.loaded, ["a", "c"], "an imported one is not loaded");
    assert_eq!(agent.closed, ["a", "c"]);
    assert_eq!(agent.forgotten, ["a", "c"]);
    assert_eq!(agent.asked, [None], "two rows were found on the first page");
}

#[tokio::test(start_paused = true)]
async fn pages_are_read_on_until_the_limit_and_the_end_is_the_end() {
    let mut agent = Script::pages(vec![
        (vec![conversation("a", None)], Some("1")),
        (Vec::new(), Some("2")),
        (vec![conversation("b", None)], None),
    ]);
    let page = rank(&mut agent, &asked(15, None)).await.unwrap();
    assert_eq!(ids(&page), ["a", "b"]);
    assert_eq!(
        agent.asked,
        [None, Some("1".to_owned()), Some("2".to_owned())]
    );
}

#[tokio::test(start_paused = true)]
async fn a_query_matches_the_title_the_prompts_or_the_folder_letter_case_aside() {
    let mut titled = conversation("titled", None);
    titled.title = Some("Fix the Build".to_owned());
    let mut foldered = conversation("foldered", None);
    foldered.cwd = PathBuf::from("/work/Widget");
    let prompted = conversation("prompted", None);
    let other = conversation("other", None);
    let script = || {
        Script::pages(vec![(
            vec![
                titled.clone(),
                foldered.clone(),
                prompted.clone(),
                other.clone(),
            ],
            None,
        )])
        .loading("prompted", 1, Some(said("deploy the APP", "and its tests")))
    };
    for (query, want) in [
        ("  build ", vec!["titled"]),
        ("WIDGET", vec!["foldered"]),
        ("app", vec!["prompted"]),
        ("ITS TESTS", vec!["prompted"]),
        ("nothing like it", vec![]),
    ] {
        let page = rank(&mut script(), &asked(15, Some(query))).await.unwrap();
        assert_eq!(ids(&page), want, "{query:?}");
    }
    assert_eq!(asked(15, Some("   ")).query(), None, "blank is no query");
}

#[tokio::test(start_paused = true)]
async fn an_empty_conversation_is_left_out_and_a_failed_load_keeps_its_row() {
    let mut agent = Script::pages(vec![(
        vec![
            conversation("empty", None),
            conversation("failed", None),
            conversation("fine", None),
        ],
        None,
    )])
    .loading("empty", 1, Some(Prompts::default()))
    .loading("failed", 1, None);
    let page = rank(&mut agent, &asked(15, None)).await.unwrap();
    assert_eq!(ids(&page), ["failed", "fine"]);
    assert_eq!(page.entries[0].first_prompt, None);
    assert_eq!(page.entries[1].first_prompt.as_deref(), Some("first fine"));
}

#[tokio::test(start_paused = true)]
async fn one_load_is_given_up_after_thirty_seconds_and_its_row_stays() {
    let mut agent = Script::pages(vec![(vec![conversation("slow", Some("t1"))], None)])
        .loading("slow", 45, None);
    let ask = asked(15, None);
    let started = Instant::now();
    let page = rank(&mut agent, &ask).await.unwrap();
    took(started.elapsed(), LOAD_LIMIT);
    assert_eq!(ids(&page), ["slow"]);
    assert_eq!(page.entries[0].first_prompt, None);
    assert_eq!(agent.forgotten, ["slow"], "its late replay is not gathered");
    assert_eq!(agent.closed, ["slow"]);
    assert_eq!(
        ask.previews.get(ACCOUNT, &conversation("slow", Some("t1"))),
        None,
        "a load given up on is not kept, so the next listing tries again"
    );
}

#[tokio::test(start_paused = true)]
async fn loads_stop_sixty_seconds_after_the_first_page_and_later_rows_have_no_prompts() {
    let entries = ["a", "b", "c", "d"]
        .map(|id| conversation(id, None))
        .to_vec();
    let mut agent = Script::pages(vec![(entries, None)])
        .loading("a", 25, Some(said("pa", "pa")))
        .loading("b", 25, Some(said("pb", "pb")))
        .loading("c", 25, Some(said("pc", "pc")));
    let started = Instant::now();
    let page = rank(&mut agent, &asked(15, None)).await.unwrap();
    // a: 0..25, b: 25..50, c: given up at 60 (10 s were left), d: none.
    took(started.elapsed(), PREVIEW_BUDGET);
    assert_eq!(agent.loaded, ["a", "b", "c"]);
    assert_eq!(ids(&page), ["a", "b", "c", "d"]);
    let firsts: Vec<_> = page
        .entries
        .iter()
        .map(|entry| entry.first_prompt.as_deref())
        .collect();
    assert_eq!(firsts, [Some("pa"), Some("pb"), None, None]);
    assert_eq!(
        agent.closed,
        ["a", "b"],
        "no close once the budget is spent"
    );
}

#[tokio::test(start_paused = true)]
async fn a_slow_close_is_left_after_two_seconds() {
    let mut agent = Script::pages(vec![(vec![conversation("a", None)], None)]);
    agent.close_takes = Duration::from_secs(20);
    let started = Instant::now();
    let page = rank(&mut agent, &asked(15, None)).await.unwrap();
    took(
        started.elapsed(),
        Duration::from_secs(1).saturating_add(CLOSE_LIMIT),
    );
    assert_eq!(page.entries[0].first_prompt.as_deref(), Some("first a"));
}

#[tokio::test(start_paused = true)]
async fn a_kept_preview_answers_until_the_conversation_changes() {
    let previews = Previews::default();
    let ask = Ranked::new(15, None, previews.clone(), ACCOUNT.to_owned());
    let mut first = Script::pages(vec![(vec![conversation("a", Some("t1"))], None)]);
    let _read = rank(&mut first, &ask).await.unwrap();
    assert_eq!(first.loaded, ["a"]);

    let mut again = Script::pages(vec![(vec![conversation("a", Some("t1"))], None)]);
    let page = rank(&mut again, &ask).await.unwrap();
    assert!(again.loaded.is_empty(), "kept while its time is the same");
    assert_eq!(page.entries[0].last_prompt.as_deref(), Some("last a"));

    let mut changed = Script::pages(vec![(vec![conversation("a", Some("t2"))], None)]);
    let _read = rank(&mut changed, &ask).await.unwrap();
    assert_eq!(
        changed.loaded,
        ["a"],
        "a conversation had since is read again"
    );

    let other = Ranked::new(15, None, previews, "claude-code/work".to_owned());
    let mut elsewhere = Script::pages(vec![(vec![conversation("a", Some("t2"))], None)]);
    let _read = rank(&mut elsewhere, &other).await.unwrap();
    assert_eq!(elsewhere.loaded, ["a"], "another account's is its own");

    let mut untimed = Script::pages(vec![(vec![conversation("u", None)], None)]);
    let _read = rank(&mut untimed, &ask).await.unwrap();
    let mut twice = Script::pages(vec![(vec![conversation("u", None)], None)]);
    let _read = rank(&mut twice, &ask).await.unwrap();
    assert_eq!(twice.loaded, ["u"], "without a time nothing is kept");
}

#[tokio::test(start_paused = true)]
async fn at_most_five_hundred_conversations_are_looked_through() {
    let entries: Vec<_> = (0..600)
        .map(|n| conversation(&format!("old-{n}"), None))
        .collect();
    let known = entries.iter().map(|entry| entry.session.clone()).collect();
    let mut agent = Script::pages(vec![(entries, Some("1"))]);
    let page = rank(&mut agent, &asked(15, None).without(known))
        .await
        .unwrap();
    assert!(page.entries.is_empty());
    assert_eq!(page.already_imported, SCAN);
    assert_eq!(agent.asked, [None], "the scan ended on the first page");
}

#[test]
fn a_listing_that_cannot_load_is_ranked_the_same_way_without_prompts() {
    let entries = ["a", "b", "c"].map(|id| conversation(id, None)).to_vec();
    let ask = asked(1, None).without([HarnessSessionId::new("a")].into());
    let page = unloaded(entries, &ask);
    assert_eq!(ids(&page), ["b"]);
    assert_eq!(page.already_imported, 1);
    assert_eq!(page.entries[0].first_prompt, None);
}

#[test]
fn a_preview_folds_its_blanks_and_stops_at_160_characters() {
    assert_eq!(
        preview("  fix\n\tthe   build  ").as_deref(),
        Some("fix the build")
    );
    assert_eq!(preview(" \n\t "), None);
    let long = "가".repeat(200);
    assert_eq!(preview(&long).unwrap().chars().count(), PREVIEW_CHARS);
}

fn user(session: &str, text: &str, id: Option<&str>) -> acp::SessionNotification {
    let mut chunk = acp::ContentChunk::new(acp::ContentBlock::Text(acp::TextContent::new(text)));
    chunk.message_id = id.map(acp::MessageId::new);
    acp::SessionNotification::new(
        acp::SessionId::new(session),
        acp::SessionUpdate::UserMessageChunk(chunk),
    )
}

fn agent_said(session: &str) -> acp::SessionNotification {
    acp::SessionNotification::new(
        acp::SessionId::new(session),
        acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk::new(acp::ContentBlock::Text(
            acp::TextContent::new("ok"),
        ))),
    )
}

#[test]
fn a_replay_gives_its_first_and_last_prompt_one_message_at_a_time() {
    let mut gathered = Gathered::default();
    gathered.begin("s");
    for notification in [
        user("s", "fix the ", Some("m1")),
        user("s", "build", Some("m1")),
        user("s", "then the tests", Some("m2")),
        agent_said("s"),
        user("s", "   ", None),
        user("s", "and ship", None),
        user("other", "not gathered", None),
    ] {
        gathered.accept(&notification);
    }
    assert_eq!(
        gathered.finish("s"),
        Prompts {
            first: Some("fix the build".to_owned()),
            last: Some("and ship".to_owned()),
            said: true,
        }
    );
    assert_eq!(gathered.finish("other"), Prompts::default());
}

#[test]
fn a_replay_of_nothing_said_is_empty_and_a_forgotten_one_gathers_nothing() {
    let mut gathered = Gathered::default();
    gathered.begin("quiet");
    gathered.accept(&acp::SessionNotification::new(
        acp::SessionId::new("quiet"),
        acp::SessionUpdate::AvailableCommandsUpdate(acp::AvailableCommandsUpdate::new(Vec::new())),
    ));
    assert!(!gathered.finish("quiet").said);

    gathered.begin("late");
    gathered.forget("late");
    gathered.accept(&user("late", "too late", None));
    assert_eq!(gathered.finish("late"), Prompts::default());
}
