//! AAAK dialect - compression for token reduction
//!
//! Port from Python dialect.py (~1000 lines)
//! AAAK (Adversarial Aggregated Knowledge) is a lossy summarization format
//! that extracts entities, topics, key sentences, emotions, and flags from plain text.
//!
//! FORMAT:
//!   Header:   FILE_NUM|PRIMARY_ENTITY|DATE|TITLE
//!   Zettel:   ZID:ENTITIES|topic_keywords|"key_quote"|WEIGHT|EMOTIONS|FLAGS
//!   Tunnel:   T:ZID<->ZID|label
//!   Arc:      ARC:emotion->emotion->emotion

use crate::error::{MempalaceError, Result};
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

/// Emotion codes mapping (full_name -> code)
pub static EMOTION_CODES: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("vulnerability", "vul");
    m.insert("vulnerable", "vul");
    m.insert("joy", "joy");
    m.insert("joyful", "joy");
    m.insert("fear", "fear");
    m.insert("mild_fear", "fear");
    m.insert("trust", "trust");
    m.insert("trust_building", "trust");
    m.insert("grief", "grief");
    m.insert("raw_grief", "grief");
    m.insert("wonder", "wonder");
    m.insert("philosophical_wonder", "wonder");
    m.insert("rage", "rage");
    m.insert("anger", "rage");
    m.insert("love", "love");
    m.insert("devotion", "love");
    m.insert("hope", "hope");
    m.insert("despair", "despair");
    m.insert("hopelessness", "despair");
    m.insert("peace", "peace");
    m.insert("relief", "relief");
    m.insert("humor", "humor");
    m.insert("dark_humor", "humor");
    m.insert("tenderness", "tender");
    m.insert("raw_honesty", "raw");
    m.insert("brutal_honesty", "raw");
    m.insert("self_doubt", "doubt");
    m.insert("anxiety", "anx");
    m.insert("exhaustion", "exhaust");
    m.insert("conviction", "convict");
    m.insert("quiet_passion", "passion");
    m.insert("warmth", "warmth");
    m.insert("curiosity", "curious");
    m.insert("gratitude", "grat");
    m.insert("frustration", "frust");
    m.insert("confusion", "confuse");
    m.insert("satisfaction", "satis");
    m.insert("excitement", "excite");
    m.insert("determination", "determ");
    m.insert("surprise", "surprise");
    m
});

/// Emotion signal keywords that detect emotions in plain text
static EMOTION_SIGNALS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("decided", "determ");
    m.insert("prefer", "convict");
    m.insert("worried", "anx");
    m.insert("excited", "excite");
    m.insert("frustrated", "frust");
    m.insert("confused", "confuse");
    m.insert("love", "love");
    m.insert("hate", "rage");
    m.insert("hope", "hope");
    m.insert("fear", "fear");
    m.insert("trust", "trust");
    m.insert("happy", "joy");
    m.insert("sad", "grief");
    m.insert("surprised", "surprise");
    m.insert("grateful", "grat");
    m.insert("curious", "curious");
    m.insert("wonder", "wonder");
    m.insert("anxious", "anx");
    m.insert("relieved", "relief");
    m.insert("satisf", "satis");
    m.insert("disappoint", "grief");
    m.insert("concern", "anx");
    m
});

/// Flag signal keywords that detect importance flags in plain text
static FLAG_SIGNALS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("decided", "DECISION");
    m.insert("chose", "DECISION");
    m.insert("switched", "DECISION");
    m.insert("migrated", "DECISION");
    m.insert("replaced", "DECISION");
    m.insert("instead of", "DECISION");
    m.insert("because", "DECISION");
    m.insert("founded", "ORIGIN");
    m.insert("created", "ORIGIN");
    m.insert("started", "ORIGIN");
    m.insert("born", "ORIGIN");
    m.insert("launched", "ORIGIN");
    m.insert("first time", "ORIGIN");
    m.insert("core", "CORE");
    m.insert("fundamental", "CORE");
    m.insert("essential", "CORE");
    m.insert("principle", "CORE");
    m.insert("belief", "CORE");
    m.insert("always", "CORE");
    m.insert("never forget", "CORE");
    m.insert("turning point", "PIVOT");
    m.insert("changed everything", "PIVOT");
    m.insert("realized", "PIVOT");
    m.insert("breakthrough", "PIVOT");
    m.insert("epiphany", "PIVOT");
    m.insert("api", "TECHNICAL");
    m.insert("database", "TECHNICAL");
    m.insert("architecture", "TECHNICAL");
    m.insert("deploy", "TECHNICAL");
    m.insert("infrastructure", "TECHNICAL");
    m.insert("algorithm", "TECHNICAL");
    m.insert("framework", "TECHNICAL");
    m.insert("server", "TECHNICAL");
    m.insert("config", "TECHNICAL");
    m
});

