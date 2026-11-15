//! A list reply may initiate work only for its accepted, still-visible origin.
use super::super::testing::{self, COMMAND_CAPACITY};
use crate::shell::Shell;
use gpui::TestAppContext;
use vessel_app_link::{
    Command, Link,
    agents::{Answer, Ask, CatalogueFetch},
};
use vessel_app_view::agent_status::pages::{MachineKey, Operation};

#[gpui::test]
fn an_accepted_old_machines_list_cannot_start_the_new_machines_registry(cx: &mut TestAppContext) {
    testing::init(cx);
    let (link, mut commands) = Link::detached(COMMAND_CAPACITY).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, _, cx| {
        testing::select(shell, "one", Vec::new(), cx);
        let one = MachineKey::from_book("one");
        let read = shell
            .areas
            .settings
            .agents
            .pages
            .begin(one.clone(), Operation::Discovery)
            .unwrap();
        testing::select(shell, "two", vec![testing::added("retained-two")], cx);
        shell.agents_answer(
            &one,
            read,
            Ok(Answer::List(testing::inventory(vec![testing::added(
                "accepted-one",
            )]))),
            cx,
        );
        let pages = &shell.areas.settings.agents.pages;
        assert_eq!(
            pages.get("one").unwrap().inventory().unwrap().agents,
            [testing::added("accepted-one")]
        );
        assert_eq!(
            pages.get("two").unwrap().inventory().unwrap().agents,
            [testing::added("retained-two")]
        );
        assert!(pages.get("two").unwrap().pending().is_none());
        assert!(
            commands.try_recv().is_err(),
            "old-machine completion must not ask the current machine"
        );
    });
}

#[gpui::test]
fn an_already_consumed_list_cannot_initiate_a_registry_read(cx: &mut TestAppContext) {
    testing::init(cx);
    let (link, mut commands) = Link::detached(COMMAND_CAPACITY).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, _, cx| {
        testing::select(shell, "one", Vec::new(), cx);
        let one = MachineKey::from_book("one");
        let pages = &mut shell.areas.settings.agents.pages;
        let read = pages.begin(one.clone(), Operation::Discovery).unwrap();
        assert!(pages.loaded(
            &one,
            read,
            testing::inventory(vec![testing::added("current")])
        ));
        shell.agents_answer(
            &one,
            read,
            Ok(Answer::List(testing::inventory(vec![testing::added(
                "stale",
            )]))),
            cx,
        );
        let page = shell.areas.settings.agents.pages.get("one").unwrap();
        assert_eq!(
            page.inventory().unwrap().agents,
            [testing::added("current")]
        );
        assert!(page.pending().is_none());
        assert!(
            commands.try_recv().is_err(),
            "duplicate reply must not start new I/O"
        );
    });
}

#[gpui::test]
fn an_accepted_visible_list_still_starts_one_registry_read(cx: &mut TestAppContext) {
    testing::init(cx);
    let (link, mut commands) = Link::detached(COMMAND_CAPACITY).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, _, cx| {
        testing::select(shell, "one", Vec::new(), cx);
        let one = MachineKey::from_book("one");
        let read = shell
            .areas
            .settings
            .agents
            .pages
            .begin(one.clone(), Operation::Discovery)
            .unwrap();
        shell.agents_answer(
            &one,
            read,
            Ok(Answer::List(testing::inventory(Vec::new()))),
            cx,
        );
        let Command::Agents {
            machine,
            request,
            ask,
        } = commands.try_recv().unwrap()
        else {
            panic!("one correlated registry read");
        };
        assert_eq!(machine, one);
        assert_ne!(request, read);
        assert!(matches!(
            ask,
            Ask::Catalogue {
                node,
                fetch: CatalogueFetch::Refresh
            } if node == testing::node()
        ));
        assert_eq!(
            shell
                .areas
                .settings
                .agents
                .pages
                .get("one")
                .unwrap()
                .pending()
                .unwrap()
                .id,
            request
        );
        shell.agents_answer(
            &one,
            read,
            Ok(Answer::List(testing::inventory(Vec::new()))),
            cx,
        );
        assert!(commands.try_recv().is_err());
    });
}
