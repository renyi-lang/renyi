//! The semantic diff of two maps (`renyi index --diff`; design document 05,
//! section 6; decision O4): what changed in each definition, the
//! definitions each change reaches through the fan-in edges, and the
//! version bump the changes force (decision G1).
//!
//! A definition is matched across the maps by its qualified name; a name
//! that disappears while its content hash reappears under another name is
//! a rename (decision D5). A content hash changes when the definition's own
//! text changes and when anything it depends on changes, so the diff reads
//! the hash of the definition's own text (`text_hash`, with every reference
//! blanked) and its edges to tell the origin of a change from its reach:
//! the callers of a changed body are listed as what the change reaches, not
//! as changed themselves.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use renyi_syntax::json::Json;

use crate::{Definition, Header, Index, Kind};

/// One change to a definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    Added,
    Removed,
    /// The same content hash under another name.
    Renamed {
        from: String,
        was_public: bool,
    },
    /// The head and its signature clauses, which name the declared effects
    /// and failure types.
    Signature {
        old: String,
        new: String,
    },
    Visibility {
        public: bool,
    },
    /// The effects the definition reaches through everything it calls.
    Effects {
        widened: Vec<String>,
        narrowed: Vec<String>,
    },
    /// The failure types it reaches.
    Failures {
        added: Vec<String>,
        removed: Vec<String>,
    },
    /// Its own text or its edges changed with the signature unchanged.
    Body,
}

/// Every change to one definition and what the changes reach.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// `module.name`.
    pub name: String,
    pub kind: Kind,
    pub public: bool,
    pub changes: Vec<Change>,
    /// Every definition that refers to this one, transitively, in the map
    /// where it exists after the change (the old map for a removed one).
    pub reaches: Vec<String>,
}

/// The version bump decision G1 derives from the diff.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Bump {
    None,
    Minor,
    Major,
}

