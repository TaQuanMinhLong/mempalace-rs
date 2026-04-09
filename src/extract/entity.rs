//! Entity detection - port from Python entity_detector.py
//!
//! Two-pass approach:
//!   Pass 1: Scan files, extract entity candidates with signal counts
//!   Pass 2: Score and classify each candidate as person, project, or uncertain

use regex::Regex;
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Entity extractor with two-pass detection
#[derive(Debug, Clone)]
pub struct EntityExtractor {
    person_verb_patterns: Vec<CompiledPattern>,
    pronoun_patterns: Vec<CompiledPattern>,
    dialogue_patterns: Vec<CompiledPattern>,
    project_verb_patterns: Vec<CompiledPattern>,
    stopwords: HashSet<String>,
}

#[derive(Debug, Clone)]
struct CompiledPattern {
    regex: Regex,
}

impl EntityExtractor {
    /// Create a new entity extractor
    pub fn new() -> Self {
        Self {
            person_verb_patterns: Self::compile_person_verb_patterns(),
            pronoun_patterns: Self::compile_pronoun_patterns(),
            dialogue_patterns: Self::compile_dialogue_patterns(),
            project_verb_patterns: Self::compile_project_verb_patterns(),
            stopwords: Self::build_stopwords(),
        }
    }

    fn compile_person_verb_patterns() -> Vec<CompiledPattern> {
        let patterns = [
            (r"\b{name}\s+said\b", "dialogue"),
            (r"\b{name}\s+asked\b", "dialogue"),
            (r"\b{name}\s+told\b", "dialogue"),
            (r"\b{name}\s+replied\b", "dialogue"),
            (r"\b{name}\s+laughed\b", "dialogue"),
            (r"\b{name}\s+smiled\b", "dialogue"),
            (r"\b{name}\s+cried\b", "dialogue"),
            (r"\b{name}\s+felt\b", "dialogue"),
            (r"\b{name}\s+thinks?\b", "dialogue"),
            (r"\b{name}\s+wants?\b", "dialogue"),
            (r"\b{name}\s+loves?\b", "dialogue"),
            (r"\b{name}\s+hates?\b", "dialogue"),
            (r"\b{name}\s+knows?\b", "dialogue"),
            (r"\b{name}\s+decided\b", "dialogue"),
            (r"\b{name}\s+pushed\b", "dialogue"),
            (r"\b{name}\s+wrote\b", "dialogue"),
            (r"\bhey\s+{name}\b", "addressed"),
            (r"\bthanks?\s+{name}\b", "addressed"),
            (r"\bhi\s+{name}\b", "addressed"),
            (r"\bdear\s+{name}\b", "addressed"),
        ];
        patterns
            .iter()
            .map(|(p, _m)| CompiledPattern {
                regex: Regex::new(&p.replace("{name}", r"[A-Z][a-z]{1,20}")).unwrap(),
            })
            .collect()
    }

    fn compile_pronoun_patterns() -> Vec<CompiledPattern> {
        let patterns = [
            r"\bshe\b",
            r"\bher\b",
            r"\bhers\b",
            r"\bhe\b",
            r"\bhim\b",
            r"\bhis\b",
            r"\bthey\b",
            r"\bthem\b",
            r"\btheir\b",
        ];
        patterns
            .iter()
            .map(|p| CompiledPattern {
                regex: Regex::new(p).unwrap(),
            })
            .collect()
    }

