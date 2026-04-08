use super::*;

#[test]
fn test_default_config() {
    let config = Config::default();
    assert_eq!(config.collection_name, "mempalace_drawers");
    assert!(config.topic_wings.contains(&"technical".to_string()));
}

#[test]
fn test_expand_tilde() {
    let path = PathBuf::from("~/test");
    let expanded = expand_tilde(&path);
    assert!(!expanded.to_string_lossy().starts_with("~/"));
}
