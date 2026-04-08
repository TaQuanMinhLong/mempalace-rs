use super::*;

#[test]
fn test_generate_id() {
    let id1 = Drawer::generate_id("wing_kai", "auth", "hello world");
    let id2 = Drawer::generate_id("wing_kai", "auth", "hello world");
    let id3 = Drawer::generate_id("wing_kai", "auth", "different");

    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
    assert!(id1.starts_with("drawer_wing_kai_auth_"));
}

#[test]
fn test_drawer_metadata() {
    let metadata = DrawerMetadata::new(
        "wing_kai",
        "auth",
        "/path/to/file.py",
        0,
        "cli",
        IngestMode::Projects,
    );

    assert_eq!(metadata.wing, "wing_kai");
    assert_eq!(metadata.room, "auth");
    assert_eq!(metadata.ingest_mode, IngestMode::Projects);
}
