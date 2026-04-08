//! General memory extraction - port from Python general_extractor.py
//!
//! Extract 5 types of memories from text:
//! 1. DECISIONS - "we went with X because Y", choices made
//! 2. PREFERENCES - "always use X", "never do Y", "I prefer Z"
//! 3. MILESTONES - breakthroughs, things that finally worked
//! 4. PROBLEMS - what broke, what fixed it, root causes
//! 5. EMOTIONAL - feelings, vulnerability, relationships

use regex::Regex;
use std::collections::{HashMap, HashSet};

/// General extractor for 5 memory types
#[derive(Debug, Clone)]
pub struct GeneralExtractor {
    decision_patterns: Vec<Regex>,
    preference_patterns: Vec<Regex>,
    milestone_patterns: Vec<Regex>,
    problem_patterns: Vec<Regex>,
    emotional_patterns: Vec<Regex>,
    code_line_patterns: Vec<Regex>,
    positive_words: HashMap<String, bool>,
    negative_words: HashMap<String, bool>,
}

impl GeneralExtractor {
    /// Create a new general extractor
    pub fn new() -> Self {
        Self {
            decision_patterns: Self::compile_patterns(&[
                r"\blet'?s (use|go with|try|pick|choose|switch to)\b",
                r"\bwe (should|decided|chose|went with|picked|settled on)\b",
                r"\bi'?m going (to|with)\b",
                r"\bbetter (to|than|approach|option|choice)\b",
                r"\binstead of\b",
                r"\brather than\b",
                r"\bthe reason (is|was|being)\b",
                r"\bbecause\b",
                r"\btrade-?off\b",
                r"\bpros and cons\b",
                r"\bover\b.*\bbecause\b",
                r"\barchitecture\b",
                r"\bapproach\b",
                r"\bstrategy\b",
                r"\bpattern\b",
                r"\bstack\b",
                r"\bframework\b",
                r"\binfrastructure\b",
                r"\bset (it |this )?to\b",
                r"\bconfigure\b",
                r"\bdefault\b",
            ]),
            preference_patterns: Self::compile_patterns(&[
                r"\bi prefer\b",
                r"\balways use\b",
                r"\bnever use\b",
                r"\bdon'?t (ever |like to )?(use|do|mock|stub|import)\b",
                r"\bi like (to|when|how)\b",
                r"\bi hate (when|how|it when)\b",
                r"\bplease (always|never|don'?t)\b",
                r"\bmy (rule|preference|style|convention) is\b",
                r"\bwe (always|never)\b",
                r"\bfunctional\b.*\bstyle\b",
                r"\bimperative\b",
                r"\bsnake_?case\b",
                r"\bcamel_?case\b",
                r"\btabs\b.*\bspaces\b",
                r"\bspaces\b.*\btabs\b",
                r"\buse\b.*\binstead of\b",
            ]),
            milestone_patterns: Self::compile_patterns(&[
                r"\bit works\b",
                r"\bit worked\b",
                r"\bgot it working\b",
                r"\bfixed\b",
                r"\bsolved\b",
                r"\bbreakthrough\b",
                r"\bfigured (it )?out\b",
                r"\bnailed it\b",
                r"\bcracked (it|the)\b",
                r"\bfinally\b",
                r"\bfirst time\b",
                r"\bfirst ever\b",
                r"\bnever (done|been|had) before\b",
                r"\bdiscovered\b",
                r"\brealized\b",
                r"\bfound (out|that)\b",
                r"\bturns out\b",
                r"\bthe key (is|was|insight)\b",
                r"\bthe trick (is|was)\b",
                r"\bnow i (understand|see|get it)\b",
                r"\bbuilt\b",
                r"\bcreated\b",
                r"\bimplemented\b",
                r"\bshipped\b",
                r"\blaunched\b",
                r"\bdeployed\b",
                r"\breleased\b",
                r"\bprototype\b",
                r"\bproof of concept\b",
                r"\bdemo\b",
                r"\bversion \d",
                r"\bv\d+\.\d+",
                r"\d+x (compression|faster|slower|better|improvement|reduction)",
                r"\d+% (reduction|improvement|faster|better|smaller)",
            ]),
            problem_patterns: Self::compile_patterns(&[
                r"\b(bug|error|crash|fail|broke|broken|issue|problem)\b",
                r"\bdoesn'?t work\b",
                r"\bnot working\b",
                r"\bwon'?t\b.*\bwork\b",
                r"\bkeeps? (failing|crashing|breaking|erroring)\b",
                r"\broot cause\b",
                r"\bthe (problem|issue|bug) (is|was)\b",
                r"\bturns out\b.*\b(was|because|due to)\b",
                r"\bthe fix (is|was)\b",
                r"\bworkaround\b",
                r"\bthat'?s why\b",
                r"\bthe reason it\b",
                r"\bfixed (it |the |by )\b",
                r"\bsolution (is|was)\b",
                r"\bresolved\b",
                r"\bpatched\b",
                r"\bthe answer (is|was)\b",
                r"\b(had|need) to\b.*\binstead\b",
            ]),
            emotional_patterns: Self::compile_patterns(&[
                r"\blove\b",
                r"\bscared\b",
                r"\bafraid\b",
                r"\bproud\b",
                r"\bhurt\b",
                r"\bhappy\b",
                r"\bsad\b",
                r"\bcry\b",
                r"\bcrying\b",
                r"\bmiss\b",
                r"\bsorry\b",
                r"\bgrateful\b",
                r"\bangry\b",
                r"\bworried\b",
                r"\blonely\b",
                r"\bbeautiful\b",
                r"\bamazing\b",
                r"\bwonderful\b",
                r"i feel",
                r"i'm scared",
                r"i love you",
                r"i'm sorry",
                r"i can't",
                r"i wish",
                r"i miss",
                r"i need",
                r"never told anyone",
                r"nobody knows",
                r"\*[^*]+\*",
            ]),
            code_line_patterns: Self::compile_code_patterns(),
            positive_words: Self::build_word_set(&[
                "pride",
                "proud",
                "joy",
                "happy",
                "love",
                "loving",
                "beautiful",
                "amazing",
                "wonderful",
                "incredible",
                "fantastic",
                "brilliant",
                "perfect",
                "excited",
                "thrilled",
                "grateful",
                "warm",
                "breakthrough",
                "success",
                "works",
                "working",
                "solved",
                "fixed",
                "nailed",
                "heart",
                "hug",
                "precious",
                "adore",
            ]),
            negative_words: Self::build_word_set(&[
                "bug",
                "error",
                "crash",
                "crashing",
                "crashed",
                "fail",
                "failed",
                "failing",
                "failure",
                "broken",
                "broke",
                "breaking",
                "breaks",
                "issue",
                "problem",
                "wrong",
                "stuck",
                "blocked",
                "unable",
                "impossible",
                "missing",
                "terrible",
                "horrible",
                "awful",
                "worse",
                "worst",
                "panic",
                "disaster",
                "mess",
            ]),
        }
    }

