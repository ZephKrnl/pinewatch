//! Repository tasks. Dev only, never shipped. Run with `cargo xtask <task>`.
//!
//! - `check-deps`: the format crates' production dependencies stay within the allowlist (spec section 4).

use std::process::{Command, ExitCode};

/// The external crates format crates may depend on, together with everything below them.
const ALLOWED_ROOTS: [&str; 3] = ["sha1", "png", "windows-sys"];

/// The crates the allowlist applies to. Each plan that adds a format crate appends it here.
const FORMAT_CRATES: [&str; 5] =
    ["pinewatch-mono", "pinewatch-io", "pinewatch-kvt", "pinewatch-level", "pinewatch-unity"];

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("check-deps") => check_deps(),
        other => {
            eprintln!("unknown task: {}", other.unwrap_or(""));
            eprintln!("tasks: check-deps");
            ExitCode::from(2)
        }
    }
}

/// A crate's normal dependency tree for every target, as `(depth, package name)` in depth-first order, from
/// `cargo tree -e normal --prefix depth --no-dedupe --target all -p <crate>`.
fn dependency_tree(krate: &str) -> Result<Vec<(usize, String)>, String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let out = Command::new(cargo)
        .args(["tree", "-e", "normal", "--prefix", "depth", "--no-dedupe", "--target", "all", "--offline", "-p", krate])
        .output()
        .map_err(|e| format!("can't run cargo tree: {e}"))?;
    if !out.status.success() {
        return Err(format!("cargo tree -p {krate} failed:\n{}", String::from_utf8_lossy(&out.stderr)));
    }
    let mut nodes = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let digits = line.chars().take_while(char::is_ascii_digit).count();
        let (Ok(depth), Some(name)) = (line[..digits].parse(), line[digits..].split_whitespace().next()) else {
            continue;
        };
        nodes.push((depth, name.to_string()));
    }
    Ok(nodes)
}

/// Every edge to a package that is neither a Pinewatch crate, nor an allowed root, nor below an allowed root.
fn disallowed(nodes: &[(usize, String)]) -> Vec<String> {
    // The path from the root to the current node: (depth, name, inside an allowed root's subtree).
    let mut path: Vec<(usize, &str, bool)> = Vec::new();
    let mut found = Vec::new();
    for (depth, name) in nodes {
        while path.last().is_some_and(|(d, _, _)| d >= depth) {
            path.pop();
        }
        let parent_allows = path.last().is_some_and(|&(_, _, inside)| inside);
        let is_root = ALLOWED_ROOTS.contains(&name.as_str());
        if !name.starts_with("pinewatch-") && !is_root && !parent_allows {
            let parent = path.last().map_or("", |&(_, parent, _)| parent);
            found.push(format!("{parent} -> {name}"));
        }
        path.push((*depth, name, is_root || parent_allows));
    }
    found
}

fn check_deps() -> ExitCode {
    let mut offending = Vec::new();
    for krate in FORMAT_CRATES {
        match dependency_tree(krate) {
            Ok(nodes) if nodes.is_empty() => offending.push(format!("{krate}: empty dependency tree")),
            Ok(nodes) => offending.extend(disallowed(&nodes).into_iter().map(|edge| format!("{krate}: {edge}"))),
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::FAILURE;
            }
        }
    }

    if offending.is_empty() {
        println!("check-deps: {} format crates use only {} and their dependencies", FORMAT_CRATES.len(), ALLOWED_ROOTS.join(", "));
        ExitCode::SUCCESS
    } else {
        eprintln!("Dependencies outside the allowlist ({}):", ALLOWED_ROOTS.join(", "));
        for line in offending {
            eprintln!("  {line}");
        }
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::disallowed;

    fn tree(lines: &[(usize, &str)]) -> Vec<(usize, String)> {
        lines.iter().map(|&(d, n)| (d, n.to_string())).collect()
    }

    #[test]
    fn packages_below_an_allowed_root_pass() {
        let nodes = tree(&[(0, "pinewatch-level"), (1, "pinewatch-io"), (2, "windows-sys"), (3, "windows-link"), (1, "sha1"), (2, "cfg-if")]);
        assert!(disallowed(&nodes).is_empty());
    }

    #[test]
    fn a_direct_dependency_outside_the_allowlist_fails_even_if_an_allowed_root_also_uses_it() {
        let nodes = tree(&[(0, "pinewatch-mono"), (1, "sha1"), (2, "cfg-if"), (1, "cfg-if")]);
        assert_eq!(disallowed(&nodes), ["pinewatch-mono -> cfg-if"]);
    }
}