    fn compile_dialogue_patterns() -> Vec<CompiledPattern> {
        let patterns = [
            (r"^>\s*{name}[:\s]", "dialogue"),
            (r"^{name}:\s", "dialogue"),
            (r"^\[{name}\]", "dialogue"),
            (r#""{name}\s+said"#, "dialogue"),
        ];
        patterns
            .iter()
            .map(|(p, _m)| CompiledPattern {
                regex: Regex::new(&p.replace("{name}", r"[A-Z][a-z]{1,20}")).unwrap(),
            })
            .collect()
    }

    fn compile_project_verb_patterns() -> Vec<CompiledPattern> {
        let patterns = [
            (r"\bbuilding\s+{name}\b", "project_verb"),
            (r"\bbuilt\s+{name}\b", "project_verb"),
            (r"\bship(?:ping|ped)?\s+{name}\b", "project_verb"),
            (r"\blaunch(?:ing|ed)?\s+{name}\b", "project_verb"),
            (r"\bdeploy(?:ing|ed)?\s+{name}\b", "project_verb"),
            (r"\binstall(?:ing|ed)?\s+{name}\b", "project_verb"),
            (r"\bthe\s+{name}\s+architecture\b", "project_verb"),
            (r"\bthe\s+{name}\s+pipeline\b", "project_verb"),
            (r"\bthe\s+{name}\s+system\b", "project_verb"),
            (r"\bthe\s+{name}\s+repo\b", "project_verb"),
            (r"\b{name}\s+v\d+\b", "versioned"),
            (r"\bimport\s+{name}\b", "import_ref"),
        ];
        patterns
            .iter()
            .map(|(p, _m)| CompiledPattern {
                regex: Regex::new(&p.replace("{name}", r"[A-Z][a-z]{1,20}")).unwrap(),
            })
            .collect()
    }

    fn build_stopwords() -> HashSet<String> {
        let words = [
            "the",
            "a",
            "an",
            "and",
            "or",
            "but",
            "in",
            "on",
            "at",
            "to",
            "for",
            "of",
            "with",
            "by",
            "from",
            "as",
            "is",
            "was",
            "are",
            "were",
            "be",
            "been",
            "being",
            "have",
            "has",
            "had",
            "do",
            "does",
            "did",
            "will",
            "would",
            "could",
            "should",
            "may",
            "might",
            "must",
            "shall",
            "can",
            "this",
            "that",
            "these",
            "those",
            "it",
            "its",
            "they",
            "them",
            "their",
            "we",
            "our",
            "you",
            "your",
            "i",
            "my",
            "me",
            "he",
            "she",
            "his",
            "her",
            "who",
            "what",
            "when",
            "where",
            "why",
            "how",
            "which",
            "if",
            "then",
            "so",
            "not",
            "no",
            "yes",
            "ok",
            "okay",
            "just",
            "very",
            "really",
            "also",
            "already",
            "still",
            "even",
            "only",
            "here",
            "there",
            "now",
            "then",
            "too",
            "up",
            "out",
            "about",
            "like",
            "use",
            "get",
            "got",
            "make",
            "made",
            "take",
            "put",
            "come",
            "go",
            "see",
            "know",
            "think",
            "true",
            "false",
            "none",
            "null",
            "new",
            "old",
            "all",
            "any",
            "some",
            "return",
            "print",
            "def",
            "class",
            "import",
            "from",
            "step",
            "usage",
            "run",
            "check",
            "find",
            "add",
            "set",
            "list",
            "args",
            "dict",
            "str",
            "int",
            "bool",
            "path",
            "file",
            "type",
            "name",
            "note",
            "example",
            "option",
            "result",
            "error",
            "warning",
            "info",
            "every",
            "each",
            "more",
            "less",
            "next",
            "last",
            "first",
            "second",
            "stack",
            "layer",
            "mode",
            "test",
            "stop",
            "start",
            "copy",
            "move",
            "source",
            "target",
            "output",
            "input",
            "data",
            "item",
            "key",
            "value",
            "returns",
            "raises",
            "yields",
            "self",
            "cls",
            "kwargs",
            "world",
            "well",
            "want",
            "topic",
            "choose",
            "social",
            "cars",
            "phones",
            "healthcare",
            "ex",
            "machina",
            "deus",
            "human",
            "humans",
            "people",
            "things",
            "something",
            "nothing",
            "everything",
            "anything",
            "someone",
            "everyone",
            "anyone",
            "way",
            "time",
            "day",
            "life",
            "place",
            "thing",
            "part",
            "kind",
            "sort",
            "case",
            "point",
            "idea",
            "fact",
            "sense",
            "question",
            "answer",
            "reason",
            "number",
            "version",
            "system",
            "hey",
            "hi",
            "hello",
            "thanks",
            "thank",
            "right",
            "let",
            "click",
            "hit",
            "press",
            "tap",
            "drag",
            "drop",
            "open",
            "close",
            "save",
            "load",
            "launch",
            "install",
            "download",
            "upload",
            "scroll",
            "select",
            "enter",
            "submit",
            "cancel",
            "confirm",
            "delete",
            "paste",
            "write",
            "read",
            "search",
            "show",
            "hide",
            "desktop",
            "documents",
            "downloads",
            "users",
            "home",
            "library",
            "applications",
            "preferences",
            "settings",
            "terminal",
            "actor",
            "vector",
            "remote",
            "control",
            "duration",
            "fetch",
            "agents",
            "tools",
            "others",
            "guards",
            "ethics",
            "regulation",
            "learning",
            "thinking",
            "memory",
            "language",
            "intelligence",
            "technology",
            "society",
            "culture",
            "future",
            "history",
            "science",
            "model",
            "models",
            "network",
            "networks",
            "training",
            "inference",
        ];
        words.iter().map(|w| w.to_string()).collect()
    }

    /// Extract entities from file paths (prose files only)
    pub fn detect_from_files(&self, file_paths: &[&Path], max_files: usize) -> DetectedEntities {
        let mut all_text = String::new();
        let mut all_lines = Vec::new();
        let mut files_read = 0;
        const MAX_BYTES: usize = 5_000;

        for filepath in file_paths {
            if files_read >= max_files {
                break;
            }
            if let Ok(content) = std::fs::read_to_string(filepath) {
                let content = content.chars().take(MAX_BYTES).collect::<String>();
                all_text.push_str(&content);
                all_lines.extend(content.lines().map(|l| l.to_string()));
                files_read += 1;
            }
        }

        // Pass 1: Extract candidates
        let candidates = self.extract_candidates(&all_text);

        if candidates.is_empty() {
            return DetectedEntities {
                people: vec![],
                projects: vec![],
                uncertain: vec![],
            };
        }

        // Pass 2: Score and classify
        let mut people = Vec::new();
        let mut projects = Vec::new();
        let mut uncertain = Vec::new();

        let mut sorted_candidates: Vec<_> = candidates.into_iter().collect();
        sorted_candidates.sort_by_key(|&(_, v)| Reverse(v));

        for (name, frequency) in sorted_candidates {
            let scores = self.score_entity(&name, &all_text, &all_lines);
            let entity = self.classify_entity(&name, frequency, &scores);

            match entity.entity_type {
                EntityType::Person => people.push(entity),
                EntityType::Project => projects.push(entity),
                EntityType::Uncertain => uncertain.push(entity),
            }
        }

        // Sort by confidence descending
        people.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        projects.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        uncertain.sort_by_key(|v| Reverse(v.frequency));

        DetectedEntities {
            people: people.into_iter().take(15).collect(),
            projects: projects.into_iter().take(10).collect(),
            uncertain: uncertain.into_iter().take(8).collect(),
        }
    }

    /// Extract candidates from text (Pass 1)
    fn extract_candidates(&self, text: &str) -> HashMap<String, usize> {
        let mut counts = HashMap::new();

        // Single-word capitalized names
        let single_word = Regex::new(r"\b([A-Z][a-z]{1,19})\b").unwrap();
        for cap in single_word.find_iter(text) {
            let word = cap.as_str();
            let word_lower = word.to_lowercase();
            if !self.stopwords.contains(&word_lower) && word.len() > 1 {
                *counts.entry(word.to_string()).or_insert(0) += 1;
            }
        }

        // Multi-word proper nouns
        let multi_word = Regex::new(r"\b([A-Z][a-z]+(?:\s+[A-Z][a-z]+)+)\b").unwrap();
        for cap in multi_word.find_iter(text) {
            let phrase = cap.as_str();
            let words: Vec<_> = phrase.split_whitespace().collect();
            if !words
                .iter()
                .any(|w| self.stopwords.contains(&w.to_lowercase()))
            {
                *counts.entry(phrase.to_string()).or_insert(0) += 1;
            }
        }

        // Filter: must appear at least 3 times
        counts.retain(|_, count| *count >= 3);
        counts
    }

    /// Score a candidate entity (Pass 2)
    fn score_entity(&self, name: &str, text: &str, lines: &[String]) -> EntityScores {
        // Build patterns for this name
        let name_escaped = regex::escape(name);

        // Person signals: dialogue markers
        let mut ps = 0;
        let mut p_sig = Vec::new();

        for pattern in &self.dialogue_patterns {
            let regex = Regex::new(
                &pattern
                    .regex
                    .as_str()
                    .replace("[A-Z][a-z]{1,20}", &name_escaped),
            )
            .unwrap();
            let matches = regex.find_iter(text).count();
            if matches > 0 {
                ps += matches * 3;
                p_sig.push(format!("dialogue marker ({}x)", matches));
            }
        }

        // Person verb patterns
        for pattern in &self.person_verb_patterns {
            let regex = Regex::new(
                &pattern
                    .regex
                    .as_str()
                    .replace("[A-Z][a-z]{1,20}", &name_escaped),
            )
            .unwrap();
            let matches = regex.find_iter(text).count();
            if matches > 0 {
                ps += matches * 2;
                p_sig.push(format!("'{} ...' action ({}x)", name, matches));
            }
        }

        // Pronoun proximity
        let name_lower = name.to_lowercase();
        let name_line_indices: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.to_lowercase().contains(&name_lower))
            .map(|(i, _)| i)
            .collect();

        let mut pronoun_hits = 0;
        for idx in &name_line_indices {
            let window_start = idx.saturating_sub(2);
            let window_end = (*idx + 3).min(lines.len());
            let window_text = lines[window_start..window_end].join(" ").to_lowercase();

            for pattern in &self.pronoun_patterns {
                if pattern.regex.is_match(&window_text) {
                    pronoun_hits += 1;
                    break;
                }
            }
        }

        if pronoun_hits > 0 {
            ps += pronoun_hits * 2;
            p_sig.push(format!("pronoun nearby ({}x)", pronoun_hits));
        }

        // Direct address
        let direct_regex =
            Regex::new(&format!(r"\b(hey|thanks?|hi|dear)\s+{}", name_escaped)).unwrap();
        let direct_matches = direct_regex.find_iter(text).count();
        if direct_matches > 0 {
            ps += direct_matches * 4;
            p_sig.push(format!("addressed directly ({}x)", direct_matches));
        }

        let person_score: usize = ps;
        let person_signals: Vec<String> = p_sig;

        // Project signals
        let mut prs = 0;
        let mut pr_sig = Vec::new();

        for pattern in &self.project_verb_patterns {
            let regex = Regex::new(
                &pattern
                    .regex
                    .as_str()
                    .replace("[A-Z][a-z]{1,20}", &name_escaped),
            )
            .unwrap();
            let matches = regex.find_iter(text).count();
            if matches > 0 {
                prs += matches * 2;
                pr_sig.push(format!("project verb ({}x)", matches));
            }
        }

        EntityScores {
            person_score,
            project_score: prs,
            person_signals,
            project_signals: pr_sig,
        }
    }

