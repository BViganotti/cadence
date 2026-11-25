//! Repository-level invariants: std-only dependencies, no CI, docs present.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn walk_rust_files(directory: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) == Some("target") {
                continue;
            }
            walk_rust_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn workspace_uses_only_path_dependencies() {
    let mut manifests = vec![root().join("Cargo.toml")];
    for member in [
        "crates/cadence-core",
        "crates/cadence-import",
        "crates/cadence-engrave",
        "crates/cadence-export",
        "crates/cadence-studio",
        "fuzz",
        "bench",
        "fixtures",
    ] {
        manifests.push(root().join(member).join("Cargo.toml"));
    }
    for manifest in manifests {
        let text = std::fs::read_to_string(&manifest).unwrap();
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || !trimmed.contains('=') {
                continue;
            }
            if trimmed.starts_with("name")
                || trimmed.starts_with("version")
                || trimmed.starts_with("edition")
                || trimmed.starts_with("description")
                || trimmed.starts_with("license")
                || trimmed.starts_with("publish")
                || trimmed.starts_with("path")
                || trimmed.starts_with("opt-level")
                || trimmed.starts_with("lto")
                || trimmed.starts_with("codegen-units")
                || trimmed.starts_with("members")
                || trimmed.starts_with("default-members")
            {
                continue;
            }
            if trimmed.contains('=') && trimmed.contains('"') {
                if trimmed.contains("path =") {
                    continue;
                }
                let dependency_like = trimmed
                    .split('=')
                    .next()
                    .map(|name| {
                        name.trim()
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                    })
                    .unwrap_or(false);
                if dependency_like {
                    panic!(
                        "non-path dependency in {}: {trimmed}",
                        manifest.display()
                    );
                }
            }
        }
    }
}

#[test]
fn no_ci_workflows_exist() {
    assert!(!root().join(".github/workflows").exists());
    assert!(!root().join(".gitlab-ci.yml").exists());
    assert!(!root().join(".circleci").exists());
}

#[test]
fn no_unsafe_code_exists_in_rust_sources() {
    let mut files = Vec::new();
    for directory in ["src", "crates", "fuzz", "bench", "fixtures", "tests"] {
        walk_rust_files(&root().join(directory), &mut files);
    }
    assert!(!files.is_empty());
    for file in files {
        if file.ends_with("repository.rs") {
            continue;
        }
        let text = std::fs::read_to_string(&file).unwrap();
        for (index, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            if code.contains("unsafe {") || code.contains("unsafe impl") {
                panic!("unsafe code at {}:{}", file.display(), index + 1);
            }
        }
    }
}

#[test]
fn required_documents_exist() {
    for path in [
        "README.md",
        "LICENSE",
        "CHANGELOG.md",
        "LIMITATIONS.md",
        "SECURITY.md",
        "CONTRIBUTING.md",
        "FINAL_REPORT.md",
        "docs/architecture.md",
        "docs/musicxml-subset.md",
        "docs/engraving.md",
        "docs/studio-api.md",
        "docs/testing.md",
        "examples/README.md",
        "fuzz/README.md",
        "bench/README.md",
    ] {
        assert!(root().join(path).exists(), "missing {path}");
    }
}

#[test]
fn assets_have_no_external_runtime_resources() {
    for path in ["assets/index.html", "assets/app.css", "assets/app.js"] {
        let text = std::fs::read_to_string(root().join(path)).unwrap();
        assert!(!text.contains("https://"), "{path}");
        assert!(!text.contains("http://"), "{path}");
        assert!(!text.contains("cdn."), "{path}");
        assert!(!text.contains("@import"), "{path}");
    }
}

#[test]
fn demo_musicxml_imports_and_is_deterministic() {
    let first = std::fs::read_to_string(root().join("examples/demo.musicxml")).unwrap();
    let second = first.clone();
    let a = cadence_import::import_score(&first).unwrap();
    let b = cadence_import::import_score(&second).unwrap();
    assert_eq!(a.metadata.title, "Cadence in G");
    assert_eq!(a.note_count(), b.note_count());
    assert_eq!(a.parts.len(), 2);
}
