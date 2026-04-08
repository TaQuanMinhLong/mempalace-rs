//! File miner - project file ingestion
//!
//! Ports from Python miner.py:
//! - Walk directory, respect .gitignore
//! - Chunk files by ~800 chars
//! - Route to rooms based on path/content
//! - Store in ChromaDB
//! - Extract entities to knowledge graph

use crate::config::Config;
use crate::error::Result;
use crate::extract::entity::{EntityExtractor, EntityType};
use crate::palace::{Drawer, DrawerMetadata, IngestMode};
use crate::storage::ChromaStorage;
use crate::storage::{Entity as KgEntity, EntityType as KgEntityType, KnowledgeGraph};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Default chunk size in characters
const CHUNK_SIZE: usize = 800;
/// Overlap between chunks
const CHUNK_OVERLAP: usize = 100;
/// Minimum chunk size
const MIN_CHUNK_SIZE: usize = 50;

/// Readable file extensions
const READABLE_EXTENSIONS: &[&str] = &[
    ".txt", ".md", ".py", ".js", ".ts", ".jsx", ".tsx", ".json", ".yaml", ".yml", ".html", ".css",
    ".java", ".go", ".rs", ".rb", ".sh", ".csv", ".sql", ".toml",
];

/// Directories to skip
const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "__pycache__",
    ".venv",
    "venv",
    "env",
    "dist",
    "build",
    ".next",
    "coverage",
    ".mempalace",
    ".ruff_cache",
    ".mypy_cache",
    ".pytest_cache",
    ".cache",
    ".tox",
    ".nox",
    ".idea",
    ".vscode",
    ".ipynb_checkpoints",
    ".eggs",
    "htmlcov",
    "target",
];

/// Filenames to skip
const SKIP_FILENAMES: &[&str] = &[
    "mempalace.yaml",
    "mempalace.yml",
    "mempal.yaml",
    "mempal.yml",
    ".gitignore",
    "package-lock.json",
];

/// Gitignore matcher for a single directory
#[derive(Debug, Clone)]
struct GitignoreMatcher {
    base_dir: PathBuf,
    rules: Vec<GitignoreRule>,
}

/// A single gitignore rule
#[derive(Debug, Clone)]
struct GitignoreRule {
    pattern: String,
    anchored: bool,
    dir_only: bool,
    negated: bool,
}

impl GitignoreRule {
    // fn matches(&self, path_parts: &[&str]) -> bool { // not called in Rust port
    //     let pattern_parts: Vec<&str> = self.pattern.split('/').collect();
    //     if self.anchored || pattern_parts.len() > 1 {
    //         return self.matches_from_root(path_parts, &pattern_parts);
    //     }
    //     path_parts.iter().any(|part| fnmatch(part, &self.pattern))
    // }

    fn matches_from_root(&self, path_parts: &[&str], pattern_parts: &[&str]) -> bool {
        self.match_at(path_parts, 0, pattern_parts, 0)
    }

    fn match_at(
        &self,
        path_parts: &[&str],
        path_idx: usize,
        pattern_parts: &[&str],
        pat_idx: usize,
    ) -> bool {
        // If we've consumed all pattern parts, it's a match
        if pat_idx == pattern_parts.len() {
            return true;
        }

        // If we've consumed all path parts but still have pattern parts,
        // only ** patterns can match
        if path_idx == path_parts.len() {
            return pattern_parts[pat_idx..].iter().all(|p| *p == "**");
        }

        let pattern_part = pattern_parts[pat_idx];

        if pattern_part == "**" {
            // ** matches anything
            return self.match_at(path_parts, path_idx, pattern_parts, pat_idx + 1)
                || self.match_at(path_parts, path_idx + 1, pattern_parts, pat_idx);
        }

        if !fnmatch(path_parts[path_idx], pattern_part) {
            return false;
        }

        self.match_at(path_parts, path_idx + 1, pattern_parts, pat_idx + 1)
    }
}

impl GitignoreMatcher {
    /// Create a matcher from a directory's .gitignore file
    fn from_dir(dir_path: &Path) -> Option<Self> {
        let gitignore_path = dir_path.join(".gitignore");
        if !gitignore_path.is_file() {
            return None;
        }

        let content = fs::read_to_string(&gitignore_path).ok()?;
        let rules = Self::parse_rules(&content);

        if rules.is_empty() {
            return None;
        }

        Some(Self {
            base_dir: dir_path.to_path_buf(),
            rules,
        })
    }

