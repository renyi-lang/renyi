//! The grant a run executes under: the capabilities `main` or a test
//! declares, narrowed from the command line (`--deny`, `--allow-host`,
//! `--allow-read`, `--allow-write`), with the budget counters of decision
//! P2 (`at most COUNT per UNIT`, `--at-most`); and the effect one primitive
//! call exercises, which the scope check, the budgets and the recorder
//! share.

use std::collections::VecDeque;

use renyi_check::effects::{self, Capability, TREE};

use crate::compile::FunctionMeta;
use crate::value::{Native, Value};

/// `second`, `minute`, `hour` and `day` are sliding windows; `run` counts
/// the whole program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Second,
    Minute,
    Hour,
    Day,
    Run,
}

impl Unit {
    pub fn parse(text: &str) -> Option<Unit> {
        Some(match text {
            "second" => Unit::Second,
            "minute" => Unit::Minute,
            "hour" => Unit::Hour,
            "day" => Unit::Day,
            "run" => Unit::Run,
            _ => return None,
        })
    }

    pub fn spelling(self) -> &'static str {
        match self {
            Unit::Second => "second",
            Unit::Minute => "minute",
            Unit::Hour => "hour",
            Unit::Day => "day",
            Unit::Run => "run",
        }
    }

    fn window_ms(self) -> Option<i64> {
        match self {
            Unit::Second => Some(1_000),
            Unit::Minute => Some(60_000),
            Unit::Hour => Some(3_600_000),
            Unit::Day => Some(86_400_000),
            Unit::Run => None,
        }
    }
}

/// The counter of one budget: the calls made under its capability in the
/// current window, or in the whole run.
#[derive(Clone, Debug)]
pub struct Counter {
    /// The budgeted capability, its path and scope only.
    pub capability: Capability,
    pub limit: u64,
    pub unit: Unit,
    /// The times of the calls still inside the window, in milliseconds
    /// since the run began.
    stamps: VecDeque<i64>,
    count: u64,
}

impl Counter {
    /// The counter of a granted capability that carries a budget.
    pub fn from_grant(capability: &Capability) -> Option<Counter> {
        let (count, unit) = capability.budget.as_ref()?;
        Some(Counter {
            capability: Capability {
                path: capability.path.clone(),
                scope: capability.scope.clone(),
                budget: None,
                only_to: Vec::new(),
            },
            limit: count.parse().ok()?,
            unit: Unit::parse(unit)?,
            stamps: VecDeque::new(),
            count: 0,
        })
    }

    /// Whether one more call at `now` stays within the budget.
    pub fn fits(&mut self, now: i64) -> bool {
        match self.unit.window_ms() {
            None => self.count < self.limit,
            Some(window) => {
                while self.stamps.front().is_some_and(|&at| at <= now - window) {
                    self.stamps.pop_front();
                }
                (self.stamps.len() as u64) < self.limit
            }
        }
    }

    /// Count a call made at `now`.
    pub fn note(&mut self, now: i64) {
        match self.unit.window_ms() {
            None => self.count += 1,
            Some(_) => self.stamps.push_back(now),
        }
    }

    pub fn spelling(&self) -> String {
        format!(
            "{} at most {} per {}",
            self.capability.spelling(),
            self.limit,
            self.unit.spelling()
        )
    }
}

/// What the command line changes about a declared grant.
#[derive(Clone, Debug, Default)]
pub struct Narrowing {
    /// `--deny CAPABILITY`: removed from the grant; the program must not
    /// need it.
    pub deny: Vec<Capability>,
    /// `--allow-host HOST`, `--allow-read PATH`, `--allow-write PATH`: a
    /// scoped capability the grant is intersected with.
    pub allow: Vec<Capability>,
    /// `--at-most CAPABILITY=COUNT/UNIT`: budgets added to the declared
    /// ones, so that the command line can only tighten.
    pub budgets: Vec<Capability>,
}

/// The grant of one run: the capabilities a primitive call is checked
/// against, and the budget counters.
#[derive(Clone, Debug, Default)]
pub struct Grant {
    pub capabilities: Vec<Capability>,
    pub counters: Vec<Counter>,
}

/// The effective grant: the declared capabilities, their scopes normalised,
/// intersected with every `allow` and without every `deny`; the counters
/// come from the declared budgets and the added ones.
pub fn effective(declared: &[Capability], narrowing: &Narrowing) -> Grant {
    let mut capabilities: Vec<Capability> = declared.iter().map(normalised).collect();
    for allow in &narrowing.allow {
        let allow = normalised(allow);
        capabilities = capabilities
            .into_iter()
            .flat_map(|granted| narrow(granted, &allow))
            .collect();
    }
    for deny in &narrowing.deny {
        capabilities = capabilities
            .into_iter()
            .flat_map(|granted| remove(granted, deny))
            .collect();
    }
    let mut counters: Vec<Counter> = declared
        .iter()
        .map(normalised)
        .filter_map(|c| Counter::from_grant(&c))
        .collect();
    counters.extend(
        narrowing
            .budgets
            .iter()
            .map(normalised)
            .filter_map(|c| Counter::from_grant(&c)),
    );
    Grant {
        capabilities,
        counters,
    }
}

