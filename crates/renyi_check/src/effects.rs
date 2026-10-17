//! Capabilities and their coverage (syntax sketch section 11, decision J11).

use renyi_syntax::ast;

/// `filesystem.read("data")`: a path in the capability tree with an optional
/// literal scope.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Capability {
    pub path: Vec<String>,
    pub scope: Option<String>,
    /// `at most COUNT per UNIT` on a grant (decision P2); not part of coverage.
    pub budget: Option<Budget>,
    /// `only to` sinks on a grant (decision P3); not part of coverage.
    pub only_to: Vec<(Vec<String>, Option<String>)>,
}

/// `at most LIMIT per UNIT` on a grant (decision P2), or `at most LIMIT of
/// PARAMETER per UNIT` on an amount (decision AY1): the sum of that
/// parameter's absolute values over the calls, the limit then a whole
/// number or a decimal.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Budget {
    pub limit: String,
    /// The parameter whose values the budget sums; `None` for a count of
    /// calls.
    pub parameter: Option<String>,
    pub unit: String,
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
    "python",
    "screen",
    "memory",
];

/// The capabilities that take a budget (decisions P2 and AY1): `network`,
/// `process` and `filesystem` with their children, and `python`.
pub fn budgeted_kind(path: &[String]) -> bool {
    matches!(
        path.first().map(String::as_str),
        Some("network") | Some("process") | Some("filesystem") | Some("python")
    )
}

/// Whether a text is the limit of a budget on an amount (decision AY1): a
/// whole number or a decimal, as the literals are spelled.
pub fn is_amount_literal(text: &str) -> bool {
    let (whole, fraction) = match text.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (text, None),
    };
    let digits =
        |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit() || b == b'_');
    digits(whole) && fraction.is_none_or(digits)
}

/// Whether a budget's unit is one of memory (decision AW64).
pub fn is_memory_unit(unit: &str) -> bool {
    ast::MEMORY_UNITS.contains(&unit)
}

/// `memory at most COUNT UNIT` in bytes; `None` when the unit is not one
/// of memory or the product does not fit in 64 bits.
pub fn memory_bytes(count: &str, unit: &str) -> Option<u64> {
    let factor: u64 = match unit {
        "bytes" => 1,
        "kilobytes" => 1 << 10,
        "megabytes" => 1 << 20,
        "gigabytes" => 1 << 30,
        _ => return None,
    };
    count.parse::<u64>().ok()?.checked_mul(factor)
}

/// Capabilities that take a scope argument (a path, a host, a variable, a
/// program name or a Python package); the others take none.
pub fn takes_scope(path: &[String]) -> bool {
    matches!(
        path.first().map(String::as_str),
        Some("filesystem")
            | Some("network")
            | Some("environment")
            | Some("process")
            | Some("python")
    )
}

impl Capability {
    pub fn from_ast(capability: &ast::Capability) -> Capability {
        Capability {
            path: capability.path.iter().map(|n| n.text.clone()).collect(),
            scope: capability.scope.clone(),
            budget: capability.budget.as_ref().map(|b| Budget {
                limit: b.count.clone(),
                parameter: b.parameter.as_ref().map(|name| name.text.clone()),
                unit: b.per.text.clone(),
            }),
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
        // the memory budget is not an effect: every grant covers it
        // (decision AW64)
        if needed.path == ["memory"] {
            return true;
        }
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

/// A path prefix contains its sub-paths; a host, a variable, a program
/// name or a package contains only itself. The runtime uses the same rule
/// for the grant it narrows and the paths it checks. The scope `.` (which
/// the runtime normalizes to the empty text) is the current directory: it
/// contains every relative path that does not climb out of it, and no
/// absolute one.
pub fn scope_contains(path: &[String], granted: &str, wanted: &str) -> bool {
    if path.first().map(String::as_str) == Some("filesystem") {
        let granted = granted.trim_end_matches('/');
        if granted.is_empty() || granted == "." {
            return within_current_directory(wanted);
        }
        wanted == granted || wanted.starts_with(&format!("{granted}/"))
    } else {
        granted == wanted
    }
}

/// Whether a path, normalized or as written, stays under the current
/// directory: not absolute (no leading separator, no drive letter) and
/// not climbing out through `..`.
fn within_current_directory(wanted: &str) -> bool {
    let absolute = wanted.starts_with('/') || wanted.as_bytes().get(1) == Some(&b':');
    !absolute && wanted != ".." && !wanted.starts_with("../")
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
    fn the_memory_budget_is_covered_by_every_grant_and_counts_in_bytes() {
        // decision AW64
        assert!(cap("console", None).covers(&cap("memory", None), false));
        assert!(!cap("memory", None).covers(&cap("console", None), false));
        assert_eq!(memory_bytes("256", "megabytes"), Some(256 << 20));
        assert_eq!(memory_bytes("4096", "bytes"), Some(4096));
        assert_eq!(memory_bytes("1", "gigabytes"), Some(1 << 30));
        assert_eq!(memory_bytes("60", "minute"), None);
        assert_eq!(memory_bytes("x", "megabytes"), None);
        assert!(is_memory_unit("kilobytes") && !is_memory_unit("run"));
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

    #[test]
    fn the_current_directory_contains_relative_paths_only() {
        for scope in [".", "./", ""] {
            assert!(cap("filesystem.read", Some(scope))
                .covers(&cap("filesystem.read", Some("data/sales.csv")), false));
            assert!(cap("filesystem.read", Some(scope))
                .covers(&cap("filesystem.read", Some("notes.txt")), false));
            assert!(!cap("filesystem.read", Some(scope))
                .covers(&cap("filesystem.read", Some("../secrets")), false));
            assert!(!cap("filesystem.read", Some(scope))
                .covers(&cap("filesystem.read", Some("..")), false));
            assert!(!cap("filesystem.read", Some(scope))
                .covers(&cap("filesystem.read", Some("/etc/passwd")), false));
            assert!(!cap("filesystem.read", Some(scope))
                .covers(&cap("filesystem.read", Some("C:/secrets")), false));
        }
    }
}
