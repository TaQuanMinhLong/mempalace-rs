use super::*;
use tempfile::tempdir;

#[test]
fn test_find_session_boundaries() {
    let lines = vec![
        "start".to_string(),
        "Claude Code v1.0".to_string(),
        "session 1".to_string(),
        "more".to_string(),
        "Claude Code v1.0".to_string(),
        "session 2".to_string(),
    ];

    let boundaries = find_session_boundaries(&lines);
    assert_eq!(boundaries, vec![1, 4]);
}

#[test]
fn test_is_true_session_start() {
    let lines = vec!["Claude Code v1.0".to_string(), "Ctrl+E to show".to_string()];
    assert!(!is_true_session_start(&lines, 0));

    let lines2 = vec!["Claude Code v1.0".to_string(), "session content".to_string()];
    assert!(is_true_session_start(&lines2, 0));
}

#[test]
fn test_extract_subject() {
    let lines = vec![
        "> cd project".to_string(),
        "> ls".to_string(),
        "> How do I implement the feature?".to_string(),
    ];

    let subject = extract_subject(&lines);
    assert!(subject.contains("implement"));
}

#[test]
fn test_split_file_dry_run() {
    let dir = tempdir().unwrap();
    let filepath = dir.path().join("test.txt");

    let content = "start\n\
        > prompt 1\n\
        response 1\n\
        > prompt 2\n\
        response 2\n\
        > prompt 3\n\
        response 3\n\
        Claude Code v1.0\n\
        session 1 line 1\n\
        session 1 line 2\n\
        session 1 line 3\n\
        session 1 line 4\n\
        session 1 line 5\n\
        session 1 line 6\n\
        > prompt 4\n\
        response 4\n\
        > prompt 5\n\
        response 5\n\
        Claude Code v1.0\n\
        session 2 line 1\n\
        session 2 line 2\n\
        session 2 line 3\n\
        session 2 line 4\n\
        session 2 line 5\n\
        session 2 line 6\n\
        session 2 line 7\n\
        session 2 line 8\n\
        session 2 line 9\n\
        session 2 line 10\n";
    fs::write(&filepath, content).unwrap();

    let splitter = MegaFileSplitter::new();
    let result = splitter.split_file(&filepath, None, true).unwrap();

    assert_eq!(result.len(), 2);
}

#[test]
fn test_no_split_for_single_session() {
    let dir = tempdir().unwrap();
    let filepath = dir.path().join("test.txt");

    let content = "start\nClaude Code v1.0\nsession 1\nmore content\n";
    fs::write(&filepath, content).unwrap();

    let splitter = MegaFileSplitter::new();
    let result = splitter.split_file(&filepath, None, false).unwrap();

    assert!(result.is_empty());
}
