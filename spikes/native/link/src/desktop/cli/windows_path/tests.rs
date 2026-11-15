use super::*;

fn raw(text: &str) -> UserPath {
    UserPath(text.encode_utf16().collect())
}
fn target(text: &str) -> Entry {
    Entry::new(text.encode_utf16().collect()).unwrap()
}

#[test]
fn unicode_install_is_idempotent_and_removal_preserves_every_other_entry() {
    let previous = raw(r"%USERPROFILE%\tools;;C:\다른 도구");
    let entry = target(r"C:\사용자\Vessel 설치\bin");
    let added = previous.changed(&entry, Operation::Install).unwrap();
    assert!(added.contains(&entry));
    assert_eq!(
        added.changed(&entry, Operation::Install).unwrap().encode(),
        added.encode()
    );
    assert_eq!(
        added.changed(&entry, Operation::Remove).unwrap().encode(),
        previous.encode()
    );
    let raw = UserPath::decode(&added.encode()).unwrap();
    assert_eq!(raw.encode(), added.encode());
}

#[test]
fn removal_is_an_exact_entry_never_a_prefix_or_a_foreign_installation() {
    let previous = raw(r"C:\Vessel\bin-old;C:\Other\bin;C:\Vessel\bin;C:\Vessel\bin\extra");
    let next = previous
        .changed(&target(r"C:\Vessel\bin"), Operation::Remove)
        .unwrap();
    assert_eq!(
        next.encode(),
        raw(r"C:\Vessel\bin-old;C:\Other\bin;C:\Vessel\bin\extra").encode()
    );
}

#[test]
fn utf16_is_lossless_and_invalid_or_oversized_data_is_refused() {
    let original = UserPath(vec![0xd800, SEPARATOR, 65]);
    assert_eq!(
        UserPath::decode(&original.encode()).unwrap().encode(),
        original.encode()
    );
    for bytes in [vec![0], vec![0, 0, 1, 0, 0, 0]] {
        assert!(UserPath::decode(&bytes).is_err());
    }
    assert!(UserPath::decode(&vec![0; MAX_PATH_UNITS * 2 + 2]).is_err());
    for text in ["", "C:\\one;C:\\two", "%APPDIR%\\bin"] {
        assert!(Entry::new(text.encode_utf16().collect()).is_err());
    }
    let full = UserPath(vec![65; MAX_PATH_UNITS - 1]);
    assert!(full.changed(&target("B"), Operation::Install).is_err());
    assert_eq!(full.0.len(), MAX_PATH_UNITS - 1);
}

#[test]
fn existing_case_variant_is_not_duplicated_and_only_that_directory_is_removed() {
    let previous = raw(r"%USERPROFILE%\tools;C:\ÄPP\Vessel\BIN;C:\Other\bin");
    let entry = target(r"c:\äpp\vessel\bin");
    assert_eq!(
        previous
            .changed(&entry, Operation::Install)
            .unwrap()
            .encode(),
        previous.encode()
    );
    assert_eq!(
        previous
            .changed(&entry, Operation::Remove)
            .unwrap()
            .encode(),
        raw(r"%USERPROFILE%\tools;C:\Other\bin").encode()
    );
    assert!(!same_entry(
        &"ß".encode_utf16().collect::<Vec<_>>(),
        &"SS".encode_utf16().collect::<Vec<_>>()
    ));
}

#[test]
fn optional_or_repeated_terminators_and_empty_values_are_valid() {
    for bytes in [vec![], vec![0, 0], vec![0, 0, 0, 0]] {
        assert_eq!(UserPath::decode(&bytes).unwrap().encode(), vec![0, 0]);
    }
    for bytes in [vec![65, 0], vec![65, 0, 0, 0], vec![65, 0, 0, 0, 0, 0]] {
        assert_eq!(
            UserPath::decode(&bytes).unwrap().encode(),
            vec![65, 0, 0, 0]
        );
    }
}

#[test]
fn extended_prefixes_use_installer_spelling_and_trailing_separator_is_reused() {
    assert_eq!(target(r"\\?\C:\Vessel\bin").0, target(r"C:\Vessel\bin").0);
    assert_eq!(
        target(r"\\?\UNC\server\Vessel\bin").0,
        target(r"\\server\Vessel\bin").0
    );
    let added = raw(r"C:\other;")
        .changed(&target(r"C:\Vessel\bin"), Operation::Install)
        .unwrap();
    assert_eq!(added.encode(), raw(r"C:\other;C:\Vessel\bin").encode());
}
