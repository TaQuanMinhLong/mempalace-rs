use super::*;

#[test]
fn test_entity_extractor_creation() {
    let extractor = EntityExtractor::new();
    assert!(extractor.stopwords.contains("hello"));
    assert!(extractor.stopwords.contains("the"));
}

#[test]
fn test_extract_candidates() {
    let extractor = EntityExtractor::new();
    let text = "Alice said hello to Bob. Alice told Bob something. Alice wrote a book about Bob.";
    let candidates = extractor.extract_candidates(text);

    assert!(candidates.contains_key("Alice"));
    assert!(candidates.contains_key("Bob"));
    assert_eq!(*candidates.get("Alice").unwrap(), 3);
    assert_eq!(*candidates.get("Bob").unwrap(), 3);
}

#[test]
fn test_classify_person() {
    let extractor = EntityExtractor::new();
    let text = "Alice said hello. Alice laughed. Alice thinks. She is happy.";
    let candidates = extractor.extract_candidates(text);

    if let Some((name, freq)) = candidates.iter().find(|(n, _)| *n == "Alice") {
        let scores = extractor.score_entity(
            name,
            text,
            &text.lines().map(|s| s.to_string()).collect::<Vec<_>>(),
        );
        let entity = extractor.classify_entity(name, *freq, &scores);
        assert_eq!(entity.entity_type, EntityType::Person);
    }
}

#[test]
fn test_classify_project() {
    let extractor = EntityExtractor::new();
    let text = "We built MemPalace v2. We shipped MemPalace. The MemPalace architecture is great.";
    let candidates = extractor.extract_candidates(text);

    if let Some((name, freq)) = candidates.iter().find(|(n, _)| *n == "MemPalace") {
        let scores = extractor.score_entity(
            name,
            text,
            &text.lines().map(|s| s.to_string()).collect::<Vec<_>>(),
        );
        let entity = extractor.classify_entity(name, *freq, &scores);
        assert_eq!(entity.entity_type, EntityType::Project);
    }
}

#[test]
fn test_detected_entities_empty() {
    let extractor = EntityExtractor::new();
    let result = extractor.detect_from_files(&[], 10);
    assert!(result.people.is_empty());
    assert!(result.projects.is_empty());
}

#[test]
fn test_entity_display() {
    let entity = Entity {
        name: "Alice".to_string(),
        entity_type: EntityType::Person,
        confidence: 0.85,
        frequency: 10,
        signals: vec!["dialogue marker (3x)".to_string()],
    };
    let display = entity.display();
    assert!(display.contains("Alice"));
    assert!(display.contains("person"));
}

#[test]
fn test_cross_file_entity_aggregation() {
    let extractor = EntityExtractor::new();

    let file1 = tempfile::NamedTempFile::with_suffix(".txt").unwrap();
    let file2 = tempfile::NamedTempFile::with_suffix(".txt").unwrap();

    std::fs::write(
        file1.path(),
        "Alice said hello. Alice laughed. Alice thinks.",
    )
    .unwrap();
    std::fs::write(
        file2.path(),
        "Alice is here. She works on Rust. Alice was happy.",
    )
    .unwrap();

    let result = extractor.detect_from_files(&[file1.path(), file2.path()], 10);

    let alice = result.all().into_iter().find(|e| e.name == "Alice");
    assert!(
        alice.is_some(),
        "Alice should be detected (frequency 5 across files): got {:?}",
        result.all()
    );
    assert_eq!(
        alice.unwrap().frequency,
        5,
        "Alice frequency should be 5 across both files"
    );
}