    fn compile_patterns(patterns: &[&str]) -> Vec<Regex> {
        patterns.iter().filter_map(|p| Regex::new(p).ok()).collect()
    }

    fn compile_code_patterns() -> Vec<Regex> {
        vec![
            Regex::new(r"^\s*[\$#]\s").unwrap(),
            Regex::new(r"^\s*(cd|source|echo|export|pip|npm|git|python|bash|curl|wget|mkdir|rm|cp|mv|ls|cat|grep|find|chmod|sudo|brew|docker)\s").unwrap(),
            Regex::new(r"^\s*```").unwrap(),
            Regex::new(r"^\s*(import|from|def|class|function|const|let|var|return)\s").unwrap(),
            Regex::new(r"^\s*[A-Z_]{2,}=").unwrap(),
            Regex::new(r"^\s*\|").unwrap(),
            Regex::new(r"^\s*[-]{2,}").unwrap(),
            Regex::new(r"^\s*[{}\[\]]\s*$").unwrap(),
            Regex::new(r"^\s*(if|for|while|try|except|elif|else:)\b").unwrap(),
            Regex::new(r"^\s*\w+\.\w+\(").unwrap(),
            Regex::new(r"^\s*\w+ = \w+\.\w+").unwrap(),
        ]
    }

    fn build_word_set(words: &[&str]) -> HashMap<String, bool> {
        words.iter().map(|w| (w.to_string(), true)).collect()
    }

