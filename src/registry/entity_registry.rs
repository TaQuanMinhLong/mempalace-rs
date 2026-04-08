//! Entity registry - port from Python entity_registry.py (~640 lines)
//!
//! Knows the difference between Riley (a person) and ever (an adverb).
//! Built from three sources:
//!   1. Onboarding - what the user explicitly told us
//!   2. Learned - what we inferred from session history with high confidence
//!   3. Researched - what we looked up via Wikipedia for unknown words

use crate::error::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Common English words that could be confused with names.
/// These get flagged as AMBIGUOUS and require context disambiguation.
const COMMON_ENGLISH_WORDS: &[&str] = &[
    // Words that are also common personal names
    "ever",
    "grace",
    "will",
    "bill",
    "mark",
    "april",
    "may",
    "june",
    "joy",
    "hope",
    "faith",
    "chance",
    "chase",
    "hunter",
    "dash",
    "flash",
    "star",
    "sky",
    "river",
    "brook",
    "lane",
    "art",
    "clay",
    "gil",
    "nat",
    "max",
    "rex",
    "ray",
    "jay",
    "rose",
    "violet",
    "lily",
    "ivy",
    "ash",
    "reed",
    "sage",
    // Days and months
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
    "january",
    "february",
    "march",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

/// Wikipedia summary phrases indicating a personal name
const NAME_INDICATOR_PHRASES: &[&str] = &[
    "given name",
    "personal name",
    "first name",
    "forename",
    "masculine name",
    "feminine name",
    "boy's name",
    "girl's name",
    "male name",
    "female name",
    "irish name",
    "welsh name",
    "scottish name",
    "gaelic name",
    "hebrew name",
    "arabic name",
    "norse name",
    "old english name",
    "is a name",
    "as a name",
    "name meaning",
    "name derived from",
    "legendary irish",
    "legendary welsh",
    "legendary scottish",
];

/// Wikipedia summary phrases indicating a place
const PLACE_INDICATOR_PHRASES: &[&str] = &[
    "city in",
    "town in",
    "village in",
    "municipality",
    "capital of",
    "district of",
    "county",
    "province",
    "region of",
    "island of",
    "mountain in",
    "river in",
];

// ─────────────────────────────────────────────────────────────────────────────
// Person info stored in registry
// ─────────────────────────────────────────────────────────────────────────────

/// Person info stored in the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonInfo {
    pub source: String,
    pub contexts: Vec<String>,
    pub aliases: Vec<String>,
    pub relationship: String,
    pub confidence: f64,
}

