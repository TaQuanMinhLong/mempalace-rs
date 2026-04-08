//! Mega file splitter - split concatenated transcript files
//!
//! Ports from Python split_mega_files.py:
//! - Scans directory for .txt files with multiple Claude Code sessions
//! - Splits each into individual files named with date, time, people, and subject
//! - Original files are renamed with .mega_backup extension

use crate::error::Result;
use chrono::{DateTime, Utc};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

const DEFAULT_KNOWN_PEOPLE: [&str; 7] = ["Alice", "Ben", "Riley", "Max", "Sam", "Devon", "Jordan"];

/// Pre-compiled regexes for subject extraction
static RE_SUBJECT_SKIP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\.\/|cd |ls |python|bash|git |cat |source |export |claude|./activate)").unwrap()
});
static RE_CLEAN_NONWORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^\w\s-]").unwrap());
static RE_CLEAN_SPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

/// Pre-compiled regexes for filename cleaning
static RE_STEM_CLEAN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^\w-]").unwrap());
static RE_SUBJECT_CLEAN1: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^\w\.\-]").unwrap());
static RE_SUBJECT_CLEAN2: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"_+").unwrap());

/// Load known names from config file
fn load_known_people() -> Vec<String> {
    let config_path = std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".mempalace/known_names.json"))
        .unwrap_or_default();

    if config_path.exists() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(names) = data.get("names").and_then(|v| v.as_array()) {
                    return names
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect();
                }
            }
        }
    }

    DEFAULT_KNOWN_PEOPLE.into_iter().map(String::from).collect()
}

/// Load username to name mapping
fn load_username_map() -> HashMap<String, String> {
    let config_path = std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".mempalace/known_names.json"))
        .unwrap_or_default();

    if config_path.exists() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(map) = data.get("username_map").and_then(|v| v.as_object()) {
                    let mut result = HashMap::new();
                    for (k, v) in map {
                        if let Some(name) = v.as_str() {
                            result.insert(k.clone(), name.to_string());
                        }
                    }
                    return result;
                }
            }
        }
    }

    HashMap::new()
}

/// Check if this is a true session start (not a context restore)
fn is_true_session_start(lines: &[String], idx: usize) -> bool {
    let nearby: String = lines[idx..idx.saturating_add(6).min(lines.len())].join("");
    !nearby.contains("Ctrl+E") && !nearby.contains("previous messages")
}

/// Find session boundaries in a mega file
fn find_session_boundaries(lines: &[String]) -> Vec<usize> {
    let mut boundaries = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if line.contains("Claude Code v") && is_true_session_start(lines, i) {
            boundaries.push(i);
        }
    }

    boundaries
}

/// Extract timestamp from session lines
fn extract_timestamp(lines: &[String]) -> Option<(String, String)> {
    let ts_pattern =
        Regex::new(r"⏺\s+(\d{1,2}:\d{2}\s+[AP]M)\s+\w+,\s+(\w+)\s+(\d{1,2}),\s+(\d{4})").ok()?;

    let months: HashMap<&str, &str> = [
        ("January", "01"),
        ("February", "02"),
        ("March", "03"),
        ("April", "04"),
        ("May", "05"),
        ("June", "06"),
        ("July", "07"),
        ("August", "08"),
        ("September", "09"),
        ("October", "10"),
        ("November", "11"),
        ("December", "12"),
    ]
    .iter()
    .cloned()
    .collect();

    for line in lines.iter().take(50) {
        if let Some(m) = ts_pattern.captures(line) {
            let time_str = m.get(1)?.as_str();
            let month = m.get(2)?.as_str();
            let day = m.get(3)?.as_str();
            let year = m.get(4)?.as_str();

            let mon = months.get(month).unwrap_or(&"00");
            let day_z = day.to_string().trim_start_matches('0').to_string();
            let time_safe = time_str.replace([':', ' '], "");
            let iso = format!("{}-{}-{}", year, mon, day_z);
            let human = format!("{}_{}_{}", iso, time_safe, mon);

            return Some((human, iso));
        }
    }

    None
}

/// Extract people mentioned in the session
fn extract_people(lines: &[String]) -> Vec<String> {
    let known_people = load_known_people();
    let username_map = load_username_map();
    let mut found: Vec<String> = Vec::new();

    let text: String = lines
        .iter()
        .take(100)
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    // Find speaker tags
    for person in &known_people {
        if let Ok(pattern) = Regex::new(&format!(r"\b{}\b", person)) {
            if pattern.is_match(&text) {
                found.push(person.clone());
            }
        }
    }

    // Find usernames in paths and map them
    if let Some(dir_match) = Regex::new(r"/Users/(\w+)/")
        .ok()
        .and_then(|r| r.captures(&text))
    {
        if let Some(username) = dir_match.get(1) {
            let username_str = username.as_str();
            if let Some(name) = username_map.get(username_str) {
                found.push(name.clone());
            }
        }
    }

    found.sort();
    found.dedup();
    found
}