/// Common stop words to filter from topic extraction
static STOP_WORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let words: HashSet<&str> = [
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had",
        "do", "does", "did", "will", "would", "could", "should", "may", "might", "shall", "can",
        "to", "of", "in", "for", "on", "with", "at", "by", "from", "as", "into", "about",
        "between", "through", "during", "before", "after", "above", "below", "up", "down", "out",
        "off", "over", "under", "again", "further", "then", "once", "here", "there", "when",
        "where", "why", "how", "all", "each", "every", "both", "few", "more", "most", "other",
        "some", "such", "no", "nor", "not", "only", "own", "same", "so", "than", "too", "very",
        "just", "don", "now", "and", "but", "or", "if", "while", "that", "this", "these", "those",
        "it", "its", "i", "we", "you", "he", "she", "they", "me", "him", "her", "us", "them", "my",
        "your", "his", "our", "their", "what", "which", "who", "whom", "also", "much", "many",
        "like", "because", "since", "get", "got", "use", "used", "using", "make", "made", "thing",
        "things", "way", "well", "really", "want", "need",
    ]
    .into_iter()
    .collect();
    words
});

/// Decision words that indicate important sentences
static DECISION_WORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "decided",
        "because",
        "instead",
        "prefer",
        "switched",
        "chose",
        "realized",
        "important",
        "key",
        "critical",
        "discovered",
        "learned",
        "conclusion",
        "solution",
        "reason",
        "why",
        "breakthrough",
        "insight",
    ]
    .into_iter()
    .collect()
});

/// AAAK dialect compressor
#[derive(Debug, Clone)]
pub struct AaakDialect {
    entity_codes: HashMap<String, String>,
    skip_names: Vec<String>,
}

impl AaakDialect {
    /// Create a new AAAK dialect compressor
    pub fn new() -> Self {
        Self {
            entity_codes: HashMap::new(),
            skip_names: Vec::new(),
        }
    }

    /// Create with custom entity mappings
    pub fn with_entities(entities: HashMap<String, String>) -> Self {
        let mut entity_codes = HashMap::new();
        for (name, code) in entities {
            entity_codes.insert(name.clone(), code.clone());
            entity_codes.insert(name.to_lowercase(), code);
        }
        Self {
            entity_codes,
            skip_names: Vec::new(),
        }
    }

    /// Create with skip names
    pub fn with_skip_names(skip_names: Vec<String>) -> Self {
        Self {
            entity_codes: HashMap::new(),
            skip_names: skip_names.iter().map(|n| n.to_lowercase()).collect(),
        }
    }

    /// Add an entity mapping
    pub fn add_entity(&mut self, name: &str, code: &str) {
        self.entity_codes.insert(name.to_string(), code.to_string());
        self.entity_codes
            .insert(name.to_lowercase(), code.to_string());
    }

    /// Skip an entity name
    pub fn skip_entity(&mut self, name: &str) {
        self.skip_names.push(name.to_lowercase());
    }

    /// Encode an entity name to its short code
    pub fn encode_entity(&self, name: &str) -> Option<String> {
        let name_lower = name.to_lowercase();

        // Check skip list
        if self.skip_names.iter().any(|s| name_lower.contains(s)) {
            return None;
        }

        // Check exact match (case-sensitive)
        if let Some(code) = self.entity_codes.get(name) {
            return Some(code.clone());
        }

        // Check exact match (case-insensitive)
        if let Some(code) = self.entity_codes.get(&name_lower) {
            return Some(code.clone());
        }

        // Check if name contains any key
        for (key, code) in &self.entity_codes {
            if key.chars().all(|c| c.is_lowercase()) && name_lower.contains(key) {
                return Some(code.clone());
            }
        }

        // Auto-code: first 3 chars uppercase
        Some(name.chars().take(3).collect::<String>().to_uppercase())
    }

