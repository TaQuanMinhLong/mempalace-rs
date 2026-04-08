use super::*;

#[test]
fn test_memory_layer_as_str() {
    assert_eq!(MemoryLayer::L0.as_str(), "L0");
    assert_eq!(MemoryLayer::L1.as_str(), "L1");
    assert_eq!(MemoryLayer::L2.as_str(), "L2");
    assert_eq!(MemoryLayer::L3.as_str(), "L3");
}

#[test]
fn test_layer0_default_text() {
    let mut layer0 = Layer0::new(Some(PathBuf::from("/nonexistent/path/identity.txt")));
    let text = layer0.render();
    assert!(text.contains("No identity configured"));
}

#[test]
fn test_truncate_safe_ascii() {
    let s = "Hello world";
    assert_eq!(truncate_safe(s, 5), "Hello");
    assert_eq!(truncate_safe(s, 100), s);
    assert_eq!(truncate_safe(s, 0), "");
}

#[test]
fn test_truncate_safe_utf8() {
    let s = "Hello ── world";
    let result = truncate_safe(s, 5);
    assert_eq!(result, "Hello");
    assert!(result.chars().count() <= 5);

    let result = truncate_safe(s, 6);
    assert_eq!(result, "Hello ");
    assert!(result.chars().count() <= 6);
}

#[test]
fn test_truncate_safe_utf8_boundary() {
    let s = "┌───────────────────────────────────────────────────────────────────────────────┐";
    let result = truncate_safe(s, 50);
    assert!(result.chars().count() <= 50);
}

#[test]
fn test_truncate_safe_utf8_mixed() {
    let s = "Hello 世界! 🎉";
    let result = truncate_safe(s, 5);
    assert!(result.chars().count() <= 5);
}

#[test]
fn test_search_hit_from_drawer() {
    let drawer = Drawer::new(
        "test_id",
        "Test document content",
        DrawerMetadata::new(
            "wing_test",
            "room_test",
            "test.rs",
            0,
            "test",
            Default::default(),
        ),
    );
    let hit = SearchHit::from_drawer(&drawer, Some(0.5));
    assert_eq!(hit.text, "Test document content");
    assert_eq!(hit.wing, "wing_test");
}
