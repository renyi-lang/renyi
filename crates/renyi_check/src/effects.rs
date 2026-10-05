//! Capabilities and their coverage (syntax sketch section 11, decision J11).

use renyi_syntax::ast;

/// `filesystem.read("data")`: a path in the capability tree with an optional
/// literal scope.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Capability {
    pub path: Vec<String>,
    pub scope: Option<String>,
}

/// The built-in tree: a parent covers its children.
pub const TREE: &[&str] = &[
    "console",
    "filesystem",
    "filesystem.read",
    "filesystem.write",
    "network",
    "network.http",
    "network.socket",
    "environment",
    "time",
    "random",
    "process",
    "foreign",
];

/// Capabilities that take a scope argument (a path, a host, a variable or a
/// program name); the others take none.
pub fn takes_scope(path: &[String]) -> bool {
    matches!(
        path.first().map(String::as_str),
        Some("filesystem") | Some("network") | Some("environment") | Some("process")
    )
}

impl Capability {
    pub fn from_ast(capability: &ast::Capability) -> Capability {
        Capability {
            path: capability.path.iter().map(|n| n.text.clone()).collect(),
            scope: capability.scope.clone(),
        }
    }

    pub fn is_known(&self) -> bool {
        TREE.contains(&self.path.join(".").as_str())
    }

    pub fn spelling(&self) -> String {
        match &self.scope {
            Some(scope) => format!("{}(\"{}\")", self.path.join("."), scope),
            None => self.path.join("."),
        }
    }

    /// Whether this granted capability covers a needed one: the same or an
    /// ancestor path, and a scope that contains the needed scope. A needed
    /// capability without a scope is covered by a scoped grant only when
    /// `runtime_scoped` says the callee checks its actual path or host at run
    /// time, which the library primitives do.
    pub fn covers(&self, needed: &Capability, runtime_scoped: bool) -> bool {
        if needed.path.len() < self.path.len() || needed.path[..self.path.len()] != self.path[..] {
            return false;
        }
        match (&self.scope, &needed.scope) {
            (None, _) => true,
            (Some(_), None) => runtime_scoped,
            (Some(granted), Some(wanted)) => scope_contains(&self.path, granted, wanted),
        }
    }
}

/// A path prefix contains its sub-paths; a host, a variable or a program
/// name contains only itself.
fn scope_contains(path: &[String], granted: &str, wanted: &str) -> bool {
    if path.first().map(String::as_str) == Some("filesystem") {
        let granted = granted.trim_end_matches('/');
        wanted == granted || wanted.starts_with(&format!("{granted}/"))
    } else {
        granted == wanted
    }
}

/// Whether any granted capability covers the needed one.
pub fn covered(granted: &[Capability], needed: &Capability, runtime_scoped: bool) -> bool {
    granted
        .iter()
        .any(|capability| capability.covers(needed, runtime_scoped))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap(spelling: &str, scope: Option<&str>) -> Capability {
        Capability {
            path: spelling.split('.').map(str::to_string).collect(),
            scope: scope.map(str::to_string),
        }
    }

    #[test]
    fn parents_cover_children_and_scopes_contain_prefixes() {
        assert!(cap("filesystem", None).covers(&cap("filesystem.read", Some("data")), false));
        assert!(cap("filesystem.read", Some("data"))
            .covers(&cap("filesystem.read", Some("data/2024")), false));
        assert!(!cap("filesystem.read", Some("data"))
            .covers(&cap("filesystem.read", Some("database")), false));
        assert!(!cap("filesystem.read", Some("data"))
            .covers(&cap("filesystem.write", Some("data")), false));
        assert!(!cap("filesystem.read", Some("data")).covers(&cap("filesystem.read", None), false));
        assert!(cap("filesystem.read", Some("data")).covers(&cap("filesystem.read", None), true));
        assert!(cap("network", Some("api.example.com"))
            .covers(&cap("network.http", Some("api.example.com")), false));
        assert!(!cap("network.http", Some("a.example.com"))
            .covers(&cap("network.http", Some("b.example.com")), false));
        assert!(!cap("console", None).covers(&cap("time", None), false));
    }
}
