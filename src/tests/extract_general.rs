use super::*;

#[test]
fn test_extractor_creation() {
    let extractor = GeneralExtractor::new();
    assert!(!extractor.decision_patterns.is_empty());
    assert!(!extractor.preference_patterns.is_empty());
    assert!(!extractor.milestone_patterns.is_empty());
}

#[test]
fn test_extract_decision() {
    let extractor = GeneralExtractor::new();
    let text = "We decided to use Postgres because it's more reliable than MySQL.";
    let memories = extractor.extract(text, 0.3);

    assert!(!memories.is_empty());
    assert_eq!(memories[0].memory_type, MemoryType::Decision);
}

#[test]
fn test_extract_preference() {
    let extractor = GeneralExtractor::new();
    let text = "I always use Rust for systems programming. Never use C++ if you can avoid it.";
    let memories = extractor.extract(text, 0.3);

    assert!(!memories.is_empty());
    assert_eq!(memories[0].memory_type, MemoryType::Preference);
}

#[test]
fn test_extract_milestone() {
    let extractor = GeneralExtractor::new();
    let text = "It works! After three days of debugging, I finally got the authentication working.";
    let memories = extractor.extract(text, 0.3);

    assert!(!memories.is_empty());
    assert_eq!(memories[0].memory_type, MemoryType::Milestone);
}

#[test]
fn test_extract_problem() {
    let extractor = GeneralExtractor::new();
    let text = "The app keeps crashing whenever I try to upload a file. The bug was caused by a null pointer.";
    let memories = extractor.extract(text, 0.3);

    assert!(!memories.is_empty());
    assert_eq!(memories[0].memory_type, MemoryType::Problem);
}

#[test]
fn test_extract_emotional() {
    let extractor = GeneralExtractor::new();
    let text = "I was so scared and proud when the user told me the app helped them. It made me feel really hurt by the criticism but I love the work.";
    let memories = extractor.extract(text, 0.3);

    assert!(!memories.is_empty());
    assert_eq!(memories[0].memory_type, MemoryType::Emotional);
}

#[test]
fn test_is_code_line() {
    let extractor = GeneralExtractor::new();

    assert!(extractor.is_code_line("import os"));
    assert!(extractor.is_code_line("def foo():"));
    assert!(extractor.is_code_line("const x = 1;"));
    assert!(!extractor.is_code_line("This is a normal sentence about something."));
}

#[test]
fn test_split_into_segments() {
    let extractor = GeneralExtractor::new();
    let text = "This is paragraph one.\n\nThis is paragraph two.";
    let segments = extractor.split_into_segments(text);

    assert_eq!(segments.len(), 2);
}

#[test]
fn test_split_by_turns() {
    let extractor = GeneralExtractor::new();
    let lines = vec![
        "Human: Hello".to_string(),
        "Assistant: Hi there".to_string(),
        "Human: How are you?".to_string(),
    ];
    let turn_patterns = vec![
        Regex::new(r"^>\s").unwrap(),
        Regex::new(r"^(Human|User|Q)\s*:").unwrap(),
        Regex::new(r"^(Assistant|AI|A|Claude|ChatGPT)\s*:").unwrap(),
    ];

    let segments = extractor.split_by_turns(lines.as_slice(), &turn_patterns);
    assert!(segments.len() >= 1);
}

#[test]
fn test_memory_type_as_str() {
    assert_eq!(MemoryType::Decision.as_str(), "decision");
    assert_eq!(MemoryType::Preference.as_str(), "preference");
    assert_eq!(MemoryType::Milestone.as_str(), "milestone");
    assert_eq!(MemoryType::Problem.as_str(), "problem");
    assert_eq!(MemoryType::Emotional.as_str(), "emotional");
}

#[test]
fn test_extract_empty_text() {
    let extractor = GeneralExtractor::new();
    let memories = extractor.extract("", 0.3);
    assert!(memories.is_empty());
}

#[test]
fn test_extract_short_text() {
    let extractor = GeneralExtractor::new();
    let memories = extractor.extract("Hi", 0.3);
    assert!(memories.is_empty());
}

#[test]
fn test_extract_with_high_min_confidence() {
    let extractor = GeneralExtractor::new();
    let text = "We decided to use Postgres.";
    let memories = extractor.extract(text, 0.9);
    assert!(memories.is_empty() || memories[0].memory_type == MemoryType::Decision);
}
