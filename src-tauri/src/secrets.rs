use regex::RegexSet;
use std::sync::LazyLock;

const SECRET_PATTERNS: [(&str, &str); 8] = [
    (
        "PEM private key",
        r"-----BEGIN (RSA |EC |OPENSSH |ENCRYPTED |)PRIVATE KEY-----",
    ),
    ("AWS access key", r"\bAKIA[0-9A-Z]{16}\b"),
    ("AWS temporary key", r"\bASIA[0-9A-Z]{16}\b"),
    (
        "JWT",
        r"\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b",
    ),
    (
        "GitHub token",
        r"\b(ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{36,}\b",
    ),
    (
        "GitHub fine-grained PAT",
        r"\bgithub_pat_[A-Za-z0-9_]{60,}\b",
    ),
    ("Slack token", r"\bxox[baprs]-[A-Za-z0-9-]{10,}\b"),
    (
        "OpenAI/Anthropic-style API key",
        r"\bsk-[A-Za-z0-9_-]{20,}\b",
    ),
];

static SECRET_SET: LazyLock<RegexSet> =
    LazyLock::new(|| RegexSet::new(SECRET_PATTERNS.map(|(_, pattern)| pattern)).unwrap());

pub fn detect_secret(text: &str) -> Option<&'static str> {
    SECRET_SET
        .matches(text)
        .iter()
        .next()
        .map(|index| SECRET_PATTERNS[index].0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_each_secret_kind() {
        let samples: [(&str, String); 9] = [
            (
                "PEM private key",
                "-----BEGIN RSA PRIVATE KEY-----\nMIIEow...\n-----END RSA PRIVATE KEY-----".into(),
            ),
            (
                "PEM private key",
                "-----BEGIN PRIVATE KEY-----\nMIIEvQ...\n-----END PRIVATE KEY-----".into(),
            ),
            (
                "AWS access key",
                format!("aws_access_key_id = {}", ["AKIA", "IOSFODNN7EXAMPLE"].concat()),
            ),
            (
                "AWS temporary key",
                format!("aws_session token uses {}", ["ASIA", "IOSFODNN7EXAMPLE"].concat()),
            ),
            (
                "JWT",
                "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c".into(),
            ),
            ("GitHub token", format!("ghp_{}", "A".repeat(36))),
            (
                "GitHub fine-grained PAT",
                format!("github_pat_{}", "A".repeat(60)),
            ),
            (
                "Slack token",
                format!(
                    "xoxb-{}-{}",
                    "1234567890123", "abcdefghijklmnopqrstuvwxyz0123456789"
                ),
            ),
            (
                "OpenAI/Anthropic-style API key",
                "sk-abcdefghijklmnopqrstuvwxyz0123456789".into(),
            ),
        ];
        for (kind, sample) in &samples {
            assert_eq!(Some(*kind), detect_secret(sample), "{sample}");
        }
    }

    #[test]
    fn common_non_secret_samples_are_never_flagged() {
        let samples = [
            "今天天气不错，我们去公园散步吧。".to_string(),
            "https://example.com/docs/getting-started?ref=readme".into(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
            "iVBORw0KGgoAAAANSUhEUgAAAAUAAAAFCAYAAACNbyblAAAAHElEQVQI12P4//8/w38GIAXDCBKAAA=="
                .into(),
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0".into(),
            "Remember to rotate your RSA private key before the audit.".into(),
            "AKIAIOSFODNN7EXAMPL is not a real key".into(),
            "Asia is a large continent with many countries.".into(),
            format!("ghp_{}", "A".repeat(20)),
            format!("github_pat_{}", "A".repeat(30)),
            "xoxz-1234567890123-abcdefghijklmnopqrstuvwxyz0123456789".into(),
            "sk-shortkey".into(),
        ];
        for sample in &samples {
            assert_eq!(None, detect_secret(sample), "{sample}");
        }
    }
}
