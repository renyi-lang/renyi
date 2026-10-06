//! Content hashes of definitions (decision D5; design document 05,
//! section 3).
//!
//! The hash of a definition is SHA-256 over its canonical text after four
//! changes: its own name token becomes `_`; every name token that refers to
//! another project definition becomes that definition's hash; the hashes of
//! every project definition it depends on in any other way (a variant
//! constructed, a pattern matched) are appended; and the qualified names of
//! the library definitions it uses are appended with the library version.
//! Renaming a definition therefore keeps its hash and the hashes of
//! everything that refers to it; changing a body changes its hash and the
//! hashes of everything that depends on it.
//!
//! Definitions that depend on each other form one strongly connected
//! component and are hashed together: references inside the component
//! become the member's position, the component's material is hashed once,
//! and a member's hash is the component hash combined with its position.

use std::collections::{BTreeSet, HashMap};

use sha2::{Digest, Sha256};

/// What the hash of one definition is computed from. Offsets are byte
/// offsets into `text`; every reference span is one name token, so the
/// spans do not overlap.
#[derive(Clone, Debug, Default)]
pub struct Input {
    /// The canonical text of the definition.
    pub text: String,
    /// The span of the definition's own name, if it has one.
    pub own_name: Option<(usize, usize)>,
    /// `(start, end, target)`: a name token that refers to the project
    /// definition at index `target` of the inputs.
    pub references: Vec<(usize, usize, usize)>,
    /// Every project definition this one depends on, by index, whether or
    /// not a name token refers to it.
    pub dependencies: Vec<usize>,
    /// The qualified names of the library definitions it uses.
    pub library: Vec<String>,
}

/// The hash of one definition's own text alone: its canonical text with its
/// own name and every reference to a project definition blanked, and no
/// dependency or library material. It changes only when the definition's
/// own text changes, so the semantic diff (`diff.rs`) uses it to tell a
/// changed body from a changed dependency, which both change the content
/// hash.
pub fn own_text_hash(input: &Input) -> String {
    let mut edits: Vec<(usize, usize)> = input
        .references
        .iter()
        .map(|&(start, end, _)| (start, end))
        .chain(input.own_name)
        .collect();
    edits.sort_by(|a, b| b.cmp(a));
    edits.dedup();
    let mut text = input.text.clone();
    for (start, end) in edits {
        text.replace_range(start..end, "_");
    }
    format!("sha256:{:x}", Sha256::digest(text.as_bytes()))
}

/// The hash of every input, as `sha256:` followed by 64 hex digits.
pub fn hashes(inputs: &[Input], library_version: &str) -> Vec<String> {
    let edges: Vec<Vec<usize>> = inputs
        .iter()
        .map(|input| {
            let mut targets: Vec<usize> = input
                .references
                .iter()
                .map(|&(_, _, target)| target)
                .chain(input.dependencies.iter().copied())
                .collect();
            targets.sort_unstable();
            targets.dedup();
            targets
        })
        .collect();
    let mut out = vec![String::new(); inputs.len()];
    // components come dependencies first, so every hash a member needs exists
    for component in strongly_connected_components(&edges) {
        let position: HashMap<usize, usize> = component
            .iter()
            .enumerate()
            .map(|(position, &member)| (member, position))
            .collect();
        let cyclic = component.len() > 1 || edges[component[0]].contains(&component[0]);
        let mut material = String::new();
        for (position_in_component, &member) in component.iter().enumerate() {
            let input = &inputs[member];
            let mut edits: Vec<(usize, usize, String)> = Vec::new();
            if let Some((start, end)) = input.own_name {
                edits.push((start, end, "_".to_string()));
            }
            for &(start, end, target) in &input.references {
                let replacement = match position.get(&target) {
                    Some(inside) => format!("#cycle:{inside}"),
                    None => format!("#{}", out[target]),
                };
                edits.push((start, end, replacement));
            }
            edits.sort_by_key(|edit| std::cmp::Reverse((edit.0, edit.1)));
            edits.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
            let mut text = input.text.clone();
            for (start, end, replacement) in edits {
                text.replace_range(start..end, &replacement);
            }
            if position_in_component > 0 {
                material.push_str("\n--\n");
            }
            material.push_str(&text);
        }
        let mut dependencies = BTreeSet::new();
        let mut library = BTreeSet::new();
        for &member in &component {
            for &target in &edges[member] {
                if !position.contains_key(&target) {
                    dependencies.insert(out[target].clone());
                }
            }
            library.extend(inputs[member].library.iter().cloned());
        }
        material.push_str("\ndeps:");
        for dependency in dependencies {
            material.push(' ');
            material.push_str(&dependency);
        }
        material.push_str("\nlibrary:");
        for name in library {
            material.push(' ');
            material.push_str(&name);
            material.push('@');
            material.push_str(library_version);
        }
        let component_hash = format!("{:x}", Sha256::digest(material.as_bytes()));
        for (position_in_component, &member) in component.iter().enumerate() {
            out[member] = if cyclic {
                let member_material = format!("{component_hash}#{position_in_component}");
                format!("sha256:{:x}", Sha256::digest(member_material.as_bytes()))
            } else {
                format!("sha256:{component_hash}")
            };
        }
    }
    out
}