fn normalised(capability: &Capability) -> Capability {
    let mut capability = capability.clone();
    if capability.path.first().map(String::as_str) == Some("filesystem") {
        capability.scope = capability.scope.as_deref().map(normalize_path);
    }
    capability
}

fn on_one_line(a: &[String], b: &[String]) -> bool {
    let shorter = a.len().min(b.len());
    a[..shorter] == b[..shorter]
}

/// The children of a capability in the built-in tree, each with the
/// parent's scope and budget.
fn children(capability: &Capability) -> Vec<Capability> {
    TREE.iter()
        .filter_map(|entry| {
            let path: Vec<String> = entry.split('.').map(str::to_string).collect();
            (path.len() == capability.path.len() + 1
                && path[..capability.path.len()] == capability.path[..])
                .then(|| Capability {
                    path,
                    ..capability.clone()
                })
        })
        .collect()
}

/// A granted capability under an `allow`: one on another line passes
/// through, one on the allow's line takes the intersection of the scopes,
/// and an ancestor of the allow's path is split into its children first,
/// so that `--allow-read data` narrows `filesystem.read` and leaves
/// `filesystem.write` as it was.
fn narrow(granted: Capability, allow: &Capability) -> Vec<Capability> {
    if !on_one_line(&granted.path, &allow.path) {
        return vec![granted];
    }
    if granted.path.len() < allow.path.len() {
        return children(&granted)
            .into_iter()
            .flat_map(|child| narrow(child, allow))
            .collect();
    }
    match intersect(
        &granted.path,
        granted.scope.as_deref(),
        allow.scope.as_deref(),
    ) {
        Some(scope) => vec![Capability { scope, ..granted }],
        None => Vec::new(),
    }
}

/// A granted capability without a denied one: covered by the denial, it
/// goes; an ancestor of the denied path is split into its children first.
fn remove(granted: Capability, deny: &Capability) -> Vec<Capability> {
    if !on_one_line(&granted.path, &deny.path) {
        return vec![granted];
    }
    if granted.path.len() < deny.path.len() {
        return children(&granted)
            .into_iter()
            .flat_map(|child| remove(child, deny))
            .collect();
    }
    Vec::new()
}

/// The scope both allow: `None` when they exclude each other.
fn intersect(path: &[String], a: Option<&str>, b: Option<&str>) -> Option<Option<String>> {
    match (a, b) {
        (None, other) | (other, None) => Some(other.map(str::to_string)),
        (Some(a), Some(b)) => {
            if effects::scope_contains(path, a, b) {
                Some(Some(b.to_string()))
            } else if effects::scope_contains(path, b, a) {
                Some(Some(a.to_string()))
            } else {
                None
            }
        }
    }
}

/// The capability spelled with its budget: `network.http("host") at most
/// 60 per minute`.
pub fn spell(capability: &Capability) -> String {
    match &capability.budget {
        Some((count, unit)) => format!("{} at most {count} per {unit}", capability.spelling()),
        None => capability.spelling(),
    }
}

/// A capability from its spelling: `filesystem.read("data")`, `console`,
/// optionally followed by `at most COUNT per UNIT`.
pub fn parse_capability(text: &str) -> Result<Capability, String> {
    let text = text.trim();
    let (head, budget) = match text.split_once(" at most ") {
        Some((head, rest)) => {
            let (count, unit) = rest
                .split_once(" per ")
                .ok_or_else(|| format!("`{text}`: a budget is `at most COUNT per UNIT`"))?;
            let count = count.trim();
            let unit = unit.trim();
            if count.parse::<u64>().is_err() {
                return Err(format!(
                    "`{text}`: the count `{count}` is not a whole number"
                ));
            }
            if Unit::parse(unit).is_none() {
                return Err(format!(
                    "`{text}`: the unit `{unit}` is not second, minute, hour, day or run"
                ));
            }
            (head.trim(), Some((count.to_string(), unit.to_string())))
        }
        None => (text, None),
    };
    let (path, scope) = match head.split_once('(') {
        Some((path, rest)) => {
            let inner = rest
                .strip_suffix(')')
                .ok_or_else(|| format!("`{text}`: the scope is not closed"))?;
            let scope = inner
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .ok_or_else(|| format!("`{text}`: the scope must be quoted"))?;
            (path, Some(scope.to_string()))
        }
        None => (head, None),
    };
    if path.is_empty()
        || !path
            .split('.')
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_lowercase()))
    {
        return Err(format!("`{text}`: not a capability"));
    }
    let capability = Capability {
        path: path.split('.').map(str::to_string).collect(),
        scope,
        budget,
        only_to: Vec::new(),
    };
    if !capability.is_known() {
        return Err(format!("`{path}` is not a built-in capability"));
    }
    Ok(capability)
}

