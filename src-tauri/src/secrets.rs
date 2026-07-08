//! High-confidence secret detection for the clipboard capture pipeline (§2).
//!
//! v1 deliberately sticks to a short list of high-confidence, low-noise
//! patterns (PEM private keys, cloud/API tokens). It does *not* attempt
//! entropy heuristics or mnemonic-phrase detection — both have high false
//! positive rates against ordinary text, and a false "why wasn't this
//! captured" is more confusing to users than an occasional missed secret.

use regex::Regex;
use std::sync::LazyLock;

static PEM_PRIVATE_KEY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"-----BEGIN (RSA |EC |OPENSSH |ENCRYPTED |)PRIVATE KEY-----").unwrap()
});

static AWS_ACCESS_KEY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bAKIA[0-9A-Z]{16}\b").unwrap());

static AWS_TEMP_KEY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bASIA[0-9A-Z]{16}\b").unwrap());

static JWT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b").unwrap()
});

static GITHUB_TOKEN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{36,}\b").unwrap());

static GITHUB_FINE_GRAINED_PAT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bgithub_pat_[A-Za-z0-9_]{60,}\b").unwrap());

static SLACK_TOKEN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bxox[baprs]-[A-Za-z0-9-]{10,}\b").unwrap());

static OPENAI_STYLE_KEY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bsk-[A-Za-z0-9_-]{20,}\b").unwrap());

