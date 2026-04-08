use super::*;

#[test]
fn test_emotion_code_mapping() {
    let dialect = AaakDialect::new();

    assert_eq!(dialect.emotion_code("vulnerability"), Some("vul"));
    assert_eq!(dialect.emotion_code("vulnerable"), Some("vul"));
    assert_eq!(dialect.emotion_code("joy"), Some("joy"));
    assert_eq!(dialect.emotion_code("fear"), Some("fear"));
    assert_eq!(dialect.emotion_code("trust"), Some("trust"));
    assert_eq!(dialect.emotion_code("grief"), Some("grief"));
    assert_eq!(dialect.emotion_code("wonder"), Some("wonder"));
    assert_eq!(dialect.emotion_code("rage"), Some("rage"));
    assert_eq!(dialect.emotion_code("love"), Some("love"));
    assert_eq!(dialect.emotion_code("hope"), Some("hope"));
    assert_eq!(dialect.emotion_code("despair"), Some("despair"));
    assert_eq!(dialect.emotion_code("peace"), Some("peace"));
    assert_eq!(dialect.emotion_code("relief"), Some("relief"));
    assert_eq!(dialect.emotion_code("humor"), Some("humor"));
    assert_eq!(dialect.emotion_code("tenderness"), Some("tender"));
    assert_eq!(dialect.emotion_code("raw_honesty"), Some("raw"));
    assert_eq!(dialect.emotion_code("self_doubt"), Some("doubt"));
    assert_eq!(dialect.emotion_code("anxiety"), Some("anx"));
    assert_eq!(dialect.emotion_code("exhaustion"), Some("exhaust"));
    assert_eq!(dialect.emotion_code("conviction"), Some("convict"));
    assert_eq!(dialect.emotion_code("quiet_passion"), Some("passion"));

    assert_eq!(dialect.emotion_code("JOY"), Some("joy"));
    assert_eq!(dialect.emotion_code("Fear"), Some("fear"));
    assert_eq!(dialect.emotion_code("unknown"), None);
}

#[test]
fn test_encode_emotions() {
    let dialect = AaakDialect::new();

    let emotions = vec!["joy", "fear", "hope"];
    assert_eq!(dialect.encode_emotions(&emotions), "joy+fear+hope");

    let emotions = vec!["joy", "fear", "hope", "love", "peace"];
    assert_eq!(dialect.encode_emotions(&emotions), "joy+fear+hope");

    let emotions = vec!["joy", "joyful", "fear"];
    assert_eq!(dialect.encode_emotions(&emotions), "joy+fear");
}

#[test]
fn test_basic_compression() {
    let dialect = AaakDialect::new();

    let text = "We decided to use GraphQL instead of REST because it provides better type safety. Alice was excited about the API change.";
    let result = dialect.compress(text).unwrap();

    assert!(result.contains("DECISION"), "Should contain DECISION flag: {}", result);
    assert!(result.contains("determ"), "Should contain determ emotion: {}", result);
    assert!(result.contains("excite"), "Should contain excite emotion: {}", result);
    assert!(
        result.contains("graphql") || result.contains("rest") || result.contains("api"),
        "Should contain technical topics: {}",
        result
    );
}

#[test]
fn test_compression_with_entities() {
    let mut dialect = AaakDialect::new();
    dialect.add_entity("Alice", "ALI");
    dialect.add_entity("Bob", "BOB");

    let text = "Alice and Bob were discussing the project. Alice was happy about the progress.";
    let result = dialect.compress(&text).unwrap();

    assert!(result.contains("ALI"), "Should contain ALI entity: {}", result);
    assert!(result.contains("BOB"), "Should contain BOB entity: {}", result);
}

#[test]
fn test_entity_skip() {
    let mut dialect = AaakDialect::new();
    dialect.add_entity("Alice", "ALI");
    dialect.skip_entity("Gandalf");

    let text = "Alice and Gandalf walked into the forest.";
    let entities = dialect.detect_entities_in_text(text);

    assert!(entities.contains(&"ALI".to_string()), "Should find Alice: {:?}", entities);
}

#[test]
fn test_compression_stats() {
    let dialect = AaakDialect::new();

    let original = "This is a longer text that should have more tokens when estimated using the word-based heuristic.";
    let compressed = "0:???|longer_text|\"This is a longer text\"|0.5";

    let stats = dialect.compression_stats(original, compressed);

    assert!(stats["original_tokens_est"].as_u64().unwrap() > stats["summary_tokens_est"].as_u64().unwrap());
    assert!(stats["original_chars"].as_u64().unwrap() > stats["summary_chars"].as_u64().unwrap());
}

