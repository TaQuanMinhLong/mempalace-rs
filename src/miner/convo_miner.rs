//! Conversation miner - conversation ingestion
//!
//! Ports from Python convo_miner.py:
//! - Normalize format (Claude AI, ChatGPT, Slack, etc.)
//! - Chunk by exchange pairs or topic
//! - Store in ChromaDB

use crate::error::Result;
use crate::normalize::parser::{ChatParser, Exchange};
use crate::palace::{Drawer, DrawerMetadata, IngestMode};
use crate::storage::ChromaStorage;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Conversation file extensions
const CONVO_EXTENSIONS: &[&str] = &[".txt", ".md", ".json", ".jsonl"];

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
    ".mempalace",
    "tool-results",
    "memory",
];

/// Minimum chunk size
const MIN_CHUNK_SIZE: usize = 30;

/// Topic keywords for room detection
fn topic_keywords() -> HashMap<&'static str, Vec<&'static str>> {
    let mut m = HashMap::new();
    m.insert(
        "technical",
        vec![
            "code", "python", "function", "bug", "error", "api", "database", "server", "deploy",
            "git", "test", "debug", "refactor",
        ],
    );
    m.insert(
        "architecture",
        vec![
            "architecture",
            "design",
            "pattern",
            "structure",
            "schema",
            "interface",
            "module",
            "component",
            "service",
            "layer",
        ],
    );
    m.insert(
        "planning",
        vec![
            "plan",
            "roadmap",
            "milestone",
            "deadline",
            "priority",
            "sprint",
            "backlog",
            "scope",
            "requirement",
            "spec",
        ],
    );
    m.insert(
        "decisions",
        vec![
            "decided",
            "chose",
            "picked",
            "switched",
            "migrated",
            "replaced",
            "trade-off",
            "alternative",
            "option",
            "approach",
        ],
    );
    m.insert(
        "problems",
        vec![
            "problem",
            "issue",
            "broken",
            "failed",
            "crash",
            "stuck",
            "workaround",
            "fix",
            "solved",
            "resolved",
        ],
    );
    m
}

/// Conversation miner
#[derive(Debug, Clone)]
pub struct ConvoMiner {
    storage: Rc<RefCell<ChromaStorage>>,
    parser: ChatParser,
}

impl ConvoMiner {
    /// Create a new conversation miner
    pub fn new(storage: ChromaStorage) -> Self {
        Self {
            storage: Rc::new(RefCell::new(storage)),
            parser: ChatParser::new(),
        }
    }

    /// Mine a conversation file
    pub fn mine_conversation_file(&mut self, path: &Path, wing: &str) -> Result<MiningResult> {
        let source_file = path.to_string_lossy().to_string();

        // Read file
        let content = fs::read_to_string(path)?;
        let content = content.trim();

        if content.len() < MIN_CHUNK_SIZE {
            return Ok(MiningResult {
                files_processed: 0,
                drawers_created: 0,
            });
        }

        // Normalize format
        let exchanges = self.parser.normalize_file(content)?;

        if exchanges.len() < 2 {
            return Ok(MiningResult {
                files_processed: 0,
                drawers_created: 0,
            });
        }

        // Chunk by exchanges
        let chunks = self.chunk_exchanges(&exchanges);

        // Detect room
        let room = self.detect_convo_room(&exchanges);

        // Create drawers
        let mut drawers_created = 0;
        for (chunk_index, chunk) in chunks.iter().enumerate() {
            let drawer = self.create_drawer(
                wing,
                &room,
                &chunk.content,
                &source_file,
                chunk_index,
                "mempalace",
            )?;

            let mut storage = self.storage.borrow_mut();
            storage.add_drawer(&drawer)?;
            drawers_created += 1;
        }

        Ok(MiningResult {
            files_processed: 1,
            drawers_created,
        })
    }

    /// Mine a directory of conversation files
    pub fn mine_directory(&mut self, path: &Path, wing: &str) -> Result<DirectoryMiningResult> {
        let files = self.scan_convos(path)?;

        let mut total_files = 0;
        let mut total_drawers = 0;
        let mut room_counts: HashMap<String, usize> = HashMap::new();

        for filepath in files {
            match self.mine_conversation_file(&filepath, wing) {
                Ok(result) => {
                    total_files += result.files_processed;
                    total_drawers += result.drawers_created;

                    if result.drawers_created > 0 {
                        let room = self.detect_room_for_file(&filepath)?;
                        *room_counts.entry(room).or_insert(0) += 1;
                    }
                }
                Err(_) => continue,
            }
        }

        Ok(DirectoryMiningResult {
            files_processed: total_files,
            drawers_created: total_drawers,
            room_counts,
        })
    }

