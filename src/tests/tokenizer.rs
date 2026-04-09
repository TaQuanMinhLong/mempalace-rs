use crate::tokenizer::{LocalTokenizer, TokenCountStatus, Tokenizer, TokenizerKind};

#[test]
fn test_local_tokenizer_uses_whitespace_count() {
    let tokenizer = LocalTokenizer::new();
    let count = tokenizer.count("alpha beta gamma");

    assert_eq!(count.tokens, 3);
    assert_eq!(count.kind, TokenizerKind::Local);
    assert_eq!(count.status, TokenCountStatus::Measured);
}

#[test]
fn test_local_tokenizer_normalizes_whitespace() {
    let tokenizer = LocalTokenizer::new();
    let count = tokenizer.count("  alpha\n\tbeta   gamma  ");

    assert_eq!(count.tokens, 3);
}