    /// Classify entity based on scores
    fn classify_entity(&self, name: &str, frequency: usize, scores: &EntityScores) -> Entity {
        let ps = scores.person_score;
        let prs = scores.project_score;
        let total = ps + prs;

        let entity_type;
        let confidence;
        let mut signals: Vec<String>;

        if total == 0 {
            // No strong signals - uncertain
            confidence = (frequency as f64 / 50.0).min(0.4);
            entity_type = EntityType::Uncertain;
            signals = vec![format!("appears {}x, no strong type signals", frequency)];
        } else {
            let person_ratio = ps as f64 / total as f64;

            // Count signal categories for person classification
            let signal_categories: HashSet<_> = scores
                .person_signals
                .iter()
                .filter_map(|s| {
                    if s.contains("dialogue") {
                        Some("dialogue")
                    } else if s.contains("action") {
                        Some("action")
                    } else if s.contains("pronoun") {
                        Some("pronoun")
                    } else if s.contains("addressed") {
                        Some("addressed")
                    } else {
                        None
                    }
                })
                .collect();

            let has_two_signal_types = signal_categories.len() >= 2;

            if person_ratio >= 0.7 && has_two_signal_types && ps >= 5 {
                entity_type = EntityType::Person;
                confidence = (0.5 + person_ratio * 0.5).min(0.99);
                signals = if scores.person_signals.is_empty() {
                    vec![format!("appears {}x", frequency)]
                } else {
                    scores.person_signals.clone()
                };
            } else if person_ratio >= 0.7 && (!has_two_signal_types || ps < 5) {
                // Pronoun-only match - downgrade to uncertain
                entity_type = EntityType::Uncertain;
                confidence = 0.4;
                signals = scores
                    .person_signals
                    .iter()
                    .chain(std::iter::once(&format!(
                        "appears {}x — pronoun-only match",
                        frequency
                    )))
                    .cloned()
                    .collect();
            } else if person_ratio <= 0.3 {
                entity_type = EntityType::Project;
                confidence = (0.5 + (1.0 - person_ratio) * 0.5).min(0.99);
                signals = if scores.project_signals.is_empty() {
                    vec![format!("appears {}x", frequency)]
                } else {
                    scores.project_signals.clone()
                };
            } else {
                entity_type = EntityType::Uncertain;
                confidence = 0.5;
                signals = scores
                    .person_signals
                    .iter()
                    .chain(scores.project_signals.iter())
                    .take(3)
                    .cloned()
                    .collect();
                signals.push("mixed signals — needs review".to_string());
            }
        }

        Entity {
            name: name.to_string(),
            entity_type,
            confidence: (confidence * 100.0).round() / 100.0,
            frequency,
            signals,
        }
    }
}