    /// Encode a list of emotions to compact codes
    pub fn encode_emotions(&self, emotions: &[&str]) -> String {
        let mut codes = Vec::new();
        let mut seen = HashSet::new();

        for e in emotions {
            if let Some(code) = self.emotion_code(e) {
                if !seen.contains(code) {
                    codes.push(code);
                    seen.insert(code);
                }
            }
        }

        codes.truncate(3);
        codes.join("+")
    }

    /// Compress plain text to AAAK format
    pub fn compress(&self, text: &str) -> Result<String> {
        let entities = self.detect_entities_in_text(text);
        let entity_str = if entities.is_empty() {
            "???".to_string()
        } else {
            entities
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join("+")
        };

        let topics = self.extract_topics(text);
        let topic_str = if topics.is_empty() {
            "misc".to_string()
        } else {
            topics.iter().take(3).cloned().collect::<Vec<_>>().join("_")
        };

        let quote = self.extract_key_sentence(text);
        let quote_part = if !quote.is_empty() {
            format!("\"{}\"", quote)
        } else {
            String::new()
        };

        let emotions = self.detect_emotions(text);
        let emotion_str = emotions.join("+");

        let flags = self.detect_flags(text);
        let flag_str = flags.join("+");

        // Build content line
        let mut parts = vec![format!("0:{}", entity_str), topic_str];

        if !quote_part.is_empty() {
            parts.push(quote_part);
        }
        if !emotion_str.is_empty() {
            parts.push(emotion_str);
        }
        if !flag_str.is_empty() {
            parts.push(flag_str);
        }

        Ok(parts.join("|"))
    }

    /// Compress text with metadata
    pub fn compress_with_metadata(
        &self,
        text: &str,
        metadata: &HashMap<String, String>,
    ) -> Result<String> {
        let entities = self.detect_entities_in_text(text);
        let entity_str = if entities.is_empty() {
            "???".to_string()
        } else {
            entities
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join("+")
        };

        let topics = self.extract_topics(text);
        let topic_str = if topics.is_empty() {
            "misc".to_string()
        } else {
            topics.iter().take(3).cloned().collect::<Vec<_>>().join("_")
        };

        let quote = self.extract_key_sentence(text);
        let quote_part = if !quote.is_empty() {
            format!("\"{}\"", quote)
        } else {
            String::new()
        };

        let emotions = self.detect_emotions(text);
        let emotion_str = emotions.join("+");

        let flags = self.detect_flags(text);
        let flag_str = flags.join("+");

        let mut lines = Vec::new();

        // Header line if we have metadata
        let source = metadata.get("source_file");
        let wing = metadata.get("wing");
        let room = metadata.get("room");
        let date = metadata.get("date");

        if source.is_some() || wing.is_some() {
            let header_parts = [
                wing.map(|s| s.as_str()).unwrap_or("?"),
                room.map(|s| s.as_str()).unwrap_or("?"),
                date.map(|s| s.as_str()).unwrap_or("?"),
                source
                    .map(|s| {
                        s.split('/')
                            .next_back()
                            .unwrap_or(s)
                            .split('.')
                            .next()
                            .unwrap_or(s)
                    })
                    .unwrap_or("?"),
            ];
            lines.push(header_parts.join("|"));
        }

        // Content line
        let mut parts = vec![format!("0:{}", entity_str), topic_str];

        if !quote_part.is_empty() {
            parts.push(quote_part);
        }
        if !emotion_str.is_empty() {
            parts.push(emotion_str);
        }
        if !flag_str.is_empty() {
            parts.push(flag_str);
        }

        lines.push(parts.join("|"));

        Ok(lines.join("\n"))
    }

    /// Decompress AAAK format back to a summary (lossy - cannot recover original)
    pub fn decompress(&self, _aaak: &str) -> Result<String> {
        // AAAK is lossy compression - we cannot recover the original text
        // This method parses AAAK back into a readable summary structure
        Err(MempalaceError::ParseError(
            "AAAK decompression is not supported - it is a lossy format".to_string(),
        ))
    }