/// The capability a primitive call exercises: the primitive's declared need
/// with the scope its argument names (a path, the host of a URL, a
/// variable); `None` for a pure primitive.
pub fn effect_of(meta: &FunctionMeta, args: &[Value]) -> Option<Capability> {
    let need = meta.needs.first()?;
    let mut effect = Capability {
        path: need.path.clone(),
        scope: None,
        budget: None,
        only_to: Vec::new(),
    };
    if effects::takes_scope(&effect.path) {
        // the argument of the named type, by position
        let typed = |wanted: &str| {
            meta.param_types
                .iter()
                .zip(args)
                .find(|(ty, _)| ty.as_str() == wanted)
                .and_then(|(_, value)| value.as_text())
        };
        effect.scope = match effect.path.first().map(String::as_str) {
            Some("network") => typed("Url").and_then(host_of),
            Some("filesystem") => match args.first() {
                // a database call acts on the file its connection opened
                Some(Value::Native(native)) => match &**native {
                    Native::Connection { path, .. } => Some(normalize_path(path)),
                    _ => None,
                },
                _ => typed("Path").map(normalize_path),
            },
            Some("environment") if meta.name == "get" => {
                args.first().and_then(Value::as_text).map(str::to_string)
            }
            Some("process") => typed("Text").map(str::to_string),
            _ => None,
        };
    }
    Some(effect)
}

/// The host of a URL, lower-cased, without user information or port.
pub fn host_of(url: &str) -> Option<String> {
    let rest = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = authority.rsplit('@').next().unwrap_or(authority);
    let host = host.split(':').next().unwrap_or(host);
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

/// A path with one separator, no `.` segments and `..` resolved where it
/// can be, so that a prefix test means containment.
pub fn normalize_path(text: &str) -> String {
    let unified = text.replace('\\', "/");
    let absolute = unified.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for segment in unified.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|last| *last != "..") {
                    parts.pop();
                } else {
                    parts.push("..");
                }
            }
            other => parts.push(other),
        }
    }
    let joined = parts.join("/");
    if absolute {
        format!("/{joined}")
    } else {
        joined
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap(text: &str) -> Capability {
        parse_capability(text).unwrap()
    }

    #[test]
    fn allows_narrow_scopes_and_split_parents() {
        let grant = effective(
            &[cap("console"), cap("filesystem")],
            &Narrowing {
                allow: vec![cap("filesystem.read(\"data\")")],
                ..Narrowing::default()
            },
        );
        let spelled: Vec<String> = grant.capabilities.iter().map(spell).collect();
        assert_eq!(
            spelled,
            ["console", "filesystem.read(\"data\")", "filesystem.write"]
        );
        // a scope outside the declared one leaves nothing
        let grant = effective(
            &[cap("network.http(\"a.example\")")],
            &Narrowing {
                allow: vec![cap("network.http(\"b.example\")")],
                ..Narrowing::default()
            },
        );
        assert!(grant.capabilities.is_empty());
    }

    #[test]
    fn denies_remove_and_budgets_count() {
        let grant = effective(
            &[cap("network at most 2 per run")],
            &Narrowing {
                deny: vec![cap("network.socket")],
                ..Narrowing::default()
            },
        );
        let spelled: Vec<String> = grant.capabilities.iter().map(spell).collect();
        assert_eq!(spelled, ["network.http at most 2 per run"]);
        let mut counter = grant.counters.into_iter().next().unwrap();
        assert!(counter.fits(0));
        counter.note(0);
        counter.note(1);
        assert!(!counter.fits(2));
        let mut window = Counter::from_grant(&cap("filesystem.read at most 1 per second")).unwrap();
        assert!(window.fits(0));
        window.note(0);
        assert!(!window.fits(500));
        assert!(window.fits(1_000));
    }

    #[test]
    fn paths_and_hosts_normalise() {
        assert_eq!(normalize_path("./data//x/../sales.csv"), "data/sales.csv");
        assert_eq!(normalize_path("..\\secrets"), "../secrets");
        assert_eq!(normalize_path("/var/log/"), "/var/log");
        assert_eq!(
            host_of("https://User@API.Example.com:8443/v1?x=1").as_deref(),
            Some("api.example.com")
        );
        assert_eq!(host_of("not a url").as_deref(), Some("not a url"));
        assert!(parse_capability("filesystem.read(data)").is_err());
        assert!(parse_capability("magic").is_err());
        assert_eq!(
            spell(&cap("network.http(\"h\") at most 60 per minute")),
            "network.http(\"h\") at most 60 per minute"
        );
    }
}
