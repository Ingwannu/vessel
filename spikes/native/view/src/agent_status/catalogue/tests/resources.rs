//! Resources name only URLs that the machine actually supplied.
use super::*;
#[test]
fn a_site_repository_or_platform_archive_keeps_its_actual_role_and_url() {
    let site: HttpUrl = "https://example.invalid/site".parse().unwrap();
    let repository: HttpUrl = "https://example.invalid/source".parse().unwrap();
    let archive: HttpUrl = "https://example.invalid/archive.zip".parse().unwrap();
    let mut row = entry("agent", "Agent");
    assert!(resource(&row).is_none());
    row.repository = Some(repository.clone());
    assert!(matches!(resource(&row),Some(Resource::Repository(url)) if url==&repository));
    row.website = Some(site.clone());
    assert!(matches!(resource(&row),Some(Resource::Website(url)) if url==&site));
    row.runs = CatalogueRun::Download {
        archive: archive.clone(),
        sha256: None,
    };
    assert!(matches!(resource(&row),Some(Resource::Archive(url)) if url==&archive));
    assert_eq!(resource(&row).unwrap().url(), &archive);
}