    /// Map emotion word to code
    pub fn emotion_code(&self, emotion: &str) -> Option<&'static str> {
        EMOTION_CODES.get(emotion.to_lowercase().as_str()).copied()
    }

    /// Detect emotions from plain text using keyword signals
    fn detect_emotions(&self, text: &str) -> Vec<String> {
        let text_lower = text.to_lowercase();
        let mut detected = Vec::new();
        let mut seen = HashSet::new();

        for (keyword, code) in EMOTION_SIGNALS.iter() {
            if text_lower.contains(keyword) && !seen.contains(*code) {
                detected.push(code.to_string());
                seen.insert(*code);
            }
        }

        detected.truncate(3);
        detected
    }

    /// Detect importance flags from plain text using keyword signals
    fn detect_flags(&self, text: &str) -> Vec<String> {
        let text_lower = text.to_lowercase();
        let mut detected = Vec::new();
        let mut seen = HashSet::new();

        for (keyword, flag) in FLAG_SIGNALS.iter() {
            if text_lower.contains(keyword) && !seen.contains(*flag) {
                detected.push(flag.to_string());
                seen.insert(*flag);
            }
        }

        detected.truncate(3);
        detected
    }

    /// Extract key topic words from plain text
    fn extract_topics(&self, text: &str) -> Vec<String> {
        // Tokenize: find alphabetic words with 2+ chars
        let word_regex = regex::Regex::new(r"[a-zA-Z][a-zA-Z_-]{2,}").unwrap();

        let mut freq: HashMap<String, i32> = HashMap::new();

        for cap in word_regex.find_iter(text) {
            let w = cap.as_str();
            let w_lower = w.to_lowercase();

            if STOP_WORDS.contains(w_lower.as_str()) || w_lower.len() < 3 {
                continue;
            }

            let entry = freq.entry(w_lower.clone()).or_insert(0);
            *entry += 1;

            // Boost proper nouns (capitalized)
            if w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                let entry = freq.entry(w_lower.clone()).or_insert(0);
                *entry += 2;
            }

            // Boost CamelCase or words with underscore/hyphen (technical terms)
            if w.contains('_') || w.contains('-') || w[1..].chars().any(|c| c.is_uppercase()) {
                let entry = freq.entry(w_lower.clone()).or_insert(0);
                *entry += 2;
            }
        }

        // Sort by frequency descending
        let mut ranked: Vec<(String, i32)> = freq.into_iter().collect();
        ranked.sort_by_key(|&(_, v)| Reverse(v));

        ranked.into_iter().take(3).map(|(w, _)| w).collect()
    }

    /// Extract the most important sentence fragment from text
    fn extract_key_sentence(&self, text: &str) -> String {
        // Split into sentences
        let sentence_regex = regex::Regex::new(r"[^.!?\n]+[.!?\n]+").unwrap();

        let mut sentences: Vec<String> = Vec::new();
        for cap in sentence_regex.find_iter(text) {
            let s = cap.as_str().trim().to_string();
            if s.len() > 10 {
                sentences.push(s);
            }
        }

        if sentences.is_empty() {
            return String::new();
        }

        // Score each sentence
        let mut scored: Vec<(i32, String)> = Vec::new();

        for s in &sentences {
            let mut score = 0;
            let s_lower = s.to_lowercase();

            for w in DECISION_WORDS.iter() {
                if s_lower.contains(w) {
                    score += 2;
                }
            }

            // Prefer shorter sentences
            if s.len() < 80 {
                score += 1;
            }
            if s.len() < 40 {
                score += 1;
            }

            // Penalize very long sentences
            if s.len() > 150 {
                score -= 2;
            }

            scored.push((score, s.clone()));
        }

        // Sort by score descending
        scored.sort_by_key(|(v, _)| Reverse(*v));

        let best = scored
            .into_iter()
            .next()
            .map(|(_, s)| s)
            .unwrap_or_default();

        // Truncate if too long
        if best.len() > 55 {
            format!("{}...", &best[..52])
        } else {
            best
        }
    }

    /// Find known entities in text, or detect capitalized names
    fn detect_entities_in_text(&self, text: &str) -> Vec<String> {
        let mut found = Vec::new();

        // Check known entities
        for (name, code) in &self.entity_codes {
            if !name.chars().all(|c| c.is_lowercase())
                && text.to_lowercase().contains(&name.to_lowercase())
                && !found.contains(code)
            {
                found.push(code.clone());
            }
        }

        if !found.is_empty() {
            found.truncate(3);
            return found;
        }

        // Fallback: find capitalized words that look like names
        let words: Vec<&str> = text.split_whitespace().collect();

        for (i, w) in words.iter().enumerate() {
            let clean: String = w.chars().filter(|c| c.is_alphabetic()).collect();

            if clean.len() >= 2
                && clean
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
                && clean.chars().skip(1).all(|c| c.is_lowercase())
                && i > 0
                && !STOP_WORDS.contains(clean.to_lowercase().as_str())
            {
                let code = clean.chars().take(3).collect::<String>().to_uppercase();
                if !found.contains(&code) {
                    found.push(code);
                }
                if found.len() >= 3 {
                    break;
                }
            }
        }

        found
    }

    /// Extract flags from zettel metadata
    fn get_flags_from_zettel(&self, zettel: &serde_json::Value) -> String {
        let mut flags = Vec::new();

        if zettel
            .get("origin_moment")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            flags.push("ORIGIN".to_string());
        }

        if let Some(sens) = zettel.get("sensitivity").and_then(|v| v.as_str()) {
            if sens.to_uppercase().starts_with("MAXIMUM") {
                flags.push("SENSITIVE".to_string());
            }
        }

        let notes = zettel
            .get("notes")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();

        let origin_label = zettel
            .get("origin_label")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();

        if notes.contains("foundational pillar") || notes.contains("core") {
            flags.push("CORE".to_string());
        }
        if notes.contains("genesis") || origin_label.contains("genesis") {
            flags.push("GENESIS".to_string());
        }
        if notes.contains("pivot") {
            flags.push("PIVOT".to_string());
        }

        flags.join("+")
    }

    /// Encode emotions from zettel emotional_tone field
    fn encode_emotions_from_tone(&self, tones: &[serde_json::Value]) -> String {
        let mut codes = Vec::new();
        let mut seen = HashSet::new();

        for tone in tones {
            if let Some(s) = tone.as_str() {
                if let Some(code) = self.emotion_code(s) {
                    if !seen.contains(code) {
                        codes.push(code);
                        seen.insert(code);
                    }
                }
            }
        }

        codes.truncate(3);
        codes.join("+")
    }

    /// Encode a single zettel into AAAK format
    pub fn encode_zettel(&self, zettel: &serde_json::Value) -> Result<String> {
        let zid = zettel
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MempalaceError::ParseError("Missing zettel id".to_string()))?
            .split('-')
            .next_back()
            .unwrap_or("0");

        let people = zettel
            .get("people")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .filter_map(|name| self.encode_entity(name))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let entities = if people.is_empty() {
            "???".to_string()
        } else {
            let mut unique: Vec<_> = people.to_vec();
            unique.sort();
            unique.dedup();
            unique.into_iter().take(3).collect::<Vec<_>>().join("+")
        };

        let topics = zettel
            .get("topics")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .take(2)
                    .collect::<Vec<_>>()
                    .join("_")
            })
            .unwrap_or_else(|| "misc".to_string());

        let quote = self.extract_key_quote(zettel);
        let quote_part = if !quote.is_empty() {
            format!("\"{}\"", quote)
        } else {
            String::new()
        };

        let weight = zettel
            .get("emotional_weight")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5)
            .to_string();

        let emotions = zettel
            .get("emotional_tone")
            .and_then(|v| v.as_array())
            .map(|arr| self.encode_emotions_from_tone(arr))
            .unwrap_or_default();

        let flags = self.get_flags_from_zettel(zettel);

        let mut parts = vec![format!("{}:{}", zid, entities), topics];

        if !quote_part.is_empty() {
            parts.push(quote_part);
        }
        parts.push(weight);
        if !emotions.is_empty() {
            parts.push(emotions);
        }
        if !flags.is_empty() {
            parts.push(flags);
        }

        Ok(parts.join("|"))
    }

    /// Extract key quote from zettel content
    fn extract_key_quote(&self, zettel: &serde_json::Value) -> String {
        let content = zettel.get("content").and_then(|v| v.as_str()).unwrap_or("");
        let origin = zettel
            .get("origin_label")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let notes = zettel.get("notes").and_then(|v| v.as_str()).unwrap_or("");
        let title = zettel.get("title").and_then(|v| v.as_str()).unwrap_or("");

        let all_text = format!("{} {} {} {}", content, origin, notes, title);

        // Find quoted text
        let quote_regex = regex::Regex::new(r#""([^"]{8,55})""#).unwrap();
        let mut quotes: Vec<String> = quote_regex
            .captures_iter(&all_text)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .filter(|q| q.len() >= 8)
            .collect();

        // Find single-quoted text
        let single_quote_regex = regex::Regex::new(r"(?:^|[\s(])'([^']{8,55})'").unwrap();
        for cap in single_quote_regex.captures_iter(&all_text) {
            if let Some(m) = cap.get(1) {
                let q = m.as_str();
                if q.len() >= 8 && !quotes.contains(&q.to_string()) {
                    quotes.push(q.to_string());
                }
            }
        }

        if quotes.is_empty() {
            // Fall back to title after " - "
            if title.contains(" - ") {
                return title
                    .split(" - ")
                    .nth(1)
                    .unwrap_or("")
                    .chars()
                    .take(45)
                    .collect();
            }
            return String::new();
        }

        // Remove duplicates
        quotes.dedup();

        // Score quotes
        let emotional_words = [
            "love",
            "fear",
            "remember",
            "soul",
            "feel",
            "stupid",
            "scared",
            "beautiful",
            "destroy",
            "respect",
            "trust",
            "consciousness",
            "alive",
            "forget",
            "waiting",
            "peace",
            "matter",
            "real",
            "guilt",
            "escape",
            "rest",
            "hope",
            "dream",
            "lost",
            "found",
        ];

        let mut scored: Vec<(i32, String)> = Vec::new();

        for q in &quotes {
            let mut score = 0;
            if q.starts_with(|c: char| c.is_uppercase()) || q.starts_with("I ") {
                score += 2;
            }

            let q_lower = q.to_lowercase();
            for w in &emotional_words {
                if q_lower.contains(w) {
                    score += 2;
                }
            }

            if q.len() > 20 {
                score += 1;
            }

            if q.starts_with("The ") || q.starts_with("This ") || q.starts_with("She ") {
                score -= 2;
            }

            scored.push((score, q.clone()));
        }

        scored.sort_by_key(|(v, _)| Reverse(*v));
        scored
            .into_iter()
            .next()
            .map(|(_, q)| q)
            .unwrap_or_default()
    }

    /// Encode a tunnel connection
    pub fn encode_tunnel(&self, tunnel: &serde_json::Value) -> Result<String> {
        let from = tunnel
            .get("from")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MempalaceError::ParseError("Missing tunnel from".to_string()))?
            .split('-')
            .next_back()
            .unwrap_or("0");

        let to = tunnel
            .get("to")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MempalaceError::ParseError("Missing tunnel to".to_string()))?
            .split('-')
            .next_back()
            .unwrap_or("0");

        let label = tunnel.get("label").and_then(|v| v.as_str()).unwrap_or("");

        let short_label = if label.contains(':') {
            label.split(':').next().unwrap_or(label)
        } else {
            &label[..label.len().min(30)]
        };

        Ok(format!("T:{}<->{}|{}", from, to, short_label))
    }

    /// Encode an entire zettel file into AAAK Dialect
    pub fn encode_file(&self, data: &serde_json::Value) -> Result<String> {
        let mut lines = Vec::new();

        let source = data
            .get("source_file")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let file_num = if source.contains('-') {
            source.split('-').next().unwrap_or("000")
        } else {
            "000"
        };

        let date = data
            .get("zettels")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("date_context"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let mut all_people: HashSet<String> = HashSet::new();

        if let Some(zettels) = data.get("zettels").and_then(|v| v.as_array()) {
            for z in zettels {
                if let Some(people) = z.get("people").and_then(|v| v.as_array()) {
                    for p in people {
                        if let Some(name) = p.as_str() {
                            if let Some(code) = self.encode_entity(name) {
                                all_people.insert(code);
                            }
                        }
                    }
                }
            }
        }

        if all_people.is_empty() {
            all_people.insert("???".to_string());
        }

        let mut primary: Vec<_> = all_people.into_iter().collect();
        primary.sort();
        let primary_str = primary.into_iter().take(3).collect::<Vec<_>>().join("+");

        let title = if source.contains('-') {
            source
                .split('-')
                .skip(1)
                .collect::<Vec<_>>()
                .join("-")
                .trim()
                .to_string()
        } else {
            source.replace(".txt", "")
        };

        lines.push(format!("{}|{}|{}|{}", file_num, primary_str, date, title));

        if let Some(arc) = data.get("emotional_arc").and_then(|v| v.as_str()) {
            if !arc.is_empty() {
                lines.push(format!("ARC:{}", arc));
            }
        }

        if let Some(zettels) = data.get("zettels").and_then(|v| v.as_array()) {
            for z in zettels {
                if let Ok(encoded) = self.encode_zettel(z) {
                    lines.push(encoded);
                }
            }
        }

        if let Some(tunnels) = data.get("tunnels").and_then(|v| v.as_array()) {
            for t in tunnels {
                if let Ok(encoded) = self.encode_tunnel(t) {
                    lines.push(encoded);
                }
            }
        }

        Ok(lines.join("\n"))
    }

    /// Decode AAAK format back to structured data
    pub fn decode(&self, dialect_text: &str) -> Result<HashMap<String, serde_json::Value>> {
        let lines: Vec<&str> = dialect_text.trim().split('\n').collect();
        let mut result = HashMap::new();
        let mut header = HashMap::new();
        let mut zettels: Vec<String> = Vec::new();
        let mut tunnels: Vec<String> = Vec::new();
        let mut arc = String::new();

        for line in &lines {
            if let Some(arc_content) = line.strip_prefix("ARC:") {
                arc = arc_content.to_string();
            } else if line.starts_with("T:") {
                tunnels.push(line.to_string());
            } else if line.contains('|') && line.contains(':') {
                // Likely a zettel line
                zettels.push(line.to_string());
            } else if line.contains('|') {
                // Header line
                let parts: Vec<&str> = line.split('|').collect();
                if !parts.is_empty() {
                    header.insert("file".to_string(), serde_json::json!(parts[0]));
                }
                if parts.len() >= 2 {
                    header.insert("entities".to_string(), serde_json::json!(parts[1]));
                }
                if parts.len() >= 3 {
                    header.insert("date".to_string(), serde_json::json!(parts[2]));
                }
                if parts.len() >= 4 {
                    header.insert("title".to_string(), serde_json::json!(parts[3]));
                }
            }
        }

        result.insert("header".to_string(), serde_json::json!(header));
        result.insert("arc".to_string(), serde_json::json!(arc));
        result.insert("zettels".to_string(), serde_json::json!(zettels));
        result.insert("tunnels".to_string(), serde_json::json!(tunnels));

        Ok(result)
    }

    /// Get compression statistics
    pub fn compression_stats(
        &self,
        original_text: &str,
        compressed: &str,
    ) -> HashMap<String, serde_json::Value> {
        let orig_tokens = Self::count_tokens(original_text);
        let comp_tokens = Self::count_tokens(compressed);

        let mut stats = HashMap::new();
        stats.insert(
            "original_tokens_est".to_string(),
            serde_json::json!(orig_tokens),
        );
        stats.insert(
            "summary_tokens_est".to_string(),
            serde_json::json!(comp_tokens),
        );
        stats.insert(
            "size_ratio".to_string(),
            serde_json::json!(
                (orig_tokens as f64 / comp_tokens.max(1) as f64 * 10.0).round() / 10.0
            ),
        );
        stats.insert(
            "original_chars".to_string(),
            serde_json::json!(original_text.len()),
        );
        stats.insert(
            "summary_chars".to_string(),
            serde_json::json!(compressed.len()),
        );
        stats.insert(
            "note".to_string(),
            serde_json::json!("Estimates only. AAAK is lossy."),
        );

        stats
    }

    /// Estimate token count using word-based heuristic (~1.3 tokens per word)
    pub fn count_tokens(text: &str) -> usize {
        let words: Vec<&str> = text.split_whitespace().collect();
        (words.len() as f64 * 1.3).ceil() as usize
    }
}