/// Returns the kind of secret found in `text` (for logging), or `None` if no
/// high-confidence pattern matched. Checked in a fixed order; the first match
/// wins since a caller only needs to know *that* the text should be skipped.
pub fn detect_secret(text: &str) -> Option<&'static str> {
    if PEM_PRIVATE_KEY.is_match(text) {
        return Some("PEM private key");
    }
    if AWS_ACCESS_KEY.is_match(text) {
        return Some("AWS access key");
    }
    if AWS_TEMP_KEY.is_match(text) {
        return Some("AWS temporary key");
    }
    if JWT.is_match(text) {
        return Some("JWT");
    }
    if GITHUB_TOKEN.is_match(text) {
        return Some("GitHub token");
    }
    if GITHUB_FINE_GRAINED_PAT.is_match(text) {
        return Some("GitHub fine-grained PAT");
    }
    if SLACK_TOKEN.is_match(text) {
        return Some("Slack token");
    }
    if OPENAI_STYLE_KEY.is_match(text) {
        return Some("OpenAI/Anthropic-style API key");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // Shared negative samples that must never be flagged by *any* pattern,
    // regardless of which specific regex is under test (SPEC-4 §2).
    const PLAIN_CHINESE_TEXT: &str = "今天天气不错，我们去公园散步吧。";
    const PLAIN_URL: &str = "https://example.com/docs/getting-started?ref=readme";
    const SHA256_HEX: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    const BASE64_IMAGE_FRAGMENT: &str =
        "iVBORw0KGgoAAAANSUhEUgAAAAUAAAAFCAYAAACNbyblAAAAHElEQVQI12P4//8/w38GIAXDCBKAAA==";
    const JWT_WITH_ONLY_TWO_SEGMENTS: &str = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0";

    #[test]
    fn common_non_secret_samples_are_never_flagged() {
        assert_eq!(None, detect_secret(PLAIN_CHINESE_TEXT));
        assert_eq!(None, detect_secret(PLAIN_URL));
        assert_eq!(None, detect_secret(SHA256_HEX));
        assert_eq!(None, detect_secret(BASE64_IMAGE_FRAGMENT));
        assert_eq!(None, detect_secret(JWT_WITH_ONLY_TWO_SEGMENTS));
    }

    #[test]
    fn detects_pem_private_key_block() {
        let pem = "-----BEGIN RSA PRIVATE KEY-----\nMIIEow...\n-----END RSA PRIVATE KEY-----";
        assert_eq!(Some("PEM private key"), detect_secret(pem));

        // The empty-prefix alternative (plain "PRIVATE KEY", e.g. PKCS#8) also
        // matches per the spec's `(RSA |EC |OPENSSH |ENCRYPTED |)` group.
        let pkcs8 = "-----BEGIN PRIVATE KEY-----\nMIIEvQ...\n-----END PRIVATE KEY-----";
        assert_eq!(Some("PEM private key"), detect_secret(pkcs8));
    }

    #[test]
    fn does_not_flag_text_merely_mentioning_private_keys() {
        let text = "Remember to rotate your RSA private key before the audit.";
        assert_eq!(None, detect_secret(text));
    }

    #[test]
    fn detects_aws_access_key() {
        let key = ["AKIA", "IOSFODNN7EXAMPLE"].concat();
        let text = format!("aws_access_key_id = {key}");
        assert_eq!(Some("AWS access key"), detect_secret(&text));
    }

    #[test]
    fn rejects_aws_access_key_with_wrong_suffix_length() {
        // 15 chars after AKIA instead of the required 16 - must not match.
        let text = "AKIAIOSFODNN7EXAMPL is not a real key";
        assert_eq!(None, detect_secret(text));
    }

    #[test]
    fn detects_aws_temporary_key() {
        let key = ["ASIA", "IOSFODNN7EXAMPLE"].concat();
        let text = format!("aws_session token uses {key}");
        assert_eq!(Some("AWS temporary key"), detect_secret(&text));
    }

    #[test]
    fn rejects_asia_prefixed_word_that_is_not_a_key() {
        // Lowercase "sia" fails the case-sensitive "ASIA" match entirely,
        // unlike an all-caps word that happens to be 20 letters long.
        let text = "Asia is a large continent with many countries.";
        assert_eq!(None, detect_secret(text));
    }

    #[test]
    fn detects_jwt() {
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        assert_eq!(Some("JWT"), detect_secret(jwt));
    }

    #[test]
    fn does_not_flag_eyj_prefixed_string_with_only_two_segments() {
        assert_eq!(None, detect_secret(JWT_WITH_ONLY_TWO_SEGMENTS));
    }

    #[test]
    fn detects_github_classic_token() {
        let token = format!("ghp_{}", "A".repeat(36));
        assert_eq!(Some("GitHub token"), detect_secret(&token));
    }

    #[test]
    fn rejects_github_token_with_short_suffix() {
        let token = format!("ghp_{}", "A".repeat(20));
        assert_eq!(None, detect_secret(&token));
    }

    #[test]
    fn detects_github_fine_grained_pat() {
        let token = format!("github_pat_{}", "A".repeat(60));
        assert_eq!(Some("GitHub fine-grained PAT"), detect_secret(&token));
    }

    #[test]
    fn rejects_github_fine_grained_pat_with_short_suffix() {
        let token = format!("github_pat_{}", "A".repeat(30));
        assert_eq!(None, detect_secret(&token));
    }

    #[test]
    fn detects_slack_token() {
        // Built at runtime so the literal never looks like a real token to
        // GitHub's push-protection scanner (same trick as the ghp_ tests).
        let token = format!(
            "xoxb-{}-{}",
            "1234567890123", "abcdefghijklmnopqrstuvwxyz0123456789"
        );
        assert_eq!(Some("Slack token"), detect_secret(&token));
    }

    #[test]
    fn rejects_xox_prefixed_word_with_unknown_kind_letter() {
        // 'z' is not one of Slack's documented token kinds (b/a/p/r/s).
        let text = "xoxz-1234567890123-abcdefghijklmnopqrstuvwxyz0123456789";
        assert_eq!(None, detect_secret(text));
    }

    #[test]
    fn detects_openai_style_key() {
        let key = "sk-abcdefghijklmnopqrstuvwxyz0123456789";
        assert_eq!(Some("OpenAI/Anthropic-style API key"), detect_secret(key));
    }

    #[test]
    fn rejects_openai_style_key_with_short_suffix() {
        let key = "sk-shortkey";
        assert_eq!(None, detect_secret(key));
    }
}