    /// Extract memories from text
    pub fn extract(&self, text: &str, min_confidence: f64) -> Vec<MemoryChunk> {
        let paragraphs = self.split_into_segments(text);
        let mut memories = Vec::new();

        for para in paragraphs {
            if para.trim().len() < 20 {
                continue;
            }

            let prose = self.extract_prose(&para);

            // Score against all types
            let scores = self.score_all_types(&prose);

            if scores.is_empty() {
                continue;
            }

            // Length bonus
            let length_bonus = if para.len() > 500 {
                2
            } else if para.len() > 200 {
                1
            } else {
                0
            };

            let max_type = scores
                .iter()
                .max_by(|a, b| (*a.1 as i32 + length_bonus).cmp(&(*b.1 as i32 + length_bonus)))
                .map(|(t, _)| *t);

            if let Some(max_type) = max_type {
                let max_score =
                    scores.get(&max_type).copied().unwrap_or(0) as f64 + length_bonus as f64;

                // Disambiguate
                let final_type = self.disambiguate(&max_type, &prose, &scores);

                // Confidence
                let confidence = (max_score / 5.0).min(1.0);
                if confidence < min_confidence {
                    continue;
                }

                memories.push(MemoryChunk {
                    content: para.trim().to_string(),
                    memory_type: final_type,
                    chunk_index: memories.len(),
                });
            }
        }

        memories
    }

    /// Score text against all marker types
    fn score_all_types(&self, text: &str) -> HashMap<MemoryType, usize> {
        let mut scores = HashMap::new();

        let text_lower = text.to_lowercase();

        if self.count_matches(&text_lower, &self.decision_patterns) > 0 {
            scores.insert(
                MemoryType::Decision,
                self.count_matches(&text_lower, &self.decision_patterns),
            );
        }
        if self.count_matches(&text_lower, &self.preference_patterns) > 0 {
            scores.insert(
                MemoryType::Preference,
                self.count_matches(&text_lower, &self.preference_patterns),
            );
        }
        if self.count_matches(&text_lower, &self.milestone_patterns) > 0 {
            scores.insert(
                MemoryType::Milestone,
                self.count_matches(&text_lower, &self.milestone_patterns),
            );
        }
        if self.count_matches(&text_lower, &self.problem_patterns) > 0 {
            scores.insert(
                MemoryType::Problem,
                self.count_matches(&text_lower, &self.problem_patterns),
            );
        }
        if self.count_matches(&text_lower, &self.emotional_patterns) > 0 {
            scores.insert(
                MemoryType::Emotional,
                self.count_matches(&text_lower, &self.emotional_patterns),
            );
        }

        scores
    }

    fn count_matches(&self, text: &str, patterns: &[Regex]) -> usize {
        patterns.iter().map(|p| p.find_iter(text).count()).sum()
    }

    /// Get sentiment of text
    fn get_sentiment(&self, text: &str) -> &'static str {
        let words: HashSet<_> = text.split_whitespace().map(|w| w.to_lowercase()).collect();

        let pos = words
            .iter()
            .filter(|w| self.positive_words.contains_key(*w))
            .count();
        let neg = words
            .iter()
            .filter(|w| self.negative_words.contains_key(*w))
            .count();