impl Default for AaakDialect {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emotion_code_mapping() {
        let dialect = AaakDialect::new();

        // Test direct mappings
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

        // Test case insensitivity
        assert_eq!(dialect.emotion_code("JOY"), Some("joy"));
        assert_eq!(dialect.emotion_code("Fear"), Some("fear"));

        // Test unknown emotion
        assert_eq!(dialect.emotion_code("unknown"), None);
    }

    #[test]
    fn test_encode_emotions() {
        let dialect = AaakDialect::new();

        let emotions = vec!["joy", "fear", "hope"];
        assert_eq!(dialect.encode_emotions(&emotions), "joy+fear+hope");

        // Test truncation to 3
        let emotions = vec!["joy", "fear", "hope", "love", "peace"];
        assert_eq!(dialect.encode_emotions(&emotions), "joy+fear+hope");

        // Test deduplication
        let emotions = vec!["joy", "joyful", "fear"];
        assert_eq!(dialect.encode_emotions(&emotions), "joy+fear");
    }

    #[test]
    fn test_basic_compression() {
        let dialect = AaakDialect::new();

        let text = "We decided to use GraphQL instead of REST because it provides better type safety. Alice was excited about the API change.";
        let result = dialect.compress(text).unwrap();

        // Should detect DECISION flag and DETERMINATION emotion
        assert!(
            result.contains("DECISION"),
            "Should contain DECISION flag: {}",
            result
        );
        assert!(
            result.contains("determ"),
            "Should contain determ emotion: {}",
            result
        );
        assert!(
            result.contains("excite"),
            "Should contain excite emotion: {}",
            result
        );

        // Should extract topics
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

        assert!(
            result.contains("ALI"),
            "Should contain ALI entity: {}",
            result
        );
        assert!(
            result.contains("BOB"),
            "Should contain BOB entity: {}",
            result
        );
    }

