use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use ra_ap_rustc_lexer::{tokenize, FrontmatterAllowed, TokenKind};

const SUPPRESSIONS: &str = "tests/comment-suppressions.json";
const PRUNE_ENV: &str = "PRUNE_COMMENT_SUPPRESSIONS";

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}

fn comment_lines(source: &str) -> Vec<usize> {
    let mut offset = 0;
    let mut lines = Vec::new();
    for token in tokenize(source, FrontmatterAllowed::No) {
        if matches!(
            token.kind,
            TokenKind::LineComment { .. } | TokenKind::BlockComment { .. }
        ) {
            lines.push(source[..offset].matches('\n').count() + 1);
        }
        offset += token.len as usize;
    }
    lines
}

#[test]
fn comment_count_never_grows() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let suppressions_path = root.join(SUPPRESSIONS);
    let suppressions: BTreeMap<String, usize> =
        serde_json::from_str(&fs::read_to_string(&suppressions_path).unwrap()).unwrap();

    let mut files = vec![root.join("build.rs")];
    collect_rust_files(&root.join("src"), &mut files);
    collect_rust_files(&root.join("tests"), &mut files);

    let mut current = BTreeMap::new();
    let mut new_comments = Vec::new();
    for path in files {
        let relative = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let lines = comment_lines(&fs::read_to_string(&path).unwrap());
        let allowed = suppressions.get(&relative).copied().unwrap_or(0);
        if lines.len() > allowed {
            new_comments.extend(lines.iter().map(|line| format!("{relative}:{line}")));
        }
        current.insert(relative, lines.len());
    }

    let tightened: BTreeMap<&String, usize> = suppressions
        .iter()
        .map(|(path, &allowed)| (path, allowed.min(current.get(path).copied().unwrap_or(0))))
        .filter(|&(_, count)| count > 0)
        .collect();
    let prunable = tightened.len() != suppressions.len()
        || tightened
            .iter()
            .any(|(path, &count)| suppressions[*path] != count);
    if prunable && std::env::var_os(PRUNE_ENV).is_some() {
        let json = serde_json::to_string_pretty(&tightened).unwrap();
        fs::write(&suppressions_path, json + "\n").unwrap();
    }

    assert!(
        new_comments.is_empty(),
        "代码里不写注释，以下文件的注释数超过 {SUPPRESSIONS} 的上限。删除新增的注释；不明显的设计决定写进 docs/adr/，权宜实现提 GitHub Issue：\n{}",
        new_comments.join("\n")
    );
    assert!(
        !prunable || std::env::var_os(PRUNE_ENV).is_some(),
        "注释已减少，运行 `{PRUNE_ENV}=1 cargo test --test no_comments` 收紧 {SUPPRESSIONS}"
    );
}