    /// Scan for conversation files
    pub fn scan_convos(&self, convo_dir: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        self.walk_dir(convo_dir, &mut files)?;
        Ok(files)
    }

    fn walk_dir(&self, dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if path.is_dir() {
                // Skip certain directories
                if Self::should_skip_dir(filename) {
                    continue;
                }
                self.walk_dir(&path, files)?;
            } else {
                // Skip .meta.json files
                if filename.ends_with(".meta.json") {
                    continue;
                }

                // Check extension
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| format!(".{}", e.to_lowercase()));

                let ext_match = ext
                    .as_ref()
                    .map(|e| CONVO_EXTENSIONS.contains(&e.as_str()))
                    .unwrap_or(false);

                if ext_match {
                    files.push(path);
                }
            }
        }

        Ok(())
    }

    fn should_skip_dir(dirname: &str) -> bool {
        SKIP_DIRS.contains(&dirname)
    }

    /// Chunk exchanges into groups
    fn chunk_exchanges(&self, exchanges: &[Exchange]) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        let mut i = 0;

        while i < exchanges.len() {
            let exchange = &exchanges[i];

            if exchange.role == "user" {
                let user_turn = exchange.content.clone();
                i += 1;

                // Collect assistant response (up to 8 lines)
                let mut ai_lines = Vec::new();
                while i < exchanges.len() {
                    let next = &exchanges[i];
                    if next.role == "user" || next.content.starts_with("---") {
                        break;
                    }
                    if !next.content.is_empty() {
                        ai_lines.push(next.content.clone());
                    }
                    if ai_lines.len() >= 8 {
                        break;
                    }
                    i += 1;
                }

                let ai_response = ai_lines.join(" ");
                let content = if ai_response.is_empty() {
                    user_turn.clone()
                } else {
                    format!("{}\n{}", user_turn, ai_response)
                };

                if content.len() > MIN_CHUNK_SIZE {
                    chunks.push(Chunk { content });
                }
            } else {
                i += 1;
            }
        }

        // If no user-turn chunks, fall back to paragraph chunking
        if chunks.is_empty() {
            return self.chunk_by_paragraph(exchanges);
        }

        chunks
    }

    /// Fallback: chunk by paragraph
    fn chunk_by_paragraph(&self, exchanges: &[Exchange]) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        let mut current_para = String::new();

        for exchange in exchanges {
            if current_para.is_empty() {
                current_para = exchange.content.clone();
            } else {
                current_para.push_str("\n\n");
                current_para.push_str(&exchange.content);
            }

            // If paragraph is long enough or we have many lines, create a chunk
            if current_para.len() > MIN_CHUNK_SIZE * 3 || current_para.lines().count() > 25 {
                chunks.push(Chunk {
                    content: current_para.clone(),
                });
                current_para.clear();
            }
        }

        // Don't forget the last paragraph
        if current_para.len() > MIN_CHUNK_SIZE {
            chunks.push(Chunk {
                content: current_para,
            });
        }

        chunks
    }

    /// Detect room from exchanges based on content keywords
    fn detect_convo_room(&self, exchanges: &[Exchange]) -> String {
        let content: String = exchanges
            .iter()
            .take(30) // Limit to first 30 exchanges
            .map(|e| e.content.to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");

        let mut scores: HashMap<&str, usize> = HashMap::new();

        for (topic, keywords) in topic_keywords() {
            let score: usize = keywords.iter().map(|kw| content.matches(*kw).count()).sum();

            if score > 0 {
                scores.insert(topic, score);
            }
        }

        scores
            .iter()
            .max_by_key(|(_, score)| *score)
            .map(|(topic, _)| (*topic).to_string())
            .unwrap_or_else(|| "general".to_string())
    }

    /// Detect room for a file
    fn detect_room_for_file(&self, _path: &Path) -> Result<String> {
        // For simplicity, use the default
        Ok("general".to_string())
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
            IngestMode::Convos,
        );

        Ok(Drawer::new(id, content.to_string(), metadata))
    }
}

/// A chunk of conversation
#[derive(Debug, Clone)]
struct Chunk {
    content: String,
    // chunk_index: usize, // not stored; index comes from enumerate()
}

/// Result of mining a conversation file
#[derive(Debug, Clone)]
pub struct MiningResult {
    pub files_processed: usize,
    pub drawers_created: usize,
}

/// Result of mining a directory
#[derive(Debug, Clone)]
pub struct DirectoryMiningResult {
    pub files_processed: usize,
    pub drawers_created: usize,
    pub room_counts: HashMap<String, usize>,
}

#[cfg(test)]
#[path = "../tests/miner_convo_miner.rs"]
mod tests;