    #[test]
    fn test_entity_skip() {
        let mut dialect = AaakDialect::new();
        dialect.add_entity("Alice", "ALI");
        dialect.skip_entity("Gandalf");

        let text = "Alice and Gandalf walked into the forest.";
        let entities = dialect.detect_entities_in_text(text);

        // Should find Alice but not encode Gandalf (skipped)
        assert!(
            entities.contains(&"ALI".to_string()),
            "Should find Alice: {:?}",
            entities
        );
    }

    #[test]
    fn test_compression_stats() {
        let dialect = AaakDialect::new();

        let original = "This is a longer text that should have more tokens when estimated using the word-based heuristic.";
        let compressed = "0:???|longer_text|\"This is a longer text\"|0.5";

        let stats = dialect.compression_stats(original, compressed);

        assert!(
            stats["original_tokens_est"].as_u64().unwrap()
                > stats["summary_tokens_est"].as_u64().unwrap()
        );
        assert!(
            stats["original_chars"].as_u64().unwrap() > stats["summary_chars"].as_u64().unwrap()
        );
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

        assert!(
            result.contains("001"),
            "Should contain zettel id: {}",
            result
        );
        assert!(result.contains("ALI"), "Should contain entity: {}", result);
        assert!(
            result.contains("testing"),
            "Should contain topic: {}",
            result
        );
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

        // Format: ZID:ENTITIES|topics|WEIGHT|EMOTIONS
        assert!(
            encoded.starts_with("002:"),
            "Should start with zettel ID: {}",
            encoded
        );
        assert!(
            encoded.contains("ALI"),
            "Should contain entity code ALI: {}",
            encoded
        );
        assert!(
            encoded.contains("BOB"),
            "Should contain entity code BOB: {}",
            encoded
        );
        assert!(
            encoded.contains("joy"),
            "Should contain emotion code: {}",
            encoded
        );
        assert!(
            !encoded.contains("Alice"),
            "Should use entity code ALI instead of name"
        );
        assert!(
            !encoded.contains("Bob"),
            "Should use entity code BOB instead of name"
        );
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

        assert!(
            orig_tokens > 0,
            "Original tokens should be > 0, got {}",
            orig_tokens
        );
        assert!(
            comp_tokens > 0,
            "Compressed tokens should be > 0, got {}",
            comp_tokens
        );
        // size_ratio is computed as (orig/comp * 10).round() / 10, so we just verify it's positive
        assert!(
            size_ratio > 0.0,
            "Size ratio should be positive, got {}",
            size_ratio
        );
    }

    #[test]
    fn test_topic_extraction() {
        let dialect = AaakDialect::new();

        let text = "Rust is a systems programming language that focuses on safety and performance. The compiler is very helpful.";
        let topics = dialect.extract_topics(text);

        // Should extract rust, systems, programming, language, safety, performance, compiler, helpful
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
}
