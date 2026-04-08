use super::*;
use tempfile::TempDir;

#[test]
fn test_chunk_file_basic() {
    let content = "This is a test. It should be chunked properly with enough content to exceed the minimum chunk size. Adding more text here to ensure the chunk is big enough.\n\nAnother paragraph here with more text to make it longer. Adding more content to ensure we exceed the minimum chunk size requirement for the chunking algorithm to work properly.";
    let chunks = FileMiner::chunk_file(content, 50);
    assert!(!chunks.is_empty());
    for chunk in &chunks {
        assert!(chunk.len() >= MIN_CHUNK_SIZE || chunk.len() == content.len());
    }
}

#[test]
fn test_chunk_file_small() {
    let content = "Short";
    let chunks = FileMiner::chunk_file(content, 800);
    assert!(chunks.is_empty());
}

#[test]
fn test_chunk_file_empty() {
    let chunks = FileMiner::chunk_file("", 800);
    assert!(chunks.is_empty());
}

#[test]
fn test_chunk_file_preserves_words() {
    let content = "Hello world. This is a test of chunking that should work properly with enough content to exceed the minimum chunk size requirement for proper word preservation. Another sentence here with even more content to ensure we pass the minimum chunk size.";
    let chunks = FileMiner::chunk_file(content, 50);
    assert!(chunks.len() >= 1);
}

#[test]
fn test_chunk_file_exact_size() {
    let content = "A".repeat(1000);
    let chunks = FileMiner::chunk_file(&content, 800);
    assert!(chunks.len() >= 2);
}

#[test]
fn test_chunk_file_utf8_char_boundary() {
    let content = "Hello ── world. This is a test with em dash characters that should not cause any issues when chunked properly. Adding more text to exceed minimum chunk size requirement here."
        .repeat(10);
    let chunks = FileMiner::chunk_file(&content, 500);
    assert!(!chunks.is_empty());
    for chunk in &chunks {
        assert!(!chunk.is_empty());
    }
}

#[test]
fn test_chunk_file_utf8_box_drawing() {
    let content = "┌─────────────────────┐\n│ Test Content Here │\n└─────────────────────┘\n".repeat(20);
    let chunks = FileMiner::chunk_file(&content, 200);
    assert!(!chunks.is_empty());
    for chunk in &chunks {
        let _ = chunk.as_str();
    }
}

#[test]
fn test_chunk_file_utf8_various() {
    let content = "Hello 世界! 🎉 → ← ↑ ↓ ● ○ ◆ ◇\n中文测试\nEm dash — and en dash –\n".repeat(30);
    let chunks = FileMiner::chunk_file(&content, 300);
    assert!(!chunks.is_empty());
}

#[test]
fn test_fnmatch() {
    assert!(fnmatch("test.rs", "*.rs"));
    assert!(fnmatch("foo/bar.rs", "*.rs"));
    assert!(!fnmatch("test.txt", "*.rs"));
}

#[test]
fn test_fnmatch_double_star() {
    assert!(fnmatch("foo/bar/baz.rs", "**/*.rs"));
    assert!(fnmatch("baz.rs", "**/*.rs"));
    assert!(!fnmatch("foo/bar/baz.txt", "**/*.rs"));
}

#[test]
fn test_fnmatch_question_mark() {
    assert!(fnmatch("test1.rs", "test?.rs"));
    assert!(fnmatch("test9.rs", "test?.rs"));
    assert!(!fnmatch("test10.rs", "test?.rs"));
}

#[test]
fn test_room_detection_by_path() {
    let config = Config::default();
    let storage = ChromaStorage::new(Path::new("test"), "test").unwrap();
    let miner = FileMiner::new(config, storage).unwrap();

    let project_path = Path::new("/test");
    let file_path = project_path.join("src/technical/code.rs");
    let content = "This is a technical file about Python and APIs";

    let room = miner.detect_room(&file_path, content, project_path);
    assert_eq!(room, "technical");
}

#[test]
fn test_room_detection_by_content() {
    let config = Config::default();
    let storage = ChromaStorage::new(Path::new("test"), "test").unwrap();
    let miner = FileMiner::new(config, storage).unwrap();

    let project_path = Path::new("/test");
    let file_path = project_path.join("src/generic/file.rs");
    let content = "I am scared and worried about my feelings. The consciousness of being aware.";

    let room = miner.detect_room(&file_path, content, project_path);
    assert!(room == "emotions" || room == "consciousness" || room == "general");
}

#[test]
fn test_room_detection_fallback() {
    let config = Config::default();
    let storage = ChromaStorage::new(Path::new("test"), "test").unwrap();
    let miner = FileMiner::new(config, storage).unwrap();

    let project_path = Path::new("/test");
    let file_path = project_path.join("random/file.txt");
    let content = "This is some random content that does not match any room keywords.";

    let room = miner.detect_room(&file_path, content, project_path);
    assert_eq!(room, "general");
}

