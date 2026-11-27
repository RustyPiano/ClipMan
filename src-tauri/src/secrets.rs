use regex::RegexSet;
use std::sync::LazyLock;

const SECRET_PATTERNS: [(&str, &str); 14] = [
    (
        "PEM private key",
        r"-----BEGIN (RSA |EC |DSA |OPENSSH |ENCRYPTED |)PRIVATE KEY-----",
    ),
    ("PGP private key", r"-----BEGIN PGP PRIVATE KEY BLOCK-----"),
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
    ("Stripe live key", r"\b(sk|rk)_live_[0-9A-Za-z]{24,}\b"),
    (
        "Google API key",
        r"\bAIza[0-9A-Za-z_-]{35}([^0-9A-Za-z_-]|$)",
    ),
    (
        "GitLab personal access token",
        r"\bglpat-[0-9A-Za-z_-]{20,}",
    ),
    ("npm access token", r"\bnpm_[A-Za-z0-9]{36}\b"),
    (
        "Connection URL with password",
        r"\b(postgres(ql)?|mysql|mongodb|rediss?|amqps?)(\+[0-9a-z]+)?://[^\s:/@]*:[^\s/@$<{*][^\s/@]*@[^\s/@]",
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
        let samples: [(&str, String); 23] = [
            (
                "PEM private key",
                "-----BEGIN RSA PRIVATE KEY-----\nMIIEow...\n-----END RSA PRIVATE KEY-----".into(),
            ),
            (
                "PEM private key",
                "-----BEGIN PRIVATE KEY-----\nMIIEvQ...\n-----END PRIVATE KEY-----".into(),
            ),
            (
                "PEM private key",
                "-----BEGIN DSA PRIVATE KEY-----\nMIIBuw...\n-----END DSA PRIVATE KEY-----".into(),
            ),
            (
                "PEM private key",
                "-----BEGIN ENCRYPTED PRIVATE KEY-----\nMIIFHz...\n-----END ENCRYPTED PRIVATE KEY-----"
                    .into(),
            ),
            (
                "PGP private key",
                "-----BEGIN PGP PRIVATE KEY BLOCK-----\n\nlQOYBF...\n-----END PGP PRIVATE KEY BLOCK-----"
                    .into(),
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
            (
                "Stripe live key",
                format!("STRIPE_SECRET_KEY={}_live_{}", "sk", "a1B2c3D4e5F6g7H8i9J0k1L2"),
            ),
            (
                "Stripe live key",
                format!("{}_live_{}", "rk", "a1B2c3D4".repeat(4)),
            ),
            (
                "Google API key",
                format!("key={}{}", "AIza", "SyA1b2C3d4E5f6G7h8I9j0K_LmNoPqRs-TU"),
            ),
            (
                "Google API key",
                format!("{}{}&callback=init", "AIza", "SyA1b2C3d4E5f6G7h8I9j0K_LmNoPqRs-T-"),
            ),
            (
                "GitLab personal access token",
                format!("{}-{}", "glpat", "xYz12AbC34dEf56GhI78"),
            ),
            (
                "npm access token",
                format!("//registry.npmjs.org/:_authToken={}_{}", "npm", "a1B2c3".repeat(6)),
            ),
            (
                "Connection URL with password",
                format!("postgresql://app:{}@db.internal:5432/app", "s3cr3t"),
            ),
            (
                "Connection URL with password",
                format!("mongodb+srv://admin:{}@cluster0.example.net/?retryWrites=true", "Tr0ub4dor"),
            ),
            (
                "Connection URL with password",
                format!("redis://:{}@cache:6379/0", "hunter2"),
            ),
            (
                "Connection URL with password",
                format!("mysql+pymysql://root:{}@127.0.0.1/shop", "p:ss"),
            ),
            (
                "Connection URL with password",
                format!("amqps://guest:{}@rabbit.example.com/vhost", "guest"),
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
            "-----BEGIN PUBLIC KEY-----\nMFkwEw...\n-----END PUBLIC KEY-----".into(),
            "-----BEGIN PGP PUBLIC KEY BLOCK-----\n\nmQINBF...\n-----END PGP PUBLIC KEY BLOCK-----"
                .into(),
            format!("{}_test_{}", "sk", "a1B2c3D4e5F6g7H8i9J0k1L2"),
            format!("{}_live_{}", "pk", "a1B2c3D4e5F6g7H8i9J0k1L2"),
            format!("{}_live_{}", "sk", "short"),
            format!("{}{}", "AIza", "SyA1b2C3d4E5f6G7h8I9"),
            format!("{}{}", "AIza", "SyA1b2C3d4E5f6G7h8I9j0K_LmNoPqRs-TUVWX"),
            "Use a glpat-token from your GitLab profile.".into(),
            format!("{}-{}", "glpat", "short"),
            "npm_config_registry=https://registry.npmjs.org/".into(),
            format!("{}_{}", "npm", "a1B2c3".repeat(3)),
            "postgres://app@localhost/app".into(),
            "postgres://app:@localhost/app".into(),
            "redis://localhost:6379/0".into(),
            "mongodb+srv://<username>:<password>@cluster0.example.net".into(),
            "DATABASE_URL=postgres://app:${DB_PASSWORD}@db:5432/app".into(),
            "mysql://root:{{password}}@localhost/shop".into(),
            "postgres://app:****@localhost/app".into(),
            "https://user:hunter2@example.com/private".into(),
        ];
        for sample in &samples {
            assert_eq!(None, detect_secret(sample), "{sample}");
        }
    }
}