#[test]
fn test_decode() {
    let dialect = AaakDialect::new();

    let aaak = r#"001|ALI|BOB|2024-01-15|Project Discussion
0:ALI+bob|project_discussion|"Alice was happy"|0.8|joy|ORIGIN
T:001<->002|colleague""#;

    let result = dialect.decode(aaak).unwrap();

    let header = result.get("header").and_then(|v| v.as_object()).unwrap();
    assert_eq!(header.get("file").and_then(|v| v.as_str()), Some("001"));

    let zettels = result.get("zettels").and_then(|v| v.as_array()).unwrap();
    assert_eq!(zettels.len(), 1);

    let tunnels = result.get("tunnels").and_then(|v| v.as_array()).unwrap();
    assert_eq!(tunnels.len(), 1);
}

#[test]
fn test_encode_zettel() {
    let mut dialect = AaakDialect::new();
    dialect.add_entity("Alice", "ALI");

    let zettel = serde_json::json!({
        "id": "zettel-001",
        "people": ["Alice"],
        "topics": ["testing"],
        "content": "Alice said: \"I love this project!\"",
        "title": "Test Zettel",
        "emotional_weight": 0.75,
        "emotional_tone": ["joy", "hope"]
    });

    let result = dialect.encode_zettel(&zettel).unwrap();

    assert!(result.contains("001"), "Should contain zettel id: {}", result);
    assert!(result.contains("ALI"), "Should contain entity: {}", result);
    assert!(result.contains("testing"), "Should contain topic: {}", result);
    assert!(result.contains("0.75"), "Should contain weight: {}", result);
    assert!(result.contains("joy"), "Should contain emotion: {}", result);
}

#[test]
fn test_encode_zettel_produces_valid_structure() {
    let mut dialect = AaakDialect::new();
    dialect.add_entity("Alice", "ALI");
    dialect.add_entity("Bob", "BOB");

    let zettel = serde_json::json!({
        "id": "zettel-002",
        "people": ["Alice", "Bob"],
        "topics": ["rust", "programming"],
        "content": "Alice and Bob discussed Rust programming.",
        "title": "Rust Discussion",
        "emotional_weight": 0.5,
        "emotional_tone": ["joy"]
    });

    let encoded = dialect.encode_zettel(&zettel).unwrap();

    assert!(encoded.starts_with("002:"), "Should start with zettel ID: {}", encoded);
    assert!(encoded.contains("ALI"), "Should contain entity code ALI: {}", encoded);
    assert!(encoded.contains("BOB"), "Should contain entity code BOB: {}", encoded);
    assert!(encoded.contains("joy"), "Should contain emotion code: {}", encoded);
    assert!(!encoded.contains("Alice"), "Should use entity code ALI instead of name");
    assert!(!encoded.contains("Bob"), "Should use entity code BOB instead of name");
}

#[test]
fn test_compression_stats_reports_reasonable_ratio() {
    let dialect = AaakDialect::new();

    let original = "Alice and Bob were talking about Rust programming. Rust is a great language. Alice thinks it's amazing. Bob agrees. They both love programming in Rust. It's fast and safe.";
    let compressed = dialect.compress(original).unwrap();

    let stats = dialect.compression_stats(original, &compressed);

    let orig_tokens = stats["original_tokens_est"].as_i64().unwrap_or(0);
    let comp_tokens = stats["summary_tokens_est"].as_i64().unwrap_or(0);
    let size_ratio = stats["size_ratio"].as_f64().unwrap_or(0.0);

    assert!(orig_tokens > 0, "Original tokens should be > 0, got {}", orig_tokens);
    assert!(comp_tokens > 0, "Compressed tokens should be > 0, got {}", comp_tokens);
    assert!(size_ratio > 0.0, "Size ratio should be positive, got {}", size_ratio);
}

#[test]
fn test_topic_extraction() {
    let dialect = AaakDialect::new();

    let text = "Rust is a systems programming language that focuses on safety and performance. The compiler is very helpful.";
    let topics = dialect.extract_topics(text);

    assert!(
        topics.contains(&"rust".to_string()) || topics.contains(&"systems".to_string()),
        "Should extract technical topics: {:?}",
        topics
    );
}

#[test]
fn test_decompress_not_supported() {
    let dialect = AaakDialect::new();
    let result = dialect.decompress("some aaak content");
    assert!(result.is_err());
}
