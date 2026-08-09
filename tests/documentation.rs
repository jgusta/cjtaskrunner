mod common;

use common::repository_root;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn directive_names() -> Vec<String> {
    let output = Command::new(env!("CARGO_BIN_EXE_cj"))
        .arg("--directives")
        .env("NO_COLOR", "1")
        .output()
        .expect("run cj --directives");
    assert!(
        output.status.success(),
        "cj --directives failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout)
        .expect("directive output utf8")
        .lines()
        .filter_map(|line| {
            let name = line.split_whitespace().next()?;
            name.strip_prefix('@').map(str::to_string)
        })
        .collect()
}

fn markdown_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read documentation directory") {
        let path = entry.expect("documentation entry").path();
        if path.is_dir() {
            markdown_files(&path, files);
        } else if path.extension().and_then(|value| value.to_str()) == Some("md") {
            files.push(path);
        }
    }
}

fn markdown_links(source: &str) -> Vec<String> {
    let mut prose = String::new();
    let mut fence = None;
    for line in source.lines() {
        let trimmed = line.trim_start();
        let backticks = trimmed.chars().take_while(|value| *value == '`').count();
        if let Some(opening) = fence {
            if backticks >= opening && trimmed.chars().all(|value| value == '`') {
                fence = None;
            }
            continue;
        }
        if backticks >= 3 {
            fence = Some(backticks);
            continue;
        }
        prose.push_str(line);
        prose.push('\n');
    }

    let mut links = Vec::new();
    let mut remaining = prose.as_str();

    while let Some(start) = remaining.find("](") {
        let after_start = &remaining[start + 2..];
        let Some(end) = after_start.find(')') else {
            break;
        };
        links.push(after_start[..end].to_string());
        remaining = &after_start[end + 1..];
    }

    links
}

#[test]
fn every_directive_is_documented_once_on_the_single_directives_page() {
    let root = repository_root();
    let page_path = root.join("docs/src/reference/directives.md");
    let page = fs::read_to_string(&page_path).expect("read directive reference");

    assert!(
        page.contains("```cjtasks"),
        "{} must contain cjtasks examples",
        page_path.display()
    );
    assert!(
        !root.join("docs/directives").exists(),
        "directive documentation must not use a nested directory"
    );

    for directive in directive_names() {
        let directive_token = format!("`@{directive}`");
        if directive == "if-not" || directive.starts_with("if-not-") {
            let anchor = format!("<a id=\"{}\"></a>", directive.trim_end_matches(':'));
            assert!(
                page.contains(&anchor),
                "{directive_token:?} must keep a same-page anchor"
            );
            assert!(
                !page
                    .lines()
                    .any(|line| line.starts_with("### ") && line.contains(&directive_token)),
                "{directive_token:?} must be documented under its positive @if heading"
            );
            assert!(
                !page
                    .lines()
                    .any(|line| line.starts_with("- ") && line.contains(&directive_token)),
                "{directive_token:?} must be documented under its positive @if index entry"
            );
        } else {
            let matches = page
                .lines()
                .filter(|line| line.starts_with("### ") && line.contains(&directive_token))
                .count();
            assert!(
                matches == 1,
                "{directive_token:?} must appear in exactly one directive heading, found {matches}"
            );
            assert!(
                page.lines().any(|line| line.starts_with("- ")
                    && line.contains(&directive_token)
                    && line.contains("](#")
                    && line.contains(" - ")),
                "directive index must include @{} using same-page dash syntax",
                directive,
            );
        }
    }
}

#[test]
fn every_if_directive_has_a_matching_if_not_directive() {
    let directives = directive_names();
    assert!(
        directives.contains(&"if-not".to_string()),
        "@if must have matching @if-not"
    );
    for directive in directives
        .iter()
        .filter(|name| name.starts_with("if-") && *name != "if-not" && !name.starts_with("if-not-"))
    {
        let counterpart = format!("if-not-{}", directive.trim_start_matches("if-"));
        assert!(
            directives.contains(&counterpart),
            "@{directive} must have matching @{counterpart}"
        );
    }
}

#[test]
fn user_documentation_uses_cjtasks_fences_for_taskfiles() {
    let root = repository_root();
    let mut files = vec![root.join("README.md")];
    markdown_files(&root.join("docs/src"), &mut files);

    for path in files {
        let source = fs::read_to_string(&path).expect("read markdown");
        assert!(
            !source.contains("```yaml"),
            "{} uses a yaml fence; taskfiles must use cjtasks",
            path.display()
        );
    }
}

#[test]
fn local_documentation_links_resolve() {
    let root = repository_root();
    let mut files = vec![root.join("README.md")];
    markdown_files(&root.join("docs/src"), &mut files);

    for path in files {
        let source = fs::read_to_string(&path).expect("read markdown");
        for link in markdown_links(&source) {
            if link.starts_with("http://")
                || link.starts_with("https://")
                || link.starts_with("mailto:")
                || link.starts_with('#')
            {
                continue;
            }
            let target = link.split('#').next().expect("link path");
            if target.is_empty() {
                continue;
            }
            let resolved = path.parent().expect("markdown parent").join(target);
            assert!(
                resolved.exists(),
                "{} contains unresolved link {link:?}",
                path.display()
            );
        }
    }
}

#[test]
fn canonical_logo_exists() {
    let root = repository_root();
    let logo = root.join("logo/cj-logo-color-d.svg");

    assert!(logo.is_file(), "canonical logo is missing");
}
