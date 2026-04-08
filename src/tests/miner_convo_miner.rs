use super::*;

#[test]
fn test_chunk_exchanges() {
    let exchanges = vec![
        Exchange {
            role: "user".to_string(),
            content: "Hello, this is a test message that should be long enough".to_string(),
            timestamp: None,
        },
        Exchange {
            role: "assistant".to_string(),
            content: "Hi there, this is a response that should also be long enough".to_string(),
            timestamp: None,
        },
    ];

    let miner = ConvoMiner::new(ChromaStorage::new(Path::new("test"), "test").unwrap());
    let chunks = miner.chunk_exchanges(&exchanges);

    assert!(!chunks.is_empty());
}

#[test]
fn test_detect_convo_room() {
    let miner = ConvoMiner::new(ChromaStorage::new(Path::new("test"), "test").unwrap());

    let exchanges = vec![
        Exchange {
            role: "user".to_string(),
            content: "I need to fix a bug in my Python code".to_string(),
            timestamp: None,
        },
        Exchange {
            role: "assistant".to_string(),
            content: "What kind of bug is it?".to_string(),
            timestamp: None,
        },
    ];

    let room = miner.detect_convo_room(&exchanges);
    assert_eq!(room, "technical");
}

#[test]
fn test_detect_convo_room_planning() {
    let miner = ConvoMiner::new(ChromaStorage::new(Path::new("test"), "test").unwrap());

    let exchanges = vec![
        Exchange {
            role: "user".to_string(),
            content: "What's the plan for the next sprint?".to_string(),
            timestamp: None,
        },
        Exchange {
            role: "assistant".to_string(),
            content: "We need to complete the milestone".to_string(),
            timestamp: None,
        },
    ];

    let room = miner.detect_convo_room(&exchanges);
    assert_eq!(room, "planning");
}

#[test]
fn test_should_skip_dirs() {
    assert!(ConvoMiner::should_skip_dir("node_modules"));
    assert!(ConvoMiner::should_skip_dir(".git"));
    assert!(!ConvoMiner::should_skip_dir("src"));
}
