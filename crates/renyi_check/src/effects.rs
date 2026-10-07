//! Capabilities and their coverage (syntax sketch section 11, decision J11).

use renyi_syntax::ast;

/// `filesystem.read("data")`: a path in the capability tree with an optional
/// literal scope.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Capability {
    pub path: Vec<String>,
    pub scope: Option<String>,
    /// `at most COUNT per UNIT` on a grant (decision P2); not part of coverage.
    pub budget: Option<(String, String)>,
    /// `only to` sinks on a grant (decision P3); not part of coverage.
    pub only_to: Vec<(Vec<String>, Option<String>)>,
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

/// The capability of the tree that no library function needs yet:
/// `foreign` waits for the foreign function interface of milestone M4
/// (decision V6); a `needs` clause may not name it until then. `process`
/// has `std.process` since decision AE1.
pub fn unavailable(path: &[String]) -> bool {
    path.first().map(String::as_str) == Some("foreign")
}

pub fn unavailable_message(path: &[String]) -> String {
    format!(
        "`{}` is not available until the foreign function interface (milestone M4)",
        path.join(".")
    )
}

impl Capability {
    pub fn from_ast(capability: &ast::Capability) -> Capability {
        Capability {
            path: capability.path.iter().map(|n| n.text.clone()).collect(),
            scope: capability.scope.clone(),
            budget: capability
                .budget
                .as_ref()
                .map(|b| (b.count.clone(), b.per.text.clone())),
            only_to: capability
                .only_to
                .iter()
                .map(|sink| {
                    (
                        sink.path.iter().map(|n| n.text.clone()).collect(),
                        sink.scope.clone(),
                    )
                })
                .collect(),
        }
    }

    /// Whether the grant carries a budget or a guard, which belong on `main`
    /// and on tests only.
    pub fn has_grant_clauses(&self) -> bool {
        self.budget.is_some() || !self.only_to.is_empty()
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
/// name contains only itself. The runtime uses the same rule for the grant
/// it narrows and the paths it checks.
pub fn scope_contains(path: &[String], granted: &str, wanted: &str) -> bool {
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
            budget: None,
            only_to: Vec::new(),
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