    fn parse_rules(content: &str) -> Vec<GitignoreRule> {
        let mut rules = Vec::new();

        for raw_line in content.lines() {
            let mut line = raw_line.trim();

            // Handle escaped special characters
            if line.starts_with("\\#") || line.starts_with("\\!") {
                line = &line[1..];
            } else if line.starts_with('#') {
                continue;
            }

            let negated = line.starts_with('!');
            if negated {
                line = &line[1..];
            }

            let anchored = line.starts_with('/');
            if anchored {
                line = line.trim_start_matches('/');
            }

            let dir_only = line.ends_with('/');
            if dir_only {
                line = line.trim_end_matches('/');
            }

            if line.is_empty() {
                continue;
            }

            rules.push(GitignoreRule {
                pattern: line.to_string(),
                anchored,
                dir_only,
                negated,
            });
        }

        rules
    }

    /// Check if a path matches the gitignore rules
    fn matches(&self, path: &Path, is_dir: bool) -> Option<bool> {
        let relative = path
            .strip_prefix(&self.base_dir)
            .ok()?
            .to_string_lossy()
            .into_owned();
        let relative = relative.strip_prefix('/').unwrap_or(&relative);

        if relative.is_empty() {
            return None;
        }

        let parts: Vec<&str> = relative.split('/').collect();
        let mut ignored = None;

        for rule in &self.rules {
            let target_parts = if rule.dir_only {
                if is_dir {
                    &parts
                } else {
                    &parts[..parts.len().saturating_sub(1)]
                }
            } else {
                &parts
            };

            if target_parts.is_empty() {
                continue;
            }

            let matches = if rule.anchored || rule.pattern.contains('/') {
                rule.matches_from_root(target_parts, &rule.pattern.split('/').collect::<Vec<_>>())
            } else {
                target_parts.iter().any(|p| fnmatch(p, &rule.pattern))
            };

            if matches {
                ignored = Some(!rule.negated);
            }
        }

        ignored
    }
}

/// Simple fnmatch-like function for gitignore patterns
///
/// The asterisk matches any sequence including forward slash.
/// Two asterisks at start of pattern means "match any directories".
fn fnmatch(text: &str, pattern: &str) -> bool {
    // Handle ** alone
    if pattern == "**" {
        return true;
    }

    // Handle **/*.rs style patterns (match anywhere in path)
    if let Some(suffix) = pattern.strip_prefix("**/") {
        return text.ends_with(suffix) || text.split('/').any(|part| fnmatch(part, suffix));
    }

    let text_chars: Vec<char> = text.chars().collect();
    let pat_chars: Vec<char> = pattern.chars().collect();

    fnmatch_recursive(&text_chars, 0, &pat_chars, 0)
}

fn fnmatch_recursive(text: &[char], t_idx: usize, pattern: &[char], p_idx: usize) -> bool {
    if p_idx == pattern.len() {
        return t_idx == text.len();
    }

    let pat = pattern[p_idx];

    if pat == '*' {
        // * in gitignore matches anything including /
        // Try matching the rest of the pattern starting from current position
        if fnmatch_recursive(text, t_idx, pattern, p_idx + 1) {
            return true;
        }
        // Try consuming one character and continuing with *
        if t_idx < text.len() && fnmatch_recursive(text, t_idx + 1, pattern, p_idx) {
            return true;
        }
        return false;
    }

    if t_idx >= text.len() {
        return false;
    }

    if pat == '?' {
        return fnmatch_recursive(text, t_idx + 1, pattern, p_idx + 1);
    }

    if pat == '[' {
        // Character class
        if p_idx + 1 < pattern.len() && pattern[p_idx + 1] == '!' {
            // Negated class
            let end = pattern[p_idx..]
                .iter()
                .position(|&c| c == ']')
                .map(|i| p_idx + i);
            if let Some(end_idx) = end {
                let class_chars = &pattern[p_idx + 2..end_idx];
                let matches = text[t_idx] != '!' && !class_chars.contains(&text[t_idx]);
                return matches && fnmatch_recursive(text, t_idx + 1, pattern, end_idx + 1);
            }
        } else {
            let end = pattern[p_idx..]
                .iter()
                .position(|&c| c == ']')
                .map(|i| p_idx + i);
            if let Some(end_idx) = end {
                let class_chars = &pattern[p_idx + 1..end_idx];
                let matches = class_chars.contains(&text[t_idx]);
                return matches && fnmatch_recursive(text, t_idx + 1, pattern, end_idx + 1);
            }
        }
    }

    text[t_idx] == pat && fnmatch_recursive(text, t_idx + 1, pattern, p_idx + 1)
}

