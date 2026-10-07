//! Where packages come from (decision AC1): a directory registry, read in
//! place, or a URL registry, fetched into the store under the project root;
//! the content hash of a package; and the textual joining of paths that
//! every toolchain does the same way, so that a file is named the same in
//! every diagnostic.

use std::fmt;

use sha2::{Digest, Sha256};

use crate::version::Version;

/// The store of a URL registry's packages, under the project root.
pub const STORE: &str = ".renyi/packages";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Registry {
    /// A directory, as the manifest's `registry` names it joined to the
    /// project root.
    Directory(String),
    /// An `http://` or `https://` base, without a trailing `/`.
    Url(String),
}

impl Registry {
    /// The manifest's `registry` field read from a project root.
    pub fn parse(spec: &str, root: &str) -> Registry {
        if spec.starts_with("http://") || spec.starts_with("https://") {
            Registry::Url(spec.trim_end_matches('/').to_string())
        } else if is_absolute(spec) {
            Registry::Directory(join("", spec))
        } else {
            Registry::Directory(join(root, spec))
        }
    }

    pub fn is_url(&self) -> bool {
        matches!(self, Registry::Url(_))
    }

    /// Where one version of a package is read from: the registry itself
    /// for a directory, the store for a URL.
    pub fn package_root(&self, project_root: &str, name: &str, version: Version) -> String {
        match self {
            Registry::Directory(directory) => join(directory, &format!("{name}/{version}")),
            Registry::Url(_) => join(project_root, &format!("{STORE}/{name}/{version}")),
        }
    }
}

impl fmt::Display for Registry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Registry::Directory(directory) => write!(f, "{directory}"),
            Registry::Url(url) => write!(f, "{url}"),
        }
    }
}

/// `sha256:` and the hex digest, as the index spells content hashes.
pub fn hash_of(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

/// A path that does not depend on the working directory: from the root, or
/// from a drive on Windows.
pub fn is_absolute(path: &str) -> bool {
    path.starts_with('/') || path.starts_with('\\') || path.chars().nth(1) == Some(':')
}

/// `base/relative` with `/` as the separator and the `.` and `..` segments
/// folded; an empty base is the working directory, and an absolute
/// `relative` replaces the base. Textual: nothing is looked up on disk, so
/// every toolchain names a file the same way.
pub fn join(base: &str, relative: &str) -> String {
    let mut segments: Vec<String> = Vec::new();
    let mut rooted = false;
    for (index, part) in [base, relative].into_iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if index == 1 && is_absolute(part) {
            segments.clear();
            rooted = false;
        }
        let slashed = part.replace('\\', "/");
        if slashed.starts_with('/') && segments.is_empty() {
            rooted = true;
        }
        for segment in slashed.split('/') {
            match segment {
                "" | "." => {}
                ".." => {
                    let drive = segments.len() == 1 && segments[0].ends_with(':');
                    match segments.last() {
                        Some(last) if last != ".." && !drive => {
                            segments.pop();
                        }
                        _ if rooted || drive => {}
                        _ => segments.push("..".to_string()),
                    }
                }
                _ => segments.push(segment.to_string()),
            }
        }
    }
    let joined = segments.join("/");
    if rooted {
        format!("/{joined}")
    } else {
        joined
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_join_textually() {
        assert_eq!(join("tests/project", "../registry"), "tests/registry");
        assert_eq!(join("", "../registry"), "../registry");
        assert_eq!(join("a/b", "c/./d"), "a/b/c/d");
        assert_eq!(join("a", "../../x"), "../x");
        assert_eq!(join("D:/work/p", "../r"), "D:/work/r");
        assert_eq!(join("D:/work", "../../r"), "D:/r");
        assert_eq!(join("/home/p", "../r"), "/home/r");
        assert_eq!(join("/p", "../../r"), "/r");
        assert_eq!(join("a\\b", "c"), "a/b/c");
        assert_eq!(join("a/b", "/abs/c"), "/abs/c");
        assert_eq!(join("a/b", "D:\\abs\\c"), "D:/abs/c");
        assert_eq!(join("a", ""), "a");
    }

    #[test]
    fn a_registry_is_a_directory_or_a_url() {
        let version = Version::parse("1.0.0").unwrap();
        let directory = Registry::parse("../registry", "tests/project");
        assert_eq!(directory, Registry::Directory("tests/registry".to_string()));
        assert_eq!(
            directory.package_root("tests/project", "greeting", version),
            "tests/registry/greeting/1.0.0"
        );
        let url = Registry::parse("https://example.org/registry/", "tests/project");
        assert_eq!(
            url,
            Registry::Url("https://example.org/registry".to_string())
        );
        assert_eq!(
            url.package_root("tests/project", "greeting", version),
            "tests/project/.renyi/packages/greeting/1.0.0"
        );
        assert_eq!(hash_of(b"").len(), "sha256:".len() + 64);
    }
}