impl PersonInfo {
    pub fn new(source: &str, contexts: Vec<String>, relationship: &str, confidence: f64) -> Self {
        Self {
            source: source.to_string(),
            contexts,
            aliases: Vec::new(),
            relationship: relationship.to_string(),
            confidence,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Wiki lookup result
// ─────────────────────────────────────────────────────────────────────────────

/// Wiki lookup result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiResult {
    pub inferred_type: String,
    pub confidence: f64,
    pub wiki_summary: Option<String>,
    pub wiki_title: Option<String>,
    pub confirmed: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Lookup result
// ─────────────────────────────────────────────────────────────────────────────

/// Lookup result returned by registry.lookup()
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupResult {
    pub entity_type: String,
    pub confidence: f64,
    pub source: String,
    pub name: String,
    pub needs_disambiguation: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Registry data structure
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryData {
    version: u32,
    mode: String,
    people: HashMap<String, PersonInfo>,
    projects: Vec<String>,
    ambiguous_flags: Vec<String>,
    #[serde(default)]
    wiki_cache: HashMap<String, WikiResult>,
}

impl Default for RegistryData {
    fn default() -> Self {
        Self {
            version: 1,
            mode: "personal".to_string(),
            people: HashMap::new(),
            projects: Vec::new(),
            ambiguous_flags: Vec::new(),
            wiki_cache: HashMap::new(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Entity Registry
// ─────────────────────────────────────────────────────────────────────────────

/// Entity registry
#[derive(Debug, Clone)]
pub struct EntityRegistry {
    data: RegistryData,
    path: PathBuf,
    /// Compiled regex patterns for person context detection
    person_patterns: Vec<Regex>,
    /// Compiled regex patterns for concept context detection (disambiguation)
    concept_patterns: Vec<Regex>,
}

impl EntityRegistry {
    // ── Construction ──────────────────────────────────────────────────────

    /// Load registry from file
    pub fn load(config_dir: Option<&Path>) -> Result<Self> {
        let path = config_dir
            .map(|p| p.join("entity_registry.json"))
            .unwrap_or_else(|| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                PathBuf::from(format!("{}/.mempalace/entity_registry.json", home))
            });

        let data = if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => RegistryData::default(),
            }
        } else {
            RegistryData::default()
        };

        Ok(Self {
            person_patterns: Self::build_person_patterns(),
            concept_patterns: Self::build_concept_patterns(),
            data,
            path,
        })
    }

    /// Save registry to file
    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&self.data)?;
        fs::write(&self.path, content)?;
        Ok(())
    }

    // ── Pattern builders ───────────────────────────────────────────────────

    fn build_person_patterns() -> Vec<Regex> {
        let patterns = [
            r"\b{name}\s+said\b",
            r"\b{name}\s+told\b",
            r"\b{name}\s+asked\b",
            r"\b{name}\s+laughed\b",
            r"\b{name}\s+smiled\b",
            r"\b{name}\s+was\b",
            r"\b{name}\s+is\b",
            r"\b{name}\s+called\b",
            r"\b{name}\s+texted\b",
            r"\bwith\s+{name}\b",
            r"\bsaw\s+{name}\b",
            r"\bcalled\s+{name}\b",
            r"\btook\s+{name}\b",
            r"\bpicked\s+up\s+{name}\b",
            r"\bdrop(?:ped)?\s+(?:off\s+)?{name}\b",
            r"\b{name}(?:'s|s')\b",
            r"\bhey\s+{name}\b",
            r"\bthanks?\s+{name}\b",
            r"^{name}[:\s]",
        ];
        patterns
            .iter()
            .filter_map(|p| Regex::new(&p.replace("{name}", r"[A-Z][a-z]{2,15}")).ok())
            .collect()
    }

    fn build_concept_patterns() -> Vec<Regex> {
        let patterns = [
            r"\bhave\s+you\s+{name}\b",
            r"\bif\s+you\s+{name}\b",
            r"\b{name}\s+since\b",
            r"\b{name}\s+again\b",
            r"\bnot\s+{name}\b",
            r"\b{name}\s+more\b",
            r"\bwould\s+{name}\b",
            r"\bcould\s+{name}\b",
            r"\bwill\s+{name}\b",
            r"(?:the\s+)?{name}\s+(?:of|in|at|for|to)\b",
        ];
        patterns
            .iter()
            .filter_map(|p| Regex::new(&p.replace("{name}", r"[A-Z][a-z]{2,15}")).ok())
            .collect()
    }

    // ── Seed from onboarding ────────────────────────────────────────────────

    /// Seed the registry from onboarding data
    pub fn seed(
        &mut self,
        mode: &str,
        people: &[(String, String, String)],
        // (name, relationship, context)
        projects: &[String],
        aliases: Option<&HashMap<String, String>>,
    ) {
        self.data.mode = mode.to_string();
        self.data.projects = projects.to_vec();

        let aliases = aliases.cloned().unwrap_or_default();
        let reverse_aliases: HashMap<String, String> = aliases
            .iter()
            .map(|(k, v)| (v.clone(), k.clone()))
            .collect();

        for (name, relationship, context) in people {
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            let ctx = if context.is_empty() {
                "personal"
            } else {
                context
            };

            self.data.people.insert(
                name.to_string(),
                PersonInfo::new("onboarding", vec![ctx.to_string()], relationship, 1.0),
            );

            // Also register alias
            if let Some(alias) = reverse_aliases.get(name) {
                self.data.people.insert(
                    alias.clone(),
                    PersonInfo::new("onboarding", vec![ctx.to_string()], relationship, 1.0),
                );
            }
        }

        // Flag ambiguous names
        self.data.ambiguous_flags = self
            .data
            .people
            .keys()
            .filter(|name| COMMON_ENGLISH_WORDS.contains(&name.to_lowercase().as_str()))
            .map(|name| name.to_lowercase())
            .collect();

        // Sort for deterministic output
        self.data.ambiguous_flags.sort();
    }

    // ── Lookup ─────────────────────────────────────────────────────────────

    /// Look up a word and return its entity classification.
    /// Returns LookupResult with entity_type, confidence, source, name, needs_disambiguation.
    pub fn lookup(&self, word: &str, context: &str) -> LookupResult {
        let word_lower = word.to_lowercase();

        // 1. Exact match in people registry
        for (canonical, info) in &self.data.people {
            if word_lower == canonical.to_lowercase()
                || info.aliases.iter().any(|a| word_lower == a.to_lowercase())
            {
                // Check if ambiguous and needs disambiguation
                if self.data.ambiguous_flags.contains(&word_lower) && !context.is_empty() {
                    if let Some(resolved) = self.disambiguate(word, context, info) {
                        return resolved;
                    }
                }
                return LookupResult {
                    entity_type: "person".to_string(),
                    confidence: info.confidence,
                    source: info.source.clone(),
                    name: canonical.clone(),
                    needs_disambiguation: false,
                };
            }
        }

        // 2. Project match
        for proj in &self.data.projects {
            if word_lower == proj.to_lowercase() {
                return LookupResult {
                    entity_type: "project".to_string(),
                    confidence: 1.0,
                    source: "onboarding".to_string(),
                    name: proj.clone(),
                    needs_disambiguation: false,
                };
            }
        }

        // 3. Wiki cache
        for (cached_word, cached) in &self.data.wiki_cache {
            if word_lower == cached_word.to_lowercase() && cached.confirmed {
                return LookupResult {
                    entity_type: cached.inferred_type.clone(),
                    confidence: cached.confidence,
                    source: "wiki".to_string(),
                    name: word.to_string(),
                    needs_disambiguation: false,
                };
            }
        }

        LookupResult {
            entity_type: "unknown".to_string(),
            confidence: 0.0,
            source: "none".to_string(),
            name: word.to_string(),
            needs_disambiguation: false,
        }
    }

    /// Disambiguate a word that is both a name and a common word.
    fn disambiguate(
        &self,
        word: &str,
        context: &str,
        person_info: &PersonInfo,
    ) -> Option<LookupResult> {
        let ctx_lower = context.to_lowercase();

        let mut person_score = 0;
        for pat in &self.person_patterns {
            if pat.is_match(&ctx_lower) {
                person_score += 1;
            }
        }

        let mut concept_score = 0;
        for pat in &self.concept_patterns {
            if pat.is_match(&ctx_lower) {
                concept_score += 1;
            }
        }

        if person_score > concept_score {
            let confidence = (0.7 + person_score as f64 * 0.1).min(0.95);
            return Some(LookupResult {
                entity_type: "person".to_string(),
                confidence,
                source: person_info.source.clone(),
                name: word.to_string(),
                needs_disambiguation: false,
            });
        }
        if concept_score > person_score {
            let confidence = (0.7 + concept_score as f64 * 0.1).min(0.90);
            return Some(LookupResult {
                entity_type: "concept".to_string(),
                confidence,
                source: "context_disambiguated".to_string(),
                name: word.to_string(),
                needs_disambiguation: false,
            });
        }

        // Truly ambiguous — fall through to registered name
        None
    }

    // ── Wikipedia research ─────────────────────────────────────────────────

    /// Research an unknown word via Wikipedia REST API.
    /// Caches result. If auto_confirm=false, marks as unconfirmed (needs user review).
    pub async fn research(&mut self, word: &str) -> Result<WikiResult> {
        // Already cached?
        if let Some(cached) = self.data.wiki_cache.get(word) {
            return Ok(cached.clone());
        }

        let result = self.wikipedia_lookup(word).await;

        // Store in cache (even on failure, to avoid repeated lookups)
        let to_store = WikiResult {
            inferred_type: result.inferred_type.clone(),
            confidence: result.confidence,
            wiki_summary: result.wiki_summary.clone(),
            wiki_title: result.wiki_title.clone(),
            confirmed: false,
        };
        self.data
            .wiki_cache
            .insert(word.to_string(), to_store.clone());
        let _ = self.save();

        Ok(to_store)
    }

    /// Perform Wikipedia lookup via REST API
    async fn wikipedia_lookup(&self, word: &str) -> WikiResult {
        let url = format!(
            "https://en.wikipedia.org/api/rest_v1/page/summary/{}",
            urlencoding::encode(word)
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        match client
            .get(&url)
            .header("User-Agent", "MemPalace/1.0")
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<serde_json::Value>().await {
                    Ok(data) => {
                        let extract = data
                            .get("extract")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_lowercase();
                        let title = data.get("title").and_then(|v| v.as_str()).unwrap_or(word);
                        let page_type = data.get("type").and_then(|v| v.as_str()).unwrap_or("");

                        // Disambiguation page
                        if page_type == "disambiguation" {
                            let description = data
                                .get("description")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_lowercase();
                            if description.contains("name") || description.contains("given name") {
                                return WikiResult {
                                    inferred_type: "person".to_string(),
                                    confidence: 0.65,
                                    wiki_summary: Some(extract.chars().take(200).collect()),
                                    wiki_title: Some(title.to_string()),
                                    confirmed: false,
                                };
                            }
                            return WikiResult {
                                inferred_type: "ambiguous".to_string(),
                                confidence: 0.4,
                                wiki_summary: Some(extract.chars().take(200).collect()),
                                wiki_title: Some(title.to_string()),
                                confirmed: false,
                            };
                        }

                        // Check for name indicators
                        let word_lower = word.to_lowercase();
                        if NAME_INDICATOR_PHRASES
                            .iter()
                            .any(|phrase| extract.contains(phrase))
                        {
                            let is_definitive = extract.contains(&format!("{} is a", word_lower))
                                || extract.contains(&format!("{} (name", word_lower));
                            let confidence = if is_definitive { 0.90 } else { 0.80 };
                            return WikiResult {
                                inferred_type: "person".to_string(),
                                confidence,
                                wiki_summary: Some(extract.chars().take(200).collect()),
                                wiki_title: Some(title.to_string()),
                                confirmed: false,
                            };
                        }

                        // Check for place indicators
                        if PLACE_INDICATOR_PHRASES
                            .iter()
                            .any(|phrase| extract.contains(phrase))
                        {
                            return WikiResult {
                                inferred_type: "place".to_string(),
                                confidence: 0.80,
                                wiki_summary: Some(extract.chars().take(200).collect()),
                                wiki_title: Some(title.to_string()),
                                confirmed: false,
                            };
                        }

                        // Found but doesn't match name/place
                        WikiResult {
                            inferred_type: "concept".to_string(),
                            confidence: 0.60,
                            wiki_summary: Some(extract.chars().take(200).collect()),
                            wiki_title: Some(title.to_string()),
                            confirmed: false,
                        }
                    }
                    Err(_) => WikiResult {
                        inferred_type: "unknown".to_string(),
                        confidence: 0.0,
                        wiki_summary: None,
                        wiki_title: None,
                        confirmed: false,
                    },
                }
            }
            Ok(resp) if resp.status() == 404 => {
                // Not in Wikipedia — strong signal it's a proper noun (unusual name)
                WikiResult {
                    inferred_type: "person".to_string(),
                    confidence: 0.70,
                    wiki_summary: None,
                    wiki_title: None,
                    confirmed: false,
                }
            }
            _ => WikiResult {
                inferred_type: "unknown".to_string(),
                confidence: 0.0,
                wiki_summary: None,
                wiki_title: None,
                confirmed: false,
            },
        }
    }

    /// Mark a researched word as confirmed and add to people registry
    pub fn confirm_research(
        &mut self,
        word: &str,
        entity_type: &str,
        relationship: &str,
        context: &str,
    ) -> Result<()> {
        if let Some(cached) = self.data.wiki_cache.get_mut(word) {
            cached.confirmed = true;
            cached.inferred_type = entity_type.to_string();
        }

        if entity_type == "person" {
            self.data.people.insert(
                word.to_string(),
                PersonInfo::new("wiki", vec![context.to_string()], relationship, 0.90),
            );
            if COMMON_ENGLISH_WORDS.contains(&word.to_lowercase().as_str())
                && !self.data.ambiguous_flags.contains(&word.to_lowercase())
            {
                self.data.ambiguous_flags.push(word.to_lowercase());
            }
        }

        self.save()
    }

    // ── Learn from text ───────────────────────────────────────────────────

    /// Scan text for new entity candidates and learn high-confidence ones.
    /// Returns list of newly discovered person names.
    pub fn learn_from_text(&mut self, text: &str, min_confidence: f64) -> Result<Vec<String>> {
        let candidates = extract_candidates_from_text(text);
        let mut new_people = Vec::new();

        for (name, _frequency) in candidates {
            // Skip if already known
            if self.data.people.contains_key(&name) {
                continue;
            }
            if self
                .data
                .projects
                .iter()
                .any(|p| p.to_lowercase() == name.to_lowercase())
            {
                continue;
            }

            let scores = score_entity(&name, text);
            let entity_type = classify_entity_type(&scores);
            let confidence = scores.person_signals.len() as f64 * 0.1;

            if entity_type == "person" && confidence >= min_confidence {
                self.data.people.insert(
                    name.clone(),
                    PersonInfo::new("learned", vec![self.data.mode.clone()], "", confidence),
                );
                if COMMON_ENGLISH_WORDS.contains(&name.to_lowercase().as_str())
                    && !self.data.ambiguous_flags.contains(&name.to_lowercase())
                {
                    self.data.ambiguous_flags.push(name.to_lowercase());
                }
                new_people.push(name);
            }
        }

        if !new_people.is_empty() {
            self.save()?;
        }

        Ok(new_people)
    }

    // ── Query helpers ─────────────────────────────────────────────────────

    /// Extract known person names from a query string.
    /// Returns canonical names found.
    pub fn extract_people_from_query(&self, query: &str) -> Vec<String> {
        let mut found = Vec::new();

        for (canonical, info) in &self.data.people {
            let names_to_check: Vec<&str> = std::iter::once(canonical.as_str())
                .chain(info.aliases.iter().map(|s| s.as_str()))
                .collect();

            for name in names_to_check {
                let pattern = format!(r"\b{}\b", regex::escape(name));
                if Regex::new(&pattern)
                    .map(|re| re.is_match(query))
                    .unwrap_or(false)
                {
                    // For ambiguous words, check context
                    if self.data.ambiguous_flags.contains(&name.to_lowercase()) {
                        if let Some(resolved) = self.disambiguate(name, query, info) {
                            if resolved.entity_type == "person" && !found.contains(canonical) {
                                found.push(canonical.clone());
                            }
                        }
                    } else if !found.contains(canonical) {
                        found.push(canonical.clone());
                    }
                }
            }
        }

        found
    }

    /// Find capitalized words in query that aren't in registry or common words.
    /// These are candidates for Wikipedia research.
    pub fn extract_unknown_candidates(&self, query: &str) -> Vec<String> {
        let pattern = Regex::new(r"\b[A-Z][a-z]{2,15}\b").unwrap();
        let mut unknown = Vec::new();

        for cap in pattern.find_iter(query) {
            let word = cap.as_str();
            if COMMON_ENGLISH_WORDS.contains(&word.to_lowercase().as_str()) {
                continue;
            }
            let result = self.lookup(word, query);
            if result.entity_type == "unknown" && !unknown.contains(&word.to_string()) {
                unknown.push(word.to_string());
            }
        }

        unknown
    }

    /// Get a summary of the registry state
    pub fn summary(&self) -> String {
        let people_keys: Vec<_> = self.data.people.keys().take(8).collect();
        let people_str = if self.data.people.len() > 8 {
            format!(
                "{} ({}{})",
                self.data.people.len(),
                people_keys
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                ",..."
            )
        } else {
            format!(
                "{} ({})",
                self.data.people.len(),
                people_keys
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };

        format!(
            "Mode: {}\nPeople: {}\nProjects: {}\nAmbiguous flags: {}\nWiki cache: {} entries",
            self.data.mode,
            people_str,
            if self.data.projects.is_empty() {
                "(none)".to_string()
            } else {
                self.data.projects.join(", ")
            },
            if self.data.ambiguous_flags.is_empty() {
                "(none)".to_string()
            } else {
                self.data.ambiguous_flags.join(", ")
            },
            self.data.wiki_cache.len()
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Entity scoring helpers (ported from entity_detector.py)
// ─────────────────────────────────────────────────────────────────────────────

/// Candidate entity extracted from text
#[derive(Debug, Clone)]
struct EntityCandidate {
    person_score: usize,
    project_score: usize,
    person_signals: Vec<String>,
    project_signals: Vec<String>,
}

/// Extract candidate entities from text (frequency-based).
fn extract_candidates_from_text(text: &str) -> HashMap<String, usize> {
    let pattern = Regex::new(r"\b[A-Z][a-z]{2,15}\b").unwrap();
    let mut freq: HashMap<String, usize> = HashMap::new();

    for cap in pattern.find_iter(text) {
        *freq.entry(cap.as_str().to_string()).or_insert(0) += 1;
    }

    // Filter: only keep names appearing 3+ times
    freq.retain(|_, count| *count >= 3);
    freq
}

/// Score an entity candidate based on context patterns
fn score_entity(name: &str, text: &str) -> EntityCandidate {
    let mut candidate = EntityCandidate {
        person_score: 0,
        project_score: 0,
        person_signals: Vec::new(),
        project_signals: Vec::new(),
    };

    let person_patterns = [
        (r"\bhey\s+{name}\b", "addressed"),
        (r"\bthanks?\s+{name}\b", "addressed"),
        (r"\b{name}\s+was\b", "verb"),
        (r"\b{name}\s+is\b", "verb"),
        (r"\bshe\b", "pronoun"),
        (r"\bher\b", "pronoun"),
        (r"\bhis\b", "pronoun"),
        (r"\bhe\b", "pronoun"),
        (r"\bthey\b", "pronoun"),
        (r">\s*{name}[:\s]", "dialogue"),
        (r"^{name}:", "dialogue"),
    ];

    let project_patterns = [
        (r"\b{name}[-_][a-z]", "hyphenated"),
        (r"\.{ext}\b", "extension"),
    ];

    // Check person patterns
    let ctx_lower = text.to_lowercase();
    for (pat, signal) in &person_patterns {
        let re_pattern = pat.replace("{name}", &format!(r"(?i){}", regex::escape(name)));
        if Regex::new(&re_pattern)
            .map(|re| re.is_match(&ctx_lower))
            .unwrap_or(false)
        {
            candidate.person_score += 1;
            candidate.person_signals.push(signal.to_string());
        }
    }

    // Check for pronoun proximity
    let pronoun_pattern = Regex::new(r"(?i)\b(he|she|they|his|her|him)\b").unwrap();
    let name_pattern = Regex::new(&format!(r"(?i)\b{}\b", regex::escape(name))).unwrap();
    if let Some(name_match) = name_pattern.find(&ctx_lower) {
        for cap in pronoun_pattern.find_iter(&ctx_lower) {
            let dist = (name_match.start() as i64 - cap.start() as i64).abs();
            if dist < 30 {
                candidate.person_score += 1;
                candidate
                    .person_signals
                    .push("pronoun_proximity".to_string());
                break;
            }
        }
    }

    // Check project patterns
    for (pat, signal) in &project_patterns {
        let re_pattern = pat.replace("{name}", &format!(r"(?i){}", regex::escape(name)));
        if Regex::new(&re_pattern)
            .map(|re| re.is_match(text))
            .unwrap_or(false)
        {
            candidate.project_score += 1;
            candidate.project_signals.push(signal.to_string());
        }
    }

    candidate
}

/// Classify entity type based on scores
fn classify_entity_type(scores: &EntityCandidate) -> &'static str {
    let total = scores.person_score + scores.project_score;
    if total == 0 {
        return "uncertain";
    }

    let person_ratio = scores.person_score as f64 / total as f64;
    let has_two_signals = scores.person_signals.len() >= 2;

    if person_ratio >= 0.7 && has_two_signals && scores.person_score >= 3 {
        "person"
    } else if person_ratio <= 0.3 {
        "project"
    } else {
        "uncertain"
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "../tests/registry_entity_registry.rs"]
mod tests;