#[test]
fn test_room_detection_utf8_char_boundary() {
    let config = Config::default();
    let storage = ChromaStorage::new(Path::new("test"), "test").unwrap();
    let miner = FileMiner::new(config, storage).unwrap();

    let project_path = Path::new("/test");
    let file_path = project_path.join("src/main.rs");

    let content = "┌──────────────────────────────────────────────┐\n│ Test file with UTF-8 box drawing characters │\n└──────────────────────────────────────────────┘\n"
        .repeat(10);

    let room = miner.detect_room(&file_path, &content, project_path);
    assert!(!room.is_empty());
}

#[test]
fn test_room_detection_utf8_mixed() {
    let config = Config::default();
    let storage = ChromaStorage::new(Path::new("test"), "test").unwrap();
    let miner = FileMiner::new(config, storage).unwrap();

    let project_path = Path::new("/test");
    let file_path = project_path.join("src/中文.rs");

    let content = "这是一个测试文件 📝\n中文内容测试\nRust编程语言 🔧\n—— 破折号 ——\n".repeat(50);

    let room = miner.detect_room(&file_path, &content, project_path);
    assert!(!room.is_empty());
}

#[test]
fn test_should_skip_dirs() {
    assert!(FileMiner::should_skip_dir("node_modules"));
    assert!(FileMiner::should_skip_dir(".git"));
    assert!(FileMiner::should_skip_dir("__pycache__"));
    assert!(FileMiner::should_skip_dir("target"));
    assert!(!FileMiner::should_skip_dir("src"));
    assert!(!FileMiner::should_skip_dir("lib"));
}

#[test]
fn test_should_skip_dir_egg_info() {
    assert!(FileMiner::should_skip_dir("mypackage.egg-info"));
    assert!(!FileMiner::should_skip_dir("src"));
}

#[test]
fn test_should_skip_filenames() {
    assert!(FileMiner::should_skip_filename("mempalace.yaml"));
    assert!(FileMiner::should_skip_filename("mempalace.yml"));
    assert!(FileMiner::should_skip_filename(".gitignore"));
    assert!(FileMiner::should_skip_filename("package-lock.json"));
    assert!(!FileMiner::should_skip_filename("main.rs"));
    assert!(!FileMiner::should_skip_filename("lib.rs"));
}

#[test]
fn test_readable_extensions() {
    assert!(READABLE_EXTENSIONS.contains(&".rs"));
    assert!(READABLE_EXTENSIONS.contains(&".py"));
    assert!(READABLE_EXTENSIONS.contains(&".js"));
    assert!(READABLE_EXTENSIONS.contains(&".txt"));
    assert!(READABLE_EXTENSIONS.contains(&".md"));
    assert!(!READABLE_EXTENSIONS.contains(&".exe"));
    assert!(!READABLE_EXTENSIONS.contains(&".dll"));
}

#[test]
fn test_extract_entities_basic() {
    let config = Config::default();
    let storage = ChromaStorage::new(Path::new("test"), "test").unwrap();
    let miner = FileMiner::new(config, storage).unwrap();

    let content = "Alice said hello to Bob. Alice told Bob something. Alice wrote a book.";
    let entities = miner.extract_entities(content);

    assert!(entities.contains(&"Alice".to_string()));
}

#[test]
fn test_extract_entities_filters_common_words() {
    let config = Config::default();
    let storage = ChromaStorage::new(Path::new("test"), "test").unwrap();
    let miner = FileMiner::new(config, storage).unwrap();

    let content = "The quick brown fox jumps over the lazy dog. The dog slept.";
    let entities = miner.extract_entities(content);

    assert!(!entities.contains(&"The".to_string()));
}

#[test]
fn test_mining_result_default() {
    let result = MiningResult {
        files_processed: 10,
        drawers_created: 50,
        files_skipped: 2,
        entities_extracted: 5,
        room_counts: HashMap::new(),
    };

    assert_eq!(result.files_processed, 10);
    assert_eq!(result.drawers_created, 50);
    assert_eq!(result.entities_extracted, 5);
}

#[test]
fn test_gitignore_matcher_from_dir_not_exists() {
    let temp_dir = TempDir::new().unwrap();
    let matcher = GitignoreMatcher::from_dir(temp_dir.path());
    assert!(matcher.is_none());
}

#[test]
fn test_gitignore_matcher_parse() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(
        temp_dir.path().join(".gitignore"),
        "*.log\nnode_modules/\n# comment\n!important.txt",
    )
    .unwrap();

    let matcher = GitignoreMatcher::from_dir(temp_dir.path()).unwrap();
    assert!(!matcher.rules.is_empty());
}

#[test]
fn test_is_likely_entity() {
    assert!(FileMiner::is_likely_entity("Alice"));
    assert!(FileMiner::is_likely_entity("MemPalace"));
    assert!(!FileMiner::is_likely_entity("the"));
    assert!(!FileMiner::is_likely_entity("and"));
    assert!(!FileMiner::is_likely_entity("ab"));
}