/// Room configuration
#[derive(Debug, Clone)]
pub struct Room {
    pub name: String,
    pub description: String,
    pub keywords: Vec<String>,
}

impl Room {
    // fn from_config(config: &serde_json::Value) -> Self { // not used; Python uses dict-based config
    //     let name = config.get("name").and_then(|v| v.as_str()).unwrap_or("general").to_string();
    //     ...
    // }
}

/// File miner
#[derive(Debug, Clone)]
pub struct FileMiner {
    storage: Arc<Mutex<ChromaStorage>>,
    knowledge_graph: Option<Arc<KnowledgeGraph>>,
    // config: Config, // not stored; used only in load_rooms()
    rooms: Vec<Room>,
    entity_extractor: EntityExtractor,
}

impl FileMiner {
    /// Create a new file miner with ChromaStorage only
    pub fn new(config: Config, storage: ChromaStorage) -> Result<Self> {
        Self::with_knowledge_graph(config, storage, None)
    }

    /// Create a new file miner with optional knowledge graph
    pub fn with_knowledge_graph(
        config: Config,
        storage: ChromaStorage,
        knowledge_graph: Option<KnowledgeGraph>,
    ) -> Result<Self> {
        let rooms = Self::load_rooms(&config);
        Ok(Self {
            storage: Arc::new(Mutex::new(storage)),
            knowledge_graph: knowledge_graph.map(Arc::new),
            rooms,
            entity_extractor: EntityExtractor::new(),
        })
    }

    fn load_rooms(config: &Config) -> Vec<Room> {
        // Try to load from wing_config.json
        let config_dir = &config.config_dir;
        let wing_config_path = config_dir.join("wing_config.json");

        if wing_config_path.exists() {
            if let Ok(content) = fs::read_to_string(&wing_config_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(wings) = json.get("wings").and_then(|v| v.as_object()) {
                        return wings
                            .iter()
                            .map(|(name, def)| {
                                let keywords = def
                                    .get("keywords")
                                    .and_then(|v| v.as_array())
                                    .map(|arr| {
                                        arr.iter()
                                            .filter_map(|v| v.as_str().map(String::from))
                                            .collect()
                                    })
                                    .unwrap_or_default();
                                Room {
                                    name: name.clone(),
                                    description: def
                                        .get("description")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    keywords,
                                }
                            })
                            .collect();
                    }
                }
            }
        }

