//! The edges of a definition: what it calls and uses, resolved to project
//! definitions or library names; the effects and failures it reaches; and
//! what its hash is computed from.

use std::collections::{BTreeSet, HashMap, HashSet};

use renyi_check::effects::Capability;
use renyi_check::{FunctionId, Target, World};
use renyi_syntax::{SourceFile, Span};

use crate::drafts::{ability_qualified, function_qualified, type_qualified, Draft, Key};
use crate::hash;

/// The edges of one definition, resolved.
#[derive(Default)]
pub struct Edges {
    pub calls: BTreeSet<String>,
    pub uses: BTreeSet<String>,
    /// Project definitions referenced, by index, this one excluded.
    pub out: BTreeSet<usize>,
    pub library_calls: BTreeSet<String>,
    /// Every library definition referenced, for the hash.
    pub library: BTreeSet<String>,
    /// Name tokens that refer to project definitions, for the hash.
    pub splices: Vec<(Span, usize)>,
    /// The functions called or passed by name, for the effect walk.
    pub callees: Vec<FunctionId>,
}

/// Turn a draft's references into edges, names and hash splices.
pub fn resolve_edges(
    world: &World,
    draft: &Draft,
    index: usize,
    key_index: &HashMap<Key, usize>,
) -> Edges {
    let mut edges = Edges::default();
    let project = |key: Key, edges: &mut Edges| -> Option<usize> {
        let target = *key_index.get(&key)?;
        if target != index {
            edges.out.insert(target);
        }
        Some(target)
    };
    for (reference, span) in &draft.refs {
        match reference {
            Target::Function(id) => {
                edges.callees.push(*id);
                let qualified = function_qualified(world, *id);
                edges.calls.insert(qualified.clone());
                if world.functions[*id].is_library {
                    edges.library_calls.insert(qualified.clone());
                    edges.library.insert(qualified);
                } else if let Some(target) = project(Key::Function(*id), &mut edges) {
                    edges.splices.push((*span, target));
                }
            }
            Target::AbilityMethod(ability, method) => {
                let info = &world.abilities[*ability];
                let method_name = &info.methods[*method].name;
                let qualified = format!("{}.{method_name}", ability_qualified(world, *ability));
                edges.calls.insert(qualified.clone());
                // any implementation may answer the call
                for implementation in world.impls.iter().filter(|i| i.ability == *ability) {
                    for &function in &implementation.functions {
                        if world.functions[function].name == *method_name {
                            edges.callees.push(function);
                        }
                    }
                }
                if world.modules[info.module].is_library {
                    edges.library_calls.insert(qualified.clone());
                    edges.library.insert(qualified);
                } else {
                    project(Key::Ability(*ability), &mut edges);
                }
            }
            Target::Constant(module, name) => {
                let qualified = format!("{}.{name}", world.modules[*module].name);
                edges.uses.insert(qualified);
                if let Some(target) = project(Key::Constant(*module, name.clone()), &mut edges) {
                    edges.splices.push((*span, target));
                }
            }
            Target::Type(id) | Target::Variant(id, _) => {
                let qualified = type_qualified(world, *id);
                edges.uses.insert(qualified.clone());
                if world.modules[world.types[*id].module].is_library {
                    edges.library.insert(qualified);
                } else if let Some(target) = project(Key::Type(*id), &mut edges) {
                    // a variant's name token stays: the type is a dependency only
                    if matches!(reference, Target::Type(_)) {
                        edges.splices.push((*span, target));
                    }
                }
            }
            Target::Ability(id) => {
                let qualified = ability_qualified(world, *id);
                edges.uses.insert(qualified.clone());
                if world.modules[world.abilities[*id].module].is_library {
                    edges.library.insert(qualified);
                } else if let Some(target) = project(Key::Ability(*id), &mut edges) {
                    edges.splices.push((*span, target));
                }
            }
            // a literal's number type, a context-decided result type and the
            // kind of an `otherwise` are for the VM, not edges
            Target::Number(_) | Target::Result(_) | Target::Otherwise { .. } => {}
        }
    }
    edges.callees.sort_unstable();
    edges.callees.dedup();
    edges
}

/// The effects and failure types reachable from a definition: its own,
/// then those of every function its body (or its methods) reaches.
pub fn transitive(
    world: &World,
    callees: &HashMap<FunctionId, Vec<FunctionId>>,
    draft: &Draft,
    edges: &Edges,
) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut effects: BTreeSet<String> = draft.needs.iter().map(Capability::spelling).collect();
    let mut fails: BTreeSet<String> = draft.fails.iter().map(|ty| world.show(ty)).collect();
    let mut seen: HashSet<FunctionId> = HashSet::new();
    let mut stack: Vec<FunctionId> = edges.callees.clone();
    stack.extend(draft.starts.iter().copied());
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        let function = &world.functions[id];
        effects.extend(function.needs.iter().map(Capability::spelling));
        fails.extend(function.fails.iter().map(|ty| world.show(ty)));
        if let Some(next) = callees.get(&id) {
            stack.extend(next.iter().copied());
        }
    }
    (effects, fails)
}

/// What the hash of a definition is computed from: its canonical text,
/// with the spans made relative to it.
pub fn hash_input(text: &SourceFile, draft: &Draft, edges: &Edges) -> hash::Input {
    let base = draft.span.start;
    let inside = |span: Span| span.start >= draft.span.start && span.end <= draft.span.end;
    let mut dependencies: Vec<usize> = edges.out.iter().copied().collect();
    let references: Vec<(usize, usize, usize)> = edges
        .splices
        .iter()
        .filter(|(span, _)| inside(*span))
        .map(|(span, target)| (span.start - base, span.end - base, *target))
        .collect();
    for &(_, _, target) in &references {
        if !dependencies.contains(&target) {
            dependencies.push(target);
        }
    }
    hash::Input {
        text: text.slice(draft.span).to_string(),
        own_name: draft
            .name_span
            .filter(|span| inside(*span))
            .map(|span| (span.start - base, span.end - base)),
        references,
        dependencies,
        library: edges.library.iter().cloned().collect(),
    }
}