        if pos > neg {
            "positive"
        } else if neg > pos {
            "negative"
        } else {
            "neutral"
        }
    }

    /// Check if text describes a resolved problem
    fn has_resolution(&self, text: &str) -> bool {
        let text_lower = text.to_lowercase();
        let patterns = [
            r"\bfixed\b",
            r"\bsolved\b",
            r"\bresolved\b",
            r"\bpatched\b",
            r"\bgot it working\b",
            r"\bit works\b",
            r"\bnailed it\b",
            r"\bfigured (it )?out\b",
            r"\bthe (fix|answer|solution)\b",
        ];

        patterns
            .iter()
            .any(|p| Regex::new(p).unwrap().is_match(&text_lower))
    }

    /// Disambiguate memory type using sentiment and resolution
    fn disambiguate(
        &self,
        memory_type: &MemoryType,
        text: &str,
        scores: &HashMap<MemoryType, usize>,
    ) -> MemoryType {
        let sentiment = self.get_sentiment(text);

        // Resolved problems are milestones
        if *memory_type == MemoryType::Problem && self.has_resolution(text) {
            if scores.get(&MemoryType::Emotional).copied().unwrap_or(0) > 0
                && sentiment == "positive"
            {
                return MemoryType::Emotional;
            }
            return MemoryType::Milestone;
        }

        // Problem + positive sentiment => milestone or emotional
        if *memory_type == MemoryType::Problem && sentiment == "positive" {
            if scores.get(&MemoryType::Milestone).copied().unwrap_or(0) > 0 {
                return MemoryType::Milestone;
            }
            if scores.get(&MemoryType::Emotional).copied().unwrap_or(0) > 0 {
                return MemoryType::Emotional;
            }
        }

        *memory_type
    }

    /// Check if a line is code
    fn is_code_line(&self, line: &str) -> bool {
        let stripped = line.trim();
        if stripped.is_empty() {
            return false;
        }

        for pattern in &self.code_line_patterns {
            if pattern.is_match(stripped) {
                return true;
            }
        }

        // Alpha ratio check
        let alpha_ratio = stripped.chars().filter(|c| c.is_alphabetic()).count() as f64
            / stripped.len().max(1) as f64;
        if alpha_ratio < 0.4 && stripped.len() > 10 {
            return true;
        }

        false
    }

    /// Extract prose lines (skip code)
    fn extract_prose(&self, text: &str) -> String {
        let lines = text.split('\n');
        let mut prose = Vec::new();
        let mut in_code = false;

        for line in lines {
            if line.trim().starts_with("```") {
                in_code = !in_code;
                continue;
            }
            if in_code {
                continue;
            }
            if !self.is_code_line(line) {
                prose.push(line);
            }
        }

        let result = prose.join("\n").trim().to_string();
        if result.is_empty() {
            text.to_string()
        } else {
            result
        }
    }

    /// Split text into segments
    fn split_into_segments(&self, text: &str) -> Vec<String> {
        let lines: Vec<_> = text.split('\n').collect();

        // Check for speaker-turn markers
        let turn_patterns = [
            Regex::new(r"^>\s").unwrap(),
            Regex::new(r"^(Human|User|Q)\s*:").unwrap(),
            Regex::new(r"^(Assistant|AI|A|Claude|ChatGPT)\s*:").unwrap(),
        ];

        let turn_count = lines
            .iter()
            .filter(|line| {
                let stripped = line.trim();
                turn_patterns.iter().any(|p| p.is_match(stripped))
            })
            .count();

        // If enough turn markers, split by turns
        if turn_count >= 3 {
            return self.split_by_turns(&lines, &turn_patterns);
        }

        // Fallback: paragraph splitting
        let paragraphs: Vec<_> = text
            .split("\n\n")
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect();

        // If single giant block, chunk by line groups
        if paragraphs.len() <= 1 && lines.len() > 20 {
            let mut segments = Vec::new();
            for i in (0..lines.len()).step_by(25) {
                let end = (i + 25).min(lines.len());
                let group = lines[i..end].join("\n").trim().to_string();
                if !group.is_empty() {
                    segments.push(group);
                }
            }
            return segments;
        }

        paragraphs
    }

    /// Split lines into segments at speaker turn boundaries
    fn split_by_turns<T: AsRef<str>>(&self, lines: &[T], turn_patterns: &[Regex]) -> Vec<String> {
        let mut segments = Vec::new();
        let mut current = Vec::new();

        for line in lines {
            let line = line.as_ref();
            let stripped = line.trim();
            let is_turn = turn_patterns.iter().any(|p| p.is_match(stripped));

            if is_turn && !current.is_empty() {
                segments.push(current.join("\n"));
                current = vec![line.to_string()];
            } else {
                current.push(line.to_string());
            }
        }

        if !current.is_empty() {
            segments.push(current.join("\n"));
        }

        segments
    }
}

impl Default for GeneralExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Type of memory
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryType {
    Decision,
    Preference,
    Milestone,
    Problem,
    Emotional,
}

impl MemoryType {
    /// Get display name
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryType::Decision => "decision",
            MemoryType::Preference => "preference",
            MemoryType::Milestone => "milestone",
            MemoryType::Problem => "problem",
            MemoryType::Emotional => "emotional",
        }
    }
}

/// Memory chunk
#[derive(Debug, Clone)]
pub struct MemoryChunk {
    pub content: String,
    pub memory_type: MemoryType,
    pub chunk_index: usize,
}

impl MemoryChunk {
    /// Get memory type as string
    pub fn memory_type_str(&self) -> &'static str {
        self.memory_type.as_str()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
#[path = "../tests/extract_general.rs"]
mod tests;
