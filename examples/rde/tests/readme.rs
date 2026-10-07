//! The README tutorial walks through this example. Every code block marked with
//! `<!-- snippet: <path> -->` must still exist, line by line, in the file at `<path>`.

use std::fs;
use std::path::Path;

// --- Snippets ----------------------------------------------------------------

struct Snippet {
    path: String,
    lines: Vec<String>,
}

/// The code blocks of the README that come right after a `<!-- snippet: <path> -->` marker.
fn snippets(readme: &str) -> Vec<Snippet> {
    let mut snippets = vec![];
    let mut lines = readme.lines();

    while let Some(line) = lines.next() {
        let Some(path) = line
            .strip_prefix("<!-- snippet: ")
            .and_then(|rest| rest.strip_suffix(" -->"))
        else {
            continue;
        };

        let _opening_fence = lines.next();
        let code = lines.by_ref().take_while(|l| !l.starts_with("```"));

        snippets.push(Snippet {
            path: path.to_string(),
            lines: code.map(|l| l.trim().to_string()).collect(),
        });
    }

    snippets
}

/// Whether `snippet` appears in `source` as consecutive lines, ignoring indentation.
fn appears_in(snippet: &[String], source: &str) -> bool {
    let source: Vec<&str> = source.lines().map(str::trim).collect();

    source
        .windows(snippet.len())
        .any(|window| window.iter().zip(snippet).all(|(a, b)| *a == b))
}

// --- Test --------------------------------------------------------------------

#[test]
fn every_readme_snippet_matches_its_source_file() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let readme = fs::read_to_string(workspace.join("README.md")).unwrap();

    let snippets = snippets(&readme);
    assert!(!snippets.is_empty(), "the README has no snippets");

    let stale: Vec<&str> = snippets
        .iter()
        .filter(|snippet| {
            let source = fs::read_to_string(workspace.join(&snippet.path)).unwrap();
            !appears_in(&snippet.lines, &source)
        })
        .map(|snippet| snippet.path.as_str())
        .collect();

    assert!(
        stale.is_empty(),
        "README snippets out of date with: {stale:?}"
    );
}
