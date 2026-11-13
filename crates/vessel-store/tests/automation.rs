//! Schedules, their runs, saved prompts and hooks kept across a restart
//! (protocol 11, schema 12).

use fixtures::{at, data_dir, open};
use vessel_proto::automation::{
    Cadence, CatchUp, DeliveryOutcome, EverySeconds, HookDefinition, HookDelivery, HookInfo,
    RunNumber, RunOutcome, RunTrigger, SavedPrompt, ScheduleDefinition, ScheduleInfo, ScheduleRun,
    ScheduleState, ScheduleTarget, SkipReason,
};
use vessel_proto::{Creator, HookId, ProjectId, SavedPromptId, ScheduleId, SessionId};
use vessel_store::{DELIVERIES_KEPT, RUNS_KEPT, StoreConfig};

#[cfg(test)]
#[path = "support/fixtures.rs"]
#[expect(
    dead_code,
    reason = "builders shared with the log's tests; this file uses some"
)]
mod fixtures;

#[cfg(test)]
mod support {
    use super::*;

    pub(crate) fn schedule() -> ScheduleInfo {
        ScheduleInfo {
            id: ScheduleId::random(),
            definition: ScheduleDefinition {
                name: Some("nightly".parse().unwrap()),
                cadence: Cadence::Every {
                    seconds: EverySeconds::try_from(3600).unwrap(),
                    anchor: Some(at(0)),
                },
                target: ScheduleTarget::Session {
                    session: SessionId::random(),
                },
                prompt: "Check the build.".parse().unwrap(),
                catch_up: CatchUp::Once,
                max_runs: None,
                ends_at: None,
            },
            state: ScheduleState::Active { next_due: at(3600) },
            runs: 0,
            last_due: None,
            last_run: None,
            defined_by: Creator::Owner,
            created_at: at(0),
            updated_at: at(0),
            revision: 0,
        }
    }

    pub(crate) fn run(number: u32, outcome: RunOutcome) -> ScheduleRun {
        ScheduleRun {
            number: RunNumber::new(number).unwrap(),
            due_at: at(i64::from(number)),
            at: at(i64::from(number)),
            trigger: RunTrigger::Due,
            outcome,
        }
    }

    pub(crate) fn prompt(name: &str, project: Option<ProjectId>) -> SavedPrompt {
        SavedPrompt {
            id: SavedPromptId::random(),
            name: name.parse().unwrap(),
            text: "Summarise the open issues.".parse().unwrap(),
            project,
            session: None,
            defined_by: Creator::Owner,
            created_at: at(0),
            updated_at: at(0),
            revision: 0,
        }
    }
}

use support::{prompt, run, schedule};

#[tokio::test]
async fn a_schedule_and_the_run_it_took_are_one_write_and_survive_a_restart() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let mut info = schedule();
    store
        .put_schedule(info.clone(), Some(run(1, RunOutcome::Making)))
        .await
        .unwrap();
    info.runs = 1;
    store.put_schedule(info.clone(), None).await.unwrap();
    store
        .put_run(
            info.id,
            run(
                1,
                RunOutcome::Prompted {
                    session: SessionId::random(),
                    delivered: vessel_proto::Delivered::Started,
                },
            ),
        )
        .await
        .unwrap();
    store.close().await;

    let store = open(&dir, StoreConfig::default()).await;
    let stored = store.automation().await.unwrap();
    assert_eq!(stored.schedules, [info.clone()]);
    let runs = store.runs(info.id).await.unwrap();
    assert_eq!(runs.len(), 1);
    assert!(matches!(
        runs.first().map(|run| &run.outcome),
        Some(RunOutcome::Prompted { .. })
    ));
    store.forget_schedule(info.id).await.unwrap();
    assert!(store.automation().await.unwrap().schedules.is_empty());
    assert!(store.runs(info.id).await.unwrap().is_empty());
}