        // Fall back to default rooms from config
        config
            .topic_wings
            .iter()
            .map(|name| Room {
                name: name.clone(),
                description: String::new(),
                keywords: config.hall_keywords.get(name).cloned().unwrap_or_default(),
            })
            .collect()
    }

    /// Mine a directory
    pub fn mine_directory(&mut self, path: &Path, wing: &str) -> Result<MiningResult> {
        let project_path = path.to_path_buf();
        let files = self.scan_project(&project_path)?;

        let mut files_processed = 0;
        let mut drawers_created = 0;
        let mut files_skipped = 0;
        let mut entities_extracted = 0;

        // Collect file paths for entity extraction (batch processing)
        let file_paths: Vec<PathBuf> = files.clone();

        // Extract entities from all files (pass 1 + 2)
        let _detected_entities = if let Some(ref kg) = self.knowledge_graph {
            let path_refs: Vec<&Path> = file_paths.iter().map(|p| p.as_path()).collect();
            let detected = self.entity_extractor.detect_from_files(&path_refs, 50);

            // Store detected entities in knowledge graph
            for entity in detected.people.iter().chain(detected.projects.iter()) {
                let kg_entity = KgEntity {
                    id: format!("file_{}", entity.name.to_lowercase().replace(' ', "_")),
                    name: entity.name.clone(),
                    entity_type: match entity.entity_type {
                        EntityType::Person => KgEntityType::Person,
                        EntityType::Project => KgEntityType::Project,
                        EntityType::Uncertain => KgEntityType::Unknown,
                    },
                    properties: serde_json::json!({
                        "confidence": entity.confidence,
                        "frequency": entity.frequency,
                        "signals": entity.signals,
                        "source": "file_miner",
                    }),
                    created_at: chrono::Utc::now(),
                };
                if let Err(e) = kg.upsert_entity(&kg_entity) {
                    tracing::warn!("Failed to upsert entity {}: {}", entity.name, e);
                } else {
                    entities_extracted += 1;
                }
            }

            // Store uncertain entities too
            for entity in detected.uncertain.iter() {
                let kg_entity = KgEntity {
                    id: format!("file_{}", entity.name.to_lowercase().replace(' ', "_")),
                    name: entity.name.clone(),
                    entity_type: KgEntityType::Unknown,
                    properties: serde_json::json!({
                        "confidence": entity.confidence,
                        "frequency": entity.frequency,
                        "signals": entity.signals,
                        "source": "file_miner",
                    }),
                    created_at: chrono::Utc::now(),
                };
                if let Err(e) = kg.upsert_entity(&kg_entity) {
                    tracing::warn!("Failed to upsert uncertain entity {}: {}", entity.name, e);
                } else {
                    entities_extracted += 1;
                }
            }

            Some(detected)
        } else {
            None
        };

        for filepath in &files {
            let source_file = filepath.to_string_lossy().to_string();

            // Read file
            let content = match fs::read_to_string(filepath) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let content = content.trim();
            if content.len() < MIN_CHUNK_SIZE {
                files_skipped += 1;
                continue;
            }

            // Detect room
            let room = self.detect_room(filepath, content, &project_path);

            // Chunk file
            let chunks = Self::chunk_file(content, CHUNK_SIZE);

            // Create drawers
            for (chunk_index, chunk_content) in chunks.iter().enumerate() {
                let drawer = self.create_drawer(
                    wing,
                    &room,
                    chunk_content,
                    &source_file,
                    chunk_index,
                    "mempalace",
                )?;

                let mut storage = self.storage.lock().map_err(|_| {
                    crate::error::MempalaceError::Mining(
                        "storage lock poisoned while adding drawer".to_string(),
                    )
                })?;
                storage.add_drawer(&drawer)?;
                drawers_created += 1;
            }

            files_processed += 1;
        }

        Ok(MiningResult {
            files_processed,
            drawers_created,
            files_skipped,
            entities_extracted,
            room_counts: HashMap::new(), // Could track per-room counts if needed
        })
    }

    /// Scan project directory for files to mine
    pub fn scan_project(&self, project_path: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let mut active_matchers: Vec<GitignoreMatcher> = Vec::new();
        let mut matcher_cache: HashMap<PathBuf, Option<GitignoreMatcher>> = HashMap::new();

        self.walk_dir(
            project_path,
            project_path,
            &mut files,
            &mut active_matchers,
            &mut matcher_cache,
        )?;
        Ok(files)
    }

    fn walk_dir(
        &self,
        project_path: &Path,
        current_path: &Path,
        files: &mut Vec<PathBuf>,
        active_matchers: &mut Vec<GitignoreMatcher>,
        matcher_cache: &mut HashMap<PathBuf, Option<GitignoreMatcher>>,
    ) -> Result<()> {
        if !current_path.is_dir() {
            return Ok(());
        }

        // Update active matchers - keep only those whose base_dir is an ancestor
        active_matchers
            .retain(|m| m.base_dir == *project_path || current_path.starts_with(&m.base_dir));

        // Load matcher for current directory
        let current_matcher = matcher_cache
            .entry(current_path.to_path_buf())
            .or_insert_with(|| GitignoreMatcher::from_dir(current_path));

        if let Some(matcher) = current_matcher {
            active_matchers.push(matcher.clone());
        }

        for entry in fs::read_dir(current_path)? {
            let entry = entry?;
            let path = entry.path();
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Skip directories
            if path.is_dir() {
                // Check if we should skip this directory
                if Self::should_skip_dir(filename) {
                    continue;
                }

                // Check gitignore
                let mut is_ignored = false;
                for matcher in active_matchers.iter() {
                    if let Some(ignored) = matcher.matches(&path, true) {
                        is_ignored = ignored;
                    }
                }

                if !is_ignored {
                    self.walk_dir(project_path, &path, files, active_matchers, matcher_cache)?;
                }
                continue;
            }

            // Skip by filename
            if Self::should_skip_filename(filename) {
                continue;
            }

            // Check extension
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{}", e.to_lowercase()));

            let ext_match = ext
                .as_ref()
                .map(|e| READABLE_EXTENSIONS.contains(&e.as_str()))
                .unwrap_or(false);
            if !ext_match {
                continue;
            }

            // Check gitignore for file
            let mut is_ignored = false;
            for matcher in active_matchers.iter() {
                if let Some(ignored) = matcher.matches(&path, false) {
                    is_ignored = ignored;
                }
            }

            if is_ignored {
                continue;
            }

            files.push(path);
        }

        Ok(())
    }

    fn should_skip_dir(dirname: &str) -> bool {
        SKIP_DIRS.contains(&dirname) || dirname.ends_with(".egg-info")
    }

    fn should_skip_filename(filename: &str) -> bool {
        SKIP_FILENAMES.contains(&filename)
    }

    /// Chunk file content into pieces
    pub fn chunk_file(content: &str, chunk_size: usize) -> Vec<String> {
        let content = content.trim();
        if content.is_empty() {
            return Vec::new();
        }

        // If content is smaller than MIN_CHUNK_SIZE, don't chunk it
        if content.len() < MIN_CHUNK_SIZE {
            return Vec::new();
        }

        let mut chunks = Vec::new();
        let mut char_start = 0;

        while char_start < content.len() {
            // Calculate tentative end position
            let mut char_end = (char_start + chunk_size).min(content.len());

            // Ensure char_end is on a valid character boundary
            while char_end < content.len() && !content.is_char_boundary(char_end) {
                char_end += 1;
            }

            // Try to find a better break point (paragraph or line boundary)
            let mut break_pos = char_end;
            if char_end < content.len() {
                let search_slice_end = char_end.min(content.len());

                // Look for double newline
                if let Some(pos) = content[char_start..search_slice_end].rfind("\n\n") {
                    let abs_pos = char_start + pos;
                    if pos > chunk_size / 2 {
                        break_pos = abs_pos;
                    }
                }
                // If no good double newline, try single newline
                if break_pos >= char_end - 1 || break_pos - char_start < chunk_size / 2 {
                    if let Some(pos) = content[char_start..search_slice_end].rfind('\n') {
                        let abs_pos = char_start + pos;
                        if pos > chunk_size / 2 {
                            break_pos = abs_pos;
                        }
                    }
                }
            }

            // Ensure break_pos is on a valid character boundary
            while break_pos > char_start && !content.is_char_boundary(break_pos) {
                break_pos -= 1;
            }

            // Ensure char_start is on a valid boundary
            let mut safe_start = char_start;
            while safe_start < content.len() && !content.is_char_boundary(safe_start) {
                safe_start += 1;
            }

            if break_pos > safe_start {
                let chunk = content[safe_start..break_pos].trim();
                if !chunk.is_empty() && chunk.len() >= MIN_CHUNK_SIZE {
                    chunks.push(chunk.to_string());
                }
            }

            // Advance to next chunk position with overlap
            if char_end < content.len() {
                let overlap = CHUNK_OVERLAP.min(chunk_size);
                let next_start = if char_end - overlap > char_start {
                    char_end - overlap
                } else {
                    // Ensure we make progress
                    (char_start + 1).max(char_end.saturating_sub(chunk_size / 2))
                };
                char_start = next_start;
                // Ensure char_start is on a valid boundary
                while char_start < content.len() && !content.is_char_boundary(char_start) {
                    char_start += 1;
                }
            } else {
                break;
            }
        }

        chunks
    }

    /// Detect room from file path and content
    pub fn detect_room(&self, file_path: &Path, content: &str, project_path: &Path) -> String {
        let relative = file_path
            .strip_prefix(project_path)
            .map(|p| p.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let filename = file_path
            .file_stem()
            .and_then(|n| n.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        // Take first 2000 chars safely (UTF-8 boundary)
        let sample_len = content.len().min(2000);
        let mut safe_len = sample_len;
        while safe_len > 0 && !content.is_char_boundary(safe_len) {
            safe_len -= 1;
        }
        let content_lower = content[..safe_len].to_lowercase();

        // Priority 1: folder path matches room name or keywords
        let relative_slash = relative.replace('\\', "/");
        let path_parts: Vec<&str> = relative_slash.split('/').collect();
        for part in &path_parts[..path_parts.len().saturating_sub(1)] {
            for room in &self.rooms {
                let room_lower = room.name.to_lowercase();
                let candidates: Vec<&str> = std::iter::once(room_lower.as_str())
                    .chain(room.keywords.iter().map(|s| s.as_str()))
                    .collect();

                if candidates
                    .iter()
                    .any(|c| *c == *part || c.contains(part) || part.contains(c))
                {
                    return room.name.clone();
                }
            }
        }

        // Priority 2: filename matches room name
        for room in &self.rooms {
            if room.name.to_lowercase() == filename || filename == room.name.to_lowercase() {
                return room.name.clone();
            }
        }

        // Priority 3: keyword scoring
        let mut scores: HashMap<String, usize> = HashMap::new();
        for room in &self.rooms {
            let keywords: Vec<String> = std::iter::once(room.name.to_lowercase())
                .chain(room.keywords.iter().map(|s| s.to_lowercase()))
                .collect();

            let score: usize = keywords
                .iter()
                .map(|kw| content_lower.matches(kw).count())
                .sum();

            if score > 0 {
                scores.insert(room.name.clone(), score);
            }
        }

        if let Some((best_room, _)) = scores.iter().max_by_key(|(_, score)| *score) {
            return best_room.clone();
        }

        "general".to_string()
    }

    /// Extract entities from content using simple capitalized word detection
    pub fn extract_entities(&self, content: &str) -> Vec<String> {
        let mut entities = Vec::new();

        // Look for capitalized words that appear frequently
        let mut counts: HashMap<String, usize> = HashMap::new();

        let re = regex::Regex::new(r"\b([A-Z][a-z]{2,20})\b").unwrap();
        for cap in re.find_iter(content) {
            let word = cap.as_str();
            let word_lower = word.to_lowercase();
            // Skip if it's a common word
            if !Self::is_likely_entity(&word_lower) {
                continue;
            }
            *counts.entry(word.to_string()).or_insert(0) += 1;
        }

        // Filter: must appear at least 2 times
        for (name, count) in counts {
            if count >= 2 {
                entities.push(name);
            }
        }

        entities
    }

    /// Check if a lowercase word is likely an entity name (not a common word)
    fn is_likely_entity(word: &str) -> bool {
        let common_words: HashSet<&str> = [
            "the", "and", "but", "for", "not", "you", "all", "can", "her", "was", "one", "our",
            "out", "day", "get", "has", "him", "his", "how", "its", "may", "new", "now", "old",
            "see", "two", "way", "who", "boy", "did", "man", "end", "her", "saw", "set", "she",
            "too", "use",
        ]
        .into_iter()
        .collect();

        !common_words.contains(word) && word.len() > 2
    }

    fn create_drawer(
        &self,
        wing: &str,
        room: &str,
        content: &str,
        source_file: &str,
        chunk_index: usize,
        agent: &str,
    ) -> Result<Drawer> {
        let id = Drawer::generate_id(wing, room, &format!("{}_{}", source_file, chunk_index));

        let metadata = DrawerMetadata::new(
            wing,
            room,
            source_file,
            chunk_index,
            agent,
            IngestMode::Projects,
        );

        Ok(Drawer::new(id, content.to_string(), metadata))
    }
}

/// Result of mining operation
#[derive(Debug, Clone)]
pub struct MiningResult {
    pub files_processed: usize,
    pub drawers_created: usize,
    pub files_skipped: usize,
    pub entities_extracted: usize,
    pub room_counts: HashMap<String, usize>,
}

#[cfg(test)]
#[path = "../tests/miner_file_miner.rs"]
mod tests;