impl Bump {
    pub fn name(self) -> &'static str {
        match self {
            Bump::None => "none",
            Bump::Minor => "minor",
            Bump::Major => "major",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diff {
    pub old: Header,
    pub new: Header,
    /// In name order.
    pub entries: Vec<Entry>,
    pub bump: Bump,
}

/// Compare the new map with the old one.
pub fn diff(old: &Index, new: &Index) -> Diff {
    let old_by: BTreeMap<String, &Definition> = old
        .definitions
        .iter()
        .map(|definition| (definition.qualified(), definition))
        .collect();
    let new_by: BTreeMap<String, &Definition> = new
        .definitions
        .iter()
        .map(|definition| (definition.qualified(), definition))
        .collect();
    let removed: Vec<&String> = old_by
        .keys()
        .filter(|name| !new_by.contains_key(*name))
        .collect();
    let added: Vec<&String> = new_by
        .keys()
        .filter(|name| !old_by.contains_key(*name))
        .collect();
    // a name gone and its hash back under another name: a rename (D5)
    let mut by_id: HashMap<&str, &String> = removed
        .iter()
        .map(|name| (old_by[*name].id.as_str(), *name))
        .collect();
    let mut renames: HashMap<String, String> = HashMap::new();
    for name in &added {
        if let Some(from) = by_id.remove(new_by[*name].id.as_str()) {
            renames.insert(from.clone(), (*name).clone());
        }
    }
    let old_callers = callers(old);
    let new_callers = callers(new);
    let mut entries = Vec::new();
    for (name, definition) in &new_by {
        let changes = match old_by.get(name) {
            Some(before) => compare(before, definition, &renames),
            None => match renames.iter().find(|(_, to)| *to == name) {
                Some((from, _)) => vec![Change::Renamed {
                    from: from.clone(),
                    was_public: old_by[from].public,
                }],
                None => vec![Change::Added],
            },
        };
        if changes.is_empty() {
            continue;
        }
        entries.push(Entry {
            name: name.clone(),
            kind: definition.kind,
            public: definition.public,
            changes,
            reaches: reach(&new_callers, name),
        });
    }
    for name in removed {
        if renames.contains_key(name) {
            continue;
        }
        let definition = old_by[name];
        entries.push(Entry {
            name: name.clone(),
            kind: definition.kind,
            public: definition.public,
            changes: vec![Change::Removed],
            reaches: reach(&old_callers, name),
        });
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    let bump = bump_of(&entries);
    Diff {
        old: old.header.clone(),
        new: new.header.clone(),
        entries,
        bump,
    }
}

/// The changes between two records of one definition. The edges are
/// compared with the old names translated through the renames, so that a
/// renamed callee does not read as a change to its callers.
fn compare(old: &Definition, new: &Definition, renames: &HashMap<String, String>) -> Vec<Change> {
    let mut changes = Vec::new();
    if old.signature != new.signature {
        changes.push(Change::Signature {
            old: old.signature.clone(),
            new: new.signature.clone(),
        });
    }
    if old.public != new.public {
        changes.push(Change::Visibility { public: new.public });
    }
    let (widened, narrowed) = difference(&old.effects_transitive, &new.effects_transitive);
    if !widened.is_empty() || !narrowed.is_empty() {
        changes.push(Change::Effects { widened, narrowed });
    }
    let (added, removed) = difference(&old.fails_transitive, &new.fails_transitive);
    if !added.is_empty() || !removed.is_empty() {
        changes.push(Change::Failures { added, removed });
    }
    if old.signature == new.signature && own_change(old, new, renames) {
        changes.push(Change::Body);
    }
    changes
}

/// Whether the definition's own text or edges changed; a map without text
/// hashes (an older toolchain's) falls back to the content hash, which also
/// changes when a dependency does.
fn own_change(old: &Definition, new: &Definition, renames: &HashMap<String, String>) -> bool {
    if old.text_hash.is_empty() || new.text_hash.is_empty() {
        return old.id != new.id;
    }
    if old.text_hash != new.text_hash {
        return true;
    }
    let translate = |name: &String| renames.get(name).cloned().unwrap_or_else(|| name.clone());
    let old_edges: BTreeSet<String> = old.calls.iter().chain(&old.uses).map(translate).collect();
    let new_edges: BTreeSet<String> = new.calls.iter().chain(&new.uses).cloned().collect();
    old_edges != new_edges
}

/// `(in new only, in old only)`, each sorted.
fn difference(old: &[String], new: &[String]) -> (Vec<String>, Vec<String>) {
    let old: BTreeSet<&String> = old.iter().collect();
    let new: BTreeSet<&String> = new.iter().collect();
    (
        new.difference(&old).map(|s| (*s).clone()).collect(),
        old.difference(&new).map(|s| (*s).clone()).collect(),
    )
}

/// Who refers to whom: callee to callers, from the calls and uses.
fn callers(index: &Index) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for definition in &index.definitions {
        let from = definition.qualified();
        for target in definition.calls.iter().chain(&definition.uses) {
            if *target != from {
                map.entry(target.clone()).or_default().push(from.clone());
            }
        }
    }
    map
}

/// Every definition that reaches `name` through the callers, sorted.
fn reach(callers: &HashMap<String, Vec<String>>, name: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut stack = vec![name.to_string()];
    while let Some(current) = stack.pop() {
        for caller in callers.get(&current).into_iter().flatten() {
            if caller != name && seen.insert(caller.clone()) {
                stack.push(caller.clone());
            }
        }
    }
    seen.into_iter().collect()
}

/// Decision G1: a public signature removed or changed is a major bump, one
/// added a minor bump.
fn bump_of(entries: &[Entry]) -> Bump {
    let mut bump = Bump::None;
    for entry in entries {
        for change in &entry.changes {
            let level = match change {
                Change::Removed | Change::Signature { .. } if entry.public => Bump::Major,
                Change::Visibility { public: false } => Bump::Major,
                Change::Renamed {
                    was_public: true, ..
                } => Bump::Major,
                Change::Added if entry.public => Bump::Minor,
                Change::Visibility { public: true } => Bump::Minor,
                Change::Renamed {
                    was_public: false, ..
                } if entry.public => Bump::Minor,
                _ => Bump::None,
            };
            bump = bump.max(level);
        }
    }
    bump
}

// --------------------------------------------------------------- rendering

/// One line per changed definition and the bump.
pub fn render_diff(diff: &Diff) -> String {
    let mut out = format!(
        "compared {} at {} with {} at {}: ",
        diff.old.project, diff.old.revision, diff.new.project, diff.new.revision
    );
    match diff.entries.len() {
        0 => out.push_str("no definition changed\n"),
        1 => out.push_str("1 definition changed\n"),
        count => out.push_str(&format!("{count} definitions changed\n")),
    }
    for entry in &diff.entries {
        out.push_str(&format!(
            "{} ({}{}): {}",
            entry.name,
            if entry.public { "public " } else { "" },
            entry.kind.name(),
            entry
                .changes
                .iter()
                .map(change_text)
                .collect::<Vec<_>>()
                .join("; ")
        ));
        if !entry.reaches.is_empty() {
            out.push_str(&format!("; reaches {}", entry.reaches.join(", ")));
        }
        out.push('\n');
    }
    out.push_str(&format!("version bump: {}\n", diff.bump.name()));
    out
}

fn change_text(change: &Change) -> String {
    match change {
        Change::Added => "added".to_string(),
        Change::Removed => "removed".to_string(),
        Change::Renamed { from, .. } => format!("renamed from {from}"),
        Change::Signature { old, new } => format!("signature `{old}` -> `{new}`"),
        Change::Visibility { public: true } => "made public".to_string(),
        Change::Visibility { public: false } => "made private".to_string(),
        Change::Effects { widened, narrowed } => {
            let mut parts = Vec::new();
            if !widened.is_empty() {
                parts.push(format!("effects widened: {}", widened.join(", ")));
            }
            if !narrowed.is_empty() {
                parts.push(format!("effects narrowed: {}", narrowed.join(", ")));
            }
            parts.join("; ")
        }
        Change::Failures { added, removed } => {
            let mut parts = Vec::new();
            if !added.is_empty() {
                parts.push(format!("failures added: {}", added.join(", ")));
            }
            if !removed.is_empty() {
                parts.push(format!("failures removed: {}", removed.join(", ")));
            }
            parts.join("; ")
        }
        Change::Body => "body changed".to_string(),
    }
}

/// The diff as one JSON document.
pub fn diff_json(diff: &Diff) -> String {
    Json::Object(vec![
        ("old", header_json(&diff.old)),
        ("new", header_json(&diff.new)),
        (
            "changes",
            Json::Array(diff.entries.iter().map(entry_json).collect()),
        ),
        ("bump", Json::String(diff.bump.name().to_string())),
    ])
    .render()
}

fn header_json(header: &Header) -> Json {
    Json::Object(vec![
        ("project", Json::String(header.project.clone())),
        ("revision", Json::String(header.revision.clone())),
        ("toolchain", Json::String(header.toolchain.clone())),
    ])
}

fn entry_json(entry: &Entry) -> Json {
    Json::Object(vec![
        ("name", Json::String(entry.name.clone())),
        ("kind", Json::String(entry.kind.name().to_string())),
        ("public", Json::Bool(entry.public)),
        (
            "changes",
            Json::Array(entry.changes.iter().map(change_json).collect()),
        ),
        ("reaches", strings(&entry.reaches)),
    ])
}

fn change_json(change: &Change) -> Json {
    let kind = |name: &str| ("change", Json::String(name.to_string()));
    Json::Object(match change {
        Change::Added => vec![kind("added")],
        Change::Removed => vec![kind("removed")],
        Change::Renamed { from, was_public } => vec![
            kind("renamed"),
            ("from", Json::String(from.clone())),
            ("was_public", Json::Bool(*was_public)),
        ],
        Change::Signature { old, new } => vec![
            kind("signature"),
            ("old", Json::String(old.clone())),
            ("new", Json::String(new.clone())),
        ],
        Change::Visibility { public } => vec![kind("visibility"), ("public", Json::Bool(*public))],
        Change::Effects { widened, narrowed } => vec![
            kind("effects"),
            ("widened", strings(widened)),
            ("narrowed", strings(narrowed)),
        ],
        Change::Failures { added, removed } => vec![
            kind("failures"),
            ("added", strings(added)),
            ("removed", strings(removed)),
        ],
        Change::Body => vec![kind("body")],
    })
}

fn strings(values: &[String]) -> Json {
    Json::Array(
        values
            .iter()
            .map(|value| Json::String(value.clone()))
            .collect(),
    )
}