/// Extract subject from first meaningful prompt
fn extract_subject(lines: &[String]) -> String {
    for line in lines {
        if let Some(stripped) = line.strip_prefix("> ") {
            let prompt = stripped.trim();
            if !prompt.is_empty() && !RE_SUBJECT_SKIP.is_match(prompt) && prompt.len() > 5 {
                // Clean for filename
                let subject = RE_CLEAN_NONWORD.replace_all(prompt, "");
                let subject = RE_CLEAN_SPACE.replace_all(&subject, "-");
                let subject = subject.trim().to_string();

                return subject.chars().take(60).collect();
            }
        }
    }

    "session".to_string()
}

/// Mega file splitter
#[derive(Debug, Clone)]
pub struct MegaFileSplitter {
    // known_people: Vec<String>, // not read; Python uses module-level constant
    // username_map: HashMap<String, String>, // not read; Python uses module-level function
}

impl MegaFileSplitter {
    /// Create a new mega file splitter
    pub fn new() -> Self {
        let _ = load_known_people();
        let _ = load_username_map();
        Self {}
    }

    /// Split mega files in a directory
    pub fn split_directory(
        &self,
        dir: &Path,
        output_dir: Option<&Path>,
        dry_run: bool,
    ) -> Result<Vec<SplitFile>> {
        let mut split_files = Vec::new();

        // Find all .txt files
        let txt_files: Vec<PathBuf> = fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map(|e| e == "txt").unwrap_or(false))
            .collect();

        for filepath in txt_files {
            let lines: Vec<String> = fs::read_to_string(&filepath)
                .map(|s| s.lines().map(String::from).collect())
                .unwrap_or_default();

            let boundaries = find_session_boundaries(&lines);

            // Skip files with fewer than 2 sessions
            if boundaries.len() < 2 {
                continue;
            }

            let result = self.split_file(&filepath, output_dir, dry_run)?;
            split_files.extend(result);
        }

        Ok(split_files)
    }

    /// Split a single mega file
    pub fn split_file(
        &self,
        filepath: &Path,
        output_dir: Option<&Path>,
        dry_run: bool,
    ) -> Result<Vec<SplitFile>> {
        let lines: Vec<String> = fs::read_to_string(filepath)?
            .lines()
            .map(String::from)
            .collect();

        let boundaries = find_session_boundaries(&lines);
        if boundaries.len() < 2 {
            return Ok(Vec::new());
        }

        // Add sentinel at end
        let mut all_boundaries = boundaries;
        all_boundaries.push(lines.len());

        let out_dir = output_dir
            .map(|p| p.to_path_buf())
            .or_else(|| filepath.parent().map(|parent| parent.to_path_buf()))
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!(
                        "cannot determine output directory for {}",
                        filepath.display()
                    ),
                )
            })?;
        let mut written = Vec::new();

        for i in 0..all_boundaries.len() - 1 {
            let start = all_boundaries[i];
            let end = all_boundaries[i + 1];
            let chunk: Vec<String> = lines[start..end].to_vec();

            if chunk.len() < 10 {
                continue; // Skip tiny fragments
            }

            let (ts_human, _ts_iso) = extract_timestamp(&chunk)
                .unwrap_or_else(|| (format!("part{:02}", i + 1), String::new()));
            let people = extract_people(&chunk);
            let subject = extract_subject(&chunk);

            // Build filename
            let ts_part = ts_human;
            let people_part = if people.is_empty() {
                "unknown".to_string()
            } else {
                people.iter().take(3).cloned().collect::<Vec<_>>().join("-")
            };

            let src_stem = filepath
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| RE_STEM_CLEAN.replace_all(s, "_"))
                .map(|s| s.chars().take(40).collect::<String>())
                .unwrap_or_else(|| "source".to_string());

            let subject_clean = RE_SUBJECT_CLEAN1.replace_all(&subject, "_");
            let subject_clean = RE_SUBJECT_CLEAN2.replace_all(&subject_clean, "_");

            let name = format!(
                "{}__{}_{}_{}.txt",
                src_stem, ts_part, people_part, subject_clean
            );
            let out_path = out_dir.join(&name);

            if dry_run {
                println!("  [{}] {}  ({} lines)", i + 1, name, chunk.len());
            } else {
                fs::write(&out_path, chunk.join("\n"))?;
                println!("  ✓ {}  ({} lines)", name, chunk.len());

                // Rename original to backup
                if output_dir.is_none() {
                    let backup_path = format!("{}.mega_backup", filepath.to_string_lossy());
                    fs::rename(filepath, Path::new(&backup_path))?;
                    println!("  → Original renamed to {}", backup_path);
                }
            }

            written.push(SplitFile {
                original_path: filepath.to_path_buf(),
                session_id: format!("{}_{}", ts_part, i),
                created_at: Utc::now(),
                output_path: out_path,
            });
        }

        Ok(written)
    }
}

impl Default for MegaFileSplitter {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of splitting a file
#[derive(Debug, Clone)]
pub struct SplitFile {
    pub original_path: PathBuf,
    pub session_id: String,
    pub created_at: DateTime<Utc>,
    #[doc(hidden)]
    pub output_path: PathBuf,
}

#[cfg(test)]
#[path = "../tests/miner_splitter.rs"]
mod tests;
