use super::*;
#[test]
fn cold_host_selection_does_not_erase_it_and_loaded_absence_falls_back_to_all() {
    let available = [1, 2, 3];
    let stored = [9, 2];
    assert_eq!(
        hosts(&available, &stored, RegistryReadiness::Loading),
        vec![&2]
    );
    assert_eq!(
        hosts(&available, &stored, RegistryReadiness::Loaded),
        vec![&2]
    );
    assert_eq!(
        hosts(&available, &[9], RegistryReadiness::Loading),
        Vec::<&u8>::new()
    );
    assert_eq!(
        hosts(&available, &[9], RegistryReadiness::Loaded),
        vec![&1, &2, &3]
    );
    assert_eq!(
        hosts(&available, &[], RegistryReadiness::Loading),
        vec![&1, &2, &3]
    );
}
#[test]
fn project_filter_goes_inert_and_resumes_without_deleting_saved_keys() {
    let saved = [4, 8];
    assert_eq!(projects(&saved, &[8, 9]), vec![&8]);
    assert!(projects(&saved, &[9]).is_empty());
    assert_eq!(projects(&saved, &[4, 8]), vec![&4, &8]);
    assert_eq!(saved, [4, 8]);
}
#[test]
fn named_labels_and_unlabelled_share_one_or_selection_without_a_second_normalizer() {
    let selected = [
        LabelChoice::Named(1),
        LabelChoice::Named(3),
        LabelChoice::Unlabelled,
    ];
    assert!(labels(&selected, &[]));
    assert!(labels(&selected, &[1, 2]));
    assert!(labels(&selected, &[3]));
    assert!(!labels(&selected, &[2]));
    assert!(labels::<u8>(&[], &[2]));
    assert!(!labels(&[LabelChoice::Unlabelled], &[2]));
}
#[test]
fn label_filter_and_hidden_rows_prune_differently_from_an_originally_empty_project() {
    assert!(keep_project::<u8, u8>(&[], &[], &[]));
    assert!(!keep_project::<_, u8>(&[1], &[], &[]));
    assert!(!keep_project::<u8, u8>(
        &[],
        &[],
        &[LabelChoice::Unlabelled]
    ));
    assert!(keep_project(&[1, 2], &[2], &[LabelChoice::Named(4)]));
}