/// Tarjan's algorithm. Components come out in reverse topological order
/// of the condensation: a component after every component it reaches.
/// Members are sorted by index.
fn strongly_connected_components(edges: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut tarjan = Tarjan {
        edges,
        index: vec![None; edges.len()],
        low: vec![0; edges.len()],
        on_stack: vec![false; edges.len()],
        stack: Vec::new(),
        next: 0,
        out: Vec::new(),
    };
    for vertex in 0..edges.len() {
        if tarjan.index[vertex].is_none() {
            tarjan.visit(vertex);
        }
    }
    tarjan.out
}

struct Tarjan<'a> {
    edges: &'a [Vec<usize>],
    index: Vec<Option<usize>>,
    low: Vec<usize>,
    on_stack: Vec<bool>,
    stack: Vec<usize>,
    next: usize,
    out: Vec<Vec<usize>>,
}

impl Tarjan<'_> {
    fn visit(&mut self, vertex: usize) {
        self.index[vertex] = Some(self.next);
        self.low[vertex] = self.next;
        self.next += 1;
        self.stack.push(vertex);
        self.on_stack[vertex] = true;
        let edges = self.edges;
        for &target in &edges[vertex] {
            match self.index[target] {
                None => {
                    self.visit(target);
                    self.low[vertex] = self.low[vertex].min(self.low[target]);
                }
                Some(index) => {
                    if self.on_stack[target] {
                        self.low[vertex] = self.low[vertex].min(index);
                    }
                }
            }
        }
        if Some(self.low[vertex]) == self.index[vertex] {
            let mut component = Vec::new();
            loop {
                let member = self.stack.pop().expect("a vertex on the stack");
                self.on_stack[member] = false;
                component.push(member);
                if member == vertex {
                    break;
                }
            }
            component.sort_unstable();
            self.out.push(component);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(text: &str) -> Input {
        Input {
            text: text.to_string(),
            ..Input::default()
        }
    }

    #[test]
    fn a_rename_keeps_the_hash_and_a_body_change_propagates() {
        // `a` calls `b` by name; the name token of `b` inside `a` is a reference
        let mut a = input("function a() return b() end");
        a.own_name = Some((9, 10));
        a.references = vec![(20, 21, 1)];
        let mut b = input("function b() return 1 end");
        b.own_name = Some((9, 10));
        let before = hashes(&[a.clone(), b.clone()], "0");
        // rename b to helper everywhere
        let mut a2 = input("function a() return helper() end");
        a2.own_name = Some((9, 10));
        a2.references = vec![(20, 26, 1)];
        let mut b2 = input("function helper() return 1 end");
        b2.own_name = Some((9, 15));
        let renamed = hashes(&[a2, b2], "0");
        assert_eq!(before, renamed);
        // change the body of b: both hashes change
        let mut b3 = b.clone();
        b3.text = "function b() return 2 end".to_string();
        let changed = hashes(&[a, b3], "0");
        assert_ne!(changed[1], before[1]);
        assert_ne!(changed[0], before[0]);
    }

    #[test]
    fn mutually_recursive_definitions_hash_together_but_apart() {
        let mut even = input("function even(n) return odd(n) end");
        even.own_name = Some((9, 13));
        even.references = vec![(24, 27, 1)];
        let mut odd = input("function odd(n) return even(n) end");
        odd.own_name = Some((9, 12));
        odd.references = vec![(23, 27, 0)];
        let first = hashes(&[even.clone(), odd.clone()], "0");
        assert_ne!(first[0], first[1]);
        assert_eq!(first, hashes(&[even, odd], "0"));
        assert!(first.iter().all(|h| h.len() == "sha256:".len() + 64));
    }

    #[test]
    fn the_own_text_hash_ignores_names_and_dependencies() {
        let mut a = input("function a() return b() end");
        a.own_name = Some((9, 10));
        a.references = vec![(20, 21, 1)];
        let mut renamed = input("function alpha() return helper() end");
        renamed.own_name = Some((9, 14));
        renamed.references = vec![(24, 30, 1)];
        assert_eq!(own_text_hash(&a), own_text_hash(&renamed));
        let mut changed = a.clone();
        changed.text = "function a() return b() + 1 end".to_string();
        assert_ne!(own_text_hash(&a), own_text_hash(&changed));
        // a dependency's change is not the definition's own
        let mut with_dependency = a.clone();
        with_dependency.dependencies = vec![1];
        with_dependency.library = vec!["std.console.print".to_string()];
        assert_eq!(own_text_hash(&a), own_text_hash(&with_dependency));
    }

    #[test]
    fn components_come_dependencies_first() {
        let edges = vec![vec![1], vec![2], vec![1], vec![]];
        let components = strongly_connected_components(&edges);
        assert_eq!(components, vec![vec![1, 2], vec![0], vec![3]]);
    }
}
