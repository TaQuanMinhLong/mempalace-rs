use super::*;

#[test]
fn test_seed_and_lookup() {
    let mut registry = EntityRegistry::load(None).unwrap();
    registry.seed(
        "personal",
        &[(
            "Riley".to_string(),
            "daughter".to_string(),
            "personal".to_string(),
        )],
        &["MemPalace".to_string()],
        None,
    );

    let riley = registry.lookup("Riley", "");
    assert_eq!(riley.entity_type, "person");
    assert_eq!(riley.confidence, 1.0);
    assert_eq!(riley.source, "onboarding");

    let project = registry.lookup("MemPalace", "");
    assert_eq!(project.entity_type, "project");

    let unknown = registry.lookup("Nobody", "");
    assert_eq!(unknown.entity_type, "unknown");
}

#[test]
fn test_ambiguous_word_disambiguation() {
    let mut registry = EntityRegistry::load(None).unwrap();
    registry.seed(
        "personal",
        &[(
            "Grace".to_string(),
            "friend".to_string(),
            "personal".to_string(),
        )],
        &[],
        None,
    );

    let no_ctx = registry.lookup("grace", "");
    assert_eq!(no_ctx.entity_type, "person");

    let person_ctx = registry.lookup("grace", "hey grace how are you");
    assert_eq!(person_ctx.entity_type, "person");
}

#[test]
fn test_extract_people_from_query() {
    let mut registry = EntityRegistry::load(None).unwrap();
    registry.seed(
        "personal",
        &[(
            "Riley".to_string(),
            "daughter".to_string(),
            "personal".to_string(),
        )],
        &[],
        None,
    );

    let people = registry.extract_people_from_query("Tell me about Riley's project");
    assert!(people.contains(&"Riley".to_string()));
}

#[test]
fn test_extract_unknown_candidates() {
    let registry = EntityRegistry::load(None).unwrap();
    let candidates = registry.extract_unknown_candidates("Tell me about Alice and Bob");
    assert!(candidates.iter().any(|c| c == "Alice" || c == "Bob"));
}

#[test]
fn test_summary() {
    let mut registry = EntityRegistry::load(None).unwrap();
    registry.seed(
        "personal",
        &[(
            "Riley".to_string(),
            "daughter".to_string(),
            "personal".to_string(),
        )],
        &["MemPalace".to_string()],
        None,
    );
    let summary = registry.summary();
    assert!(summary.contains("Riley"));
    assert!(summary.contains("MemPalace"));
}
