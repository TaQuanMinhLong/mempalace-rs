use super::*;

#[test]
fn test_detect_plain_text_with_markers() {
    let parser = ChatParser::new();
    let content = "> Hello\nHi there\n> How are you?\nI'm good";
    assert_eq!(parser.detect_format(content), ChatFormat::PlainText);
}

#[test]
fn test_detect_empty_content() {
    let parser = ChatParser::new();
    assert_eq!(parser.detect_format(""), ChatFormat::PlainText);
}

#[test]
fn test_normalize_plain_text() {
    let parser = ChatParser::new();
    let content = "> Hello\nHi there\n\n> How are you?\nI'm good";
    let exchanges = parser.normalize(content, ChatFormat::PlainText).unwrap();
    assert!(exchanges.len() >= 1);
}

#[test]
fn test_claude_code_jsonl() {
    let parser = ChatParser::new();
    let content = r#"{"type": "human", "message": {"content": [{"type": "text", "text": "Hello"}]}}
{"type": "assistant", "message": {"content": [{"type": "text", "text": "Hi there"}]}}"#;

    let exchanges = parser.normalize(content, ChatFormat::ClaudeCode).unwrap();
    assert_eq!(exchanges.len(), 2);
    assert_eq!(exchanges[0].role, "user");
    assert_eq!(exchanges[0].content, "Hello");
    assert_eq!(exchanges[1].role, "assistant");
    assert_eq!(exchanges[1].content, "Hi there");
}

#[test]
fn test_codex_jsonl() {
    let parser = ChatParser::new();
    let content = r#"{"type": "session_meta", "version": 1}
{"type": "event_msg", "payload": {"type": "user_message", "message": "Hello"}}
{"type": "event_msg", "payload": {"type": "agent_message", "message": "Hi there"}}"#;

    let exchanges = parser.normalize(content, ChatFormat::Codex).unwrap();
    assert_eq!(exchanges.len(), 2);
}

#[test]
fn test_extract_content_string() {
    let parser = ChatParser::new();
    let content = serde_json::json!("Hello world");
    assert_eq!(parser.extract_content(content), "Hello world");
}

#[test]
fn test_extract_content_array() {
    let parser = ChatParser::new();
    let content = serde_json::json!([{"type": "text", "text": "Hello"}, {"type": "text", "text": "World"}]);
    assert_eq!(parser.extract_content(content), "Hello World");
}

#[test]
fn test_extract_content_object() {
    let parser = ChatParser::new();
    let content = serde_json::json!({"text": "Hello"});
    assert_eq!(parser.extract_content(content), "Hello");
}

#[test]
fn test_to_transcript() {
    let parser = ChatParser::new();
    let exchanges = vec![
        Exchange {
            role: "user".to_string(),
            content: "Hello".to_string(),
            timestamp: None,
        },
        Exchange {
            role: "assistant".to_string(),
            content: "Hi there".to_string(),
            timestamp: None,
        },
    ];

    let transcript = parser.to_transcript(&exchanges);
    assert!(transcript.contains("> Hello"));
    assert!(transcript.contains("Hi there"));
}

#[test]
fn test_slack_normalization() {
    let parser = ChatParser::new();
    let content = r#"[
        {"type": "message", "user": "U1", "text": "Hello"},
        {"type": "message", "user": "U2", "text": "Hi there"},
        {"type": "message", "user": "U1", "text": "How are you?"}
    ]"#;

    let exchanges = parser.normalize(content, ChatFormat::Slack).unwrap();
    assert_eq!(exchanges.len(), 3);
    assert_eq!(exchanges[0].role, "user");
    assert_eq!(exchanges[1].role, "assistant");
    assert_eq!(exchanges[2].role, "user");
}

#[test]
fn test_chatgpt_topological_sort() {
    let parser = ChatParser::new();

    let content = r#"{"mapping":{"0":{"children":["1"]},"1":{"parent":"0","message":{"author":{"role":"user"},"content":{"parts":["Hello"]},"children":[]}}}}"#;

    let result = std::panic::catch_unwind(|| parser.normalize(content, ChatFormat::ChatGPT));
    assert!(result.is_ok(), "ChatGPT normalization should not panic");
}
