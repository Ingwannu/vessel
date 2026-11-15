use super::*;

fn host(name: &str) -> ProjectHost {
    ProjectHost {
        target: ProjectTarget::new(Name::parse(name).unwrap(), NodeId::random()),
        methods: ProjectMethods {
            register: Availability::Available,
            browse: Availability::Available,
            clone: Availability::Available,
            new_directory: Availability::Available,
        },
    }
}

#[test]
fn preferred_single_and_late_online_hosts_skip_only_the_initial_host_page() {
    let a = host("a");
    let b = host("b");
    let hosts = [a.clone(), b.clone()];
    let flow = ProjectNavigationModel::open(&hosts, Some(b.target.host()));
    assert_eq!(
        flow.page(),
        Some(&ProjectPage::Methods {
            target: b.target.clone()
        })
    );
    let mut flow = ProjectNavigationModel::open(&[], None);
    flow.refresh_hosts(std::slice::from_ref(&a), None);
    assert_eq!(
        flow.page(),
        Some(&ProjectPage::Methods {
            target: a.target.clone()
        })
    );
    assert_eq!(
        flow.choose_method(AddMethod::Directory, &hosts),
        Ok(ProjectNavigation::PageChanged)
    );
    flow.refresh_hosts(std::slice::from_ref(&b), None);
    assert!(
        matches!(flow.page(), Some(ProjectPage::Directory { target, .. }) if target == &a.target)
    );
}

#[test]
fn a_rebound_alias_cannot_retarget_the_captured_node_or_advance_its_draft() {
    let a = host("laptop");
    let replacement = host("laptop");
    let mut flow = ProjectNavigationModel::open(std::slice::from_ref(&a), None);
    assert_eq!(
        flow.choose_method(AddMethod::Directory, &[replacement]),
        Err(NavigationRefusal::HostUnavailable)
    );
    assert_eq!(
        flow.page(),
        Some(&ProjectPage::Methods { target: a.target })
    );
}

#[test]
fn disabled_methods_refuse_and_browse_is_only_an_explicit_intent() {
    let mut a = host("a");
    a.methods.clone = Availability::UpdateRequired;
    a.methods.new_directory = Availability::UpdateRequired;
    a.methods.browse = Availability::Unavailable;
    let mut flow = ProjectNavigationModel::open(std::slice::from_ref(&a), None);
    for method in [AddMethod::Github, AddMethod::NewDirectory] {
        assert_eq!(
            flow.choose_method(method, std::slice::from_ref(&a)),
            Err(NavigationRefusal::NotSupported(
                Availability::UpdateRequired
            ))
        );
    }
    assert_eq!(
        flow.choose_method(AddMethod::Browse, std::slice::from_ref(&a)),
        Err(NavigationRefusal::NotSupported(Availability::Unavailable))
    );
    a.methods.browse = Availability::Available;
    assert_eq!(
        flow.choose_method(AddMethod::Browse, std::slice::from_ref(&a)),
        Ok(ProjectNavigation::Browse(a.target.clone()))
    );
    assert_eq!(
        flow.page(),
        Some(&ProjectPage::Methods { target: a.target })
    );
}

#[test]
fn back_restores_search_and_parent_specific_name_drafts_and_then_closes() {
    let a = host("a");
    let hosts = [a.clone(), host("b")];
    let mut flow = ProjectNavigationModel::open(&hosts, None);
    flow.set_input("find host".to_owned()).unwrap();
    flow.choose_host(&a.target, &hosts).unwrap();
    flow.choose_method(AddMethod::NewDirectory, &hosts).unwrap();
    flow.choose_parent(PathBuf::from("/remote/parent-a"))
        .unwrap();
    flow.set_input("retained name".to_owned()).unwrap();
    assert_eq!(flow.back(), ProjectNavigation::PageChanged);
    flow.choose_parent(PathBuf::from("/remote/parent-b"))
        .unwrap();
    assert_eq!(flow.input(), Some(""));
    assert_eq!(flow.back(), ProjectNavigation::PageChanged);
    flow.choose_parent(PathBuf::from("/remote/parent-a"))
        .unwrap();
    assert_eq!(flow.input(), Some("retained name"));
    assert_eq!(flow.back(), ProjectNavigation::PageChanged);
    assert_eq!(flow.back(), ProjectNavigation::PageChanged);
    assert_eq!(flow.back(), ProjectNavigation::PageChanged);
    assert_eq!(flow.input(), Some("find host"));
    assert_eq!(flow.back(), ProjectNavigation::Closed);
    assert!(flow.page().is_none());
    assert!(flow.drafts.is_empty());
}

#[test]
fn clone_location_is_per_exact_target_and_remote_and_survives_back() {
    let a = host("a");
    let hosts = [a];
    let mut flow = ProjectNavigationModel::open(&hosts, None);
    flow.choose_method(AddMethod::Github, &hosts).unwrap();
    let first: CloneUrl = "https://example.invalid/first.git".parse().unwrap();
    let other: CloneUrl = "https://example.invalid/other.git".parse().unwrap();
    flow.choose_repository(first.clone()).unwrap();
    flow.set_input("/node/source".to_owned()).unwrap();
    assert_eq!(flow.back(), ProjectNavigation::PageChanged);
    flow.choose_repository(other).unwrap();
    assert_eq!(flow.input(), Some(""));
    assert_eq!(flow.back(), ProjectNavigation::PageChanged);
    flow.choose_repository(first).unwrap();
    assert_eq!(flow.input(), Some("/node/source"));
}

#[test]
fn draft_capacity_never_evicts_existing_input_and_updates_still_work_when_full() {
    let hosts = [host("a")];
    let mut flow = ProjectNavigationModel::open(&hosts, None);
    flow.choose_method(AddMethod::NewDirectory, &hosts).unwrap();
    for i in 0..MAX_FLOW_DRAFTS {
        flow.choose_parent(PathBuf::from(format!("/node/parent-{i}")))
            .unwrap();
        flow.set_input(format!("name-{i}")).unwrap();
        assert_eq!(flow.back(), ProjectNavigation::PageChanged);
    }
    flow.choose_parent(PathBuf::from("/node/extra")).unwrap();
    assert_eq!(
        flow.set_input("extra".to_owned()),
        Err(NavigationRefusal::DraftLimit)
    );
    assert_eq!(flow.back(), ProjectNavigation::PageChanged);
    flow.choose_parent(PathBuf::from("/node/parent-0")).unwrap();
    assert_eq!(flow.input(), Some("name-0"));
    flow.set_input("changed".to_owned()).unwrap();
    assert_eq!(flow.input(), Some("changed"));
}