#[tokio::test]
async fn runs_are_trimmed_to_the_newest() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let info = schedule();
    store.put_schedule(info.clone(), None).await.unwrap();
    let skipped = || RunOutcome::Skipped {
        reason: SkipReason::PreviousWaiting,
    };
    for number in 1..=RUNS_KEPT + 5 {
        store
            .put_run(info.id, run(number, skipped()))
            .await
            .unwrap();
    }
    let runs = store.runs(info.id).await.unwrap();
    let numbers: Vec<u32> = runs.iter().map(|run| run.number.get()).collect();
    assert_eq!(numbers.len(), usize::try_from(RUNS_KEPT).unwrap());
    assert_eq!(numbers.first(), Some(&(RUNS_KEPT + 5)));
    assert_eq!(numbers.last(), Some(&6));
}

#[tokio::test]
async fn saved_prompts_are_unique_per_scope_and_listed_by_name() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let project = ProjectId::random();
    let machine = prompt("review", None);
    let mine = prompt("review", Some(project));
    let other = prompt("alpha", Some(ProjectId::random()));
    for saved in [&machine, &mine, &other] {
        store.put_prompt(saved.clone()).await.unwrap();
    }
    // The same name in the same scope is refused by the database too.
    assert!(store.put_prompt(prompt("review", None)).await.is_err());

    let (found, count) = store
        .prompt_named(Some(project), "review".to_owned())
        .await
        .unwrap();
    assert_eq!((found, count), (Some(mine.id), 3));
    let (found, _) = store.prompt_named(None, "alpha".to_owned()).await.unwrap();
    assert_eq!(found, None);

    let names = |list: Vec<SavedPrompt>| list.into_iter().map(|saved| saved.id).collect::<Vec<_>>();
    assert_eq!(
        names(store.saved_prompts(None).await.unwrap()),
        [other.id, machine.id, mine.id]
    );
    let scoped = names(store.saved_prompts(Some(project)).await.unwrap());
    assert_eq!(scoped.len(), 2);
    assert!(scoped.contains(&machine.id) && scoped.contains(&mine.id));

    store.forget_prompt(mine.id).await.unwrap();
    assert_eq!(store.saved_prompt(mine.id).await.unwrap(), None);
    assert_eq!(store.saved_prompt(machine.id).await.unwrap(), Some(machine));
}

#[tokio::test]
async fn hooks_keep_a_trimmed_log_and_a_day_of_keys() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let hook = HookInfo {
        id: HookId::random(),
        definition: HookDefinition {
            name: None,
            target: ScheduleTarget::Session {
                session: SessionId::random(),
            },
            prompt: "A build failed.".parse().unwrap(),
            enabled: true,
        },
        path: "/hooks/x".to_owned(),
        defined_by: Creator::Owner,
        created_at: at(0),
        revision: 0,
    };
    store.put_hook(hook.clone()).await.unwrap();
    for second in 0..i64::from(DELIVERIES_KEPT) + 3 {
        let delivery = HookDelivery {
            at: at(second),
            key: None,
            bytes: 10,
            outcome: DeliveryOutcome::Duplicate,
        };
        store.put_delivery(hook.id, delivery).await.unwrap();
    }
    let log = store.deliveries(hook.id).await.unwrap();
    assert_eq!(log.len(), usize::try_from(DELIVERIES_KEPT).unwrap());
    assert_eq!(
        log.first().map(|delivery| delivery.at),
        Some(at(i64::from(DELIVERIES_KEPT) + 2))
    );

    store
        .remember_key(hook.id, "a".to_owned(), at(10), at(0))
        .await
        .unwrap();
    store
        .remember_key(hook.id, "b".to_owned(), at(20), at(0))
        .await
        .unwrap();
    // A key seen before `forget_before` goes when the next is remembered.
    store
        .remember_key(hook.id, "c".to_owned(), at(30), at(15))
        .await
        .unwrap();
    let keys: Vec<String> = store
        .automation()
        .await
        .unwrap()
        .keys
        .into_iter()
        .map(|(_hook, key, _at)| key)
        .collect();
    assert_eq!(keys, ["b", "c"]);

    store.forget_hook(hook.id).await.unwrap();
    let after = store.automation().await.unwrap();
    assert!(after.hooks.is_empty() && after.keys.is_empty());
    assert!(store.deliveries(hook.id).await.unwrap().is_empty());
}