impl Default for EntityExtractor {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Scores for an entity
#[derive(Debug, Clone)]
struct EntityScores {
    person_score: usize,
    project_score: usize,
    person_signals: Vec<String>,
    project_signals: Vec<String>,
}

/// Type of entity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    Person,
    Project,
    Uncertain,
}

/// Detected entity
#[derive(Debug, Clone)]
pub struct Entity {
    pub name: String,
    pub entity_type: EntityType,
    pub confidence: f64,
    pub frequency: usize,
    pub signals: Vec<String>,
}

impl Entity {
    /// Format entity as a display string
    #[inline]
    pub fn display(&self) -> String {
        let type_str = match self.entity_type {
            EntityType::Person => "person",
            EntityType::Project => "project",
            EntityType::Uncertain => "uncertain",
        };
        format!(
            "{} ({}): confidence={:.2}, freq={}, signals={:?}",
            self.name, type_str, self.confidence, self.frequency, self.signals
        )
    }
}

/// Result of entity detection
#[derive(Debug, Clone)]
pub struct DetectedEntities {
    pub people: Vec<Entity>,
    pub projects: Vec<Entity>,
    pub uncertain: Vec<Entity>,
}

impl DetectedEntities {
    /// Get all entities
    #[inline]
    pub fn all(&self) -> Vec<Entity> {
        let mut all = Vec::new();
        all.extend(self.people.clone());
        all.extend(self.projects.clone());
        all.extend(self.uncertain.clone());
        all
    }

    /// Get person names
    #[inline]
    pub fn person_names(&self) -> Vec<String> {
        self.people.iter().map(|e| e.name.clone()).collect()
    }

    /// Get project names
    #[inline]
    pub fn project_names(&self) -> Vec<String> {
        self.projects.iter().map(|e| e.name.clone()).collect()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
#[path = "../tests/extract_entity.rs"]
mod tests;
