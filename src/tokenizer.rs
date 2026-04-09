//! Token accounting utilities.
//!
//! Token counts are model-dependent, so all measured counts must declare which
//! tokenizer produced them. For now, `measured` means counted by a deterministic
//! tokenizer implementation. It does not imply model-accurate LLM token counts.
//! When a model-specific tokenizer is unavailable, callers should use estimated
//! counts and label them clearly.

use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Supported tokenizer families for token accounting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenizerKind {
    OpenAi,
    Claude,
    Local,
}

impl TokenizerKind {
    /// Stable identifier for CLI and JSON output.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Claude => "claude",
            Self::Local => "local",
        }
    }
}

/// Whether a token count is measured by a tokenizer or estimated heuristically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenCountStatus {
    Measured,
    Estimated,
}

impl TokenCountStatus {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Measured => "measured",
            Self::Estimated => "estimated",
        }
    }
}

/// Structured token count with provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenCount {
    pub tokens: usize,
    pub kind: TokenizerKind,
    pub status: TokenCountStatus,
}

impl TokenCount {
    #[must_use]
    pub fn measured(tokens: usize, kind: TokenizerKind) -> Self {
        Self {
            tokens,
            kind,
            status: TokenCountStatus::Measured,
        }
    }

    #[must_use]
    pub fn estimated(tokens: usize, kind: TokenizerKind) -> Self {
        Self {
            tokens,
            kind,
            status: TokenCountStatus::Estimated,
        }
    }
}

/// Tokenizer interface for explicit accounting.
pub trait Tokenizer {
    fn kind(&self) -> TokenizerKind;

    fn count(&self, text: &str) -> TokenCount;

    fn count_batch(&self, texts: &[&str]) -> Vec<TokenCount> {
        texts.iter().map(|text| self.count(text)).collect()
    }
}

/// Lightweight local tokenizer.
///
/// This is intentionally simple for phase 1: it provides deterministic,
/// tokenizer-backed counts that are clearly labeled as measured for the local
/// tokenizer family, without pretending to match vendor-specific models.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalTokenizer;

impl LocalTokenizer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Tokenizer for LocalTokenizer {
    fn kind(&self) -> TokenizerKind {
        TokenizerKind::Local
    }

    fn count(&self, text: &str) -> TokenCount {
        TokenCount::measured(count_local_tokens(text), self.kind())
    }
}

#[must_use]
pub fn normalize_text(text: &str) -> Cow<'_, str> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Cow::Borrowed("");
    }

    if trimmed
        .split_whitespace()
        .all(|segment| !segment.is_empty())
        && !trimmed.contains(['\n', '\t', '\r'])
    {
        return Cow::Borrowed(trimmed);
    }

    Cow::Owned(trimmed.split_whitespace().collect::<Vec<_>>().join(" "))
}

#[must_use]
pub fn estimate_openai_tokens(text: &str) -> TokenCount {
    TokenCount::estimated(estimate_word_tokens(text), TokenizerKind::OpenAi)
}

#[must_use]
pub fn estimate_claude_tokens(text: &str) -> TokenCount {
    TokenCount::estimated(estimate_word_tokens(text), TokenizerKind::Claude)
}

#[must_use]
pub fn estimate_word_tokens(text: &str) -> usize {
    let words = text.split_whitespace().count();
    ((words as f64) * 1.3).ceil() as usize
}

#[must_use]
pub fn count_local_tokens(text: &str) -> usize {
    normalize_text(text).split_whitespace().count()
}
