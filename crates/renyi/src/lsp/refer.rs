//! References and rename of declarations (decision AW38, stage 35a): every
//! place a definition is named across the workspace (the bodies'
//! references the checker recorded, the signatures', the names imports
//! expose, the declaration itself), the family a method belongs to (an
//! ability's method and the method of every implementation of the
//! ability, renamed together), the rules a new name must pass, and the
//! project checked again with the rename applied.

use std::collections::BTreeMap;

use renyi_check::check::Target;
use renyi_check::world::TypeKindInfo;
use renyi_check::{
    check_project_in, AbilityId, BodyLocation, CheckedProject, FunctionId, Library, ModuleId, World,
};
use renyi_index::signature_references;
use renyi_syntax::ast::{Item, TypeKind};
use renyi_syntax::{lex, Diagnostic, SourceFile, Span, TokenKind};

use super::describe::{ability_item, function_item, position_of, type_item};

/// One place a definition is named: the module, the span of the name
/// there, and whether it is the declaration itself.
pub struct Occurrence {
    pub module: ModuleId,
    pub span: Span,
    pub declaration: bool,
}

/// The targets renamed together. A target is its own family, but an
/// ability's method and the method of every implementation of the ability
/// share a name: a call on a value of a concrete type resolves to the
/// implementation's function, a call on a value of a constrained type
/// parameter to the ability's method, and an implementation's methods are
/// matched to the ability's by name.
pub fn family(world: &World, target: &Target) -> Vec<Target> {
    let ability_method = match target {
        Target::AbilityMethod(ability, index) => Some((*ability, *index)),
        Target::Function(id) => implemented_method(world, *id),
        _ => None,
    };
    let Some((ability, index)) = ability_method else {
        return vec![target.clone()];
    };
    let name = &world.abilities[ability].methods[index].name;
    let mut members = vec![Target::AbilityMethod(ability, index)];
    for implementation in world.impls.iter().filter(|i| i.ability == ability) {
        members.extend(
            implementation
                .functions
                .iter()
                .copied()
                .filter(|&function| world.functions[function].name == *name)
                .map(Target::Function),
        );
    }
    members
}

/// The ability's method an implementation's function implements.
fn implemented_method(world: &World, id: FunctionId) -> Option<(AbilityId, usize)> {
    let info = &world.functions[id];
    if !matches!(info.body, BodyLocation::Implementation(..)) {
        return None;
    }
    let implementation = world
        .impls
        .iter()
        .find(|i| i.module == info.module && i.functions.contains(&id))?;
    let index = world.abilities[implementation.ability]
        .methods
        .iter()
        .position(|method| method.name == info.name)?;
    Some((implementation.ability, index))
}

/// The module a target is declared in.
pub fn target_module(world: &World, target: &Target) -> Option<ModuleId> {
    match target {
        Target::Function(id) => Some(world.functions[*id].module),
        Target::AbilityMethod(ability, _) | Target::Ability(ability) => {
            Some(world.abilities[*ability].module)
        }
        Target::Constant(module, _) => Some(*module),
        Target::Type(id) | Target::Variant(id, _) => Some(world.types[*id].module),
        Target::Number(_) | Target::Result(_) | Target::Typed(_) | Target::Otherwise { .. } => None,
    }
}

/// The name a target is declared under.
pub fn target_name(world: &World, target: &Target) -> String {
    match target {
        Target::Function(id) => world.functions[*id].name.clone(),
        Target::AbilityMethod(ability, index) => {
            world.abilities[*ability].methods[*index].name.clone()
        }
        Target::Constant(_, name) => name.clone(),
        Target::Type(id) => world.types[*id].name.clone(),
        Target::Variant(id, index) => match &world.types[*id].kind {
            TypeKindInfo::Sum(variants) => variants[*index].name.clone(),
            _ => world.types[*id].name.clone(),
        },
        Target::Ability(id) => world.abilities[*id].name.clone(),
        Target::Number(_) | Target::Result(_) | Target::Typed(_) | Target::Otherwise { .. } => {
            String::new()
        }
    }
}

/// Where a target is declared: its module and its name token.
pub fn declaration_of(world: &World, target: &Target) -> Option<(ModuleId, Span)> {
    match target {
        Target::Function(id) => {
            let (module, function) = function_item(world, *id)?;
            Some((module, function.name.span))
        }
        Target::AbilityMethod(ability, index) => {
            let (module, decl) = ability_item(world, *ability)?;
            Some((module, decl.functions.get(*index)?.name.span))
        }
        Target::Constant(module, name) => {
            let info = world.modules[*module].constants.get(name)?;
            match world.modules[*module].ast.items.get(info.item_index)? {
                Item::Constant(constant) => Some((*module, constant.name.span)),
                _ => None,
            }
        }
        Target::Type(id) => {
            let (module, def) = type_item(world, *id)?;
            Some((module, def.name.span))
        }
        Target::Variant(id, index) => {
            let (module, def) = type_item(world, *id)?;
            match &def.kind {
                TypeKind::Sum { variants, .. } => Some((module, variants.get(*index)?.name.span)),
                _ => None,
            }
        }
        Target::Ability(id) => {
            let (module, decl) = ability_item(world, *id)?;
            Some((module, decl.name.span))
        }
        Target::Number(_) | Target::Result(_) | Target::Typed(_) | Target::Otherwise { .. } => None,
    }
}

/// Every place a member of the family is named in the modules
/// `in_workspace` admits, sorted by module and position, the declarations
/// marked: the bodies' references, the signatures', the names imports
/// expose, and each member's own declaration.
pub fn occurrences(
    checked: &CheckedProject,
    in_workspace: &dyn Fn(ModuleId) -> bool,
    members: &[Target],
) -> Vec<Occurrence> {
    let world = &checked.world;
    let mut found: Vec<Occurrence> = checked
        .references
        .iter()
        .filter(|(module, _, reference)| {
            in_workspace(*module) && members.contains(&reference.target)
        })
        .map(|(module, _, reference)| Occurrence {
            module: *module,
            span: reference.span,
            declaration: false,
        })
        .collect();
    for (module, info) in world.modules.iter().enumerate() {
        if !in_workspace(module) {
            continue;
        }
        for (target, span) in signature_references(world, module) {
            if members.contains(&target) {
                found.push(Occurrence {
                    module,
                    span,
                    declaration: false,
                });
            }
        }
        for exposed in info.ast.imports.iter().flat_map(|import| &import.exposing) {
            let target = info
                .exposed_types
                .get(&exposed.text)
                .map(|&id| Target::Type(id))
                .or_else(|| {
                    info.exposed_abilities
                        .get(&exposed.text)
                        .map(|&id| Target::Ability(id))
                });
            if target.is_some_and(|target| members.contains(&target)) {
                found.push(Occurrence {
                    module,
                    span: exposed.span,
                    declaration: false,
                });
            }
        }
    }
    for member in members {
        if let Some((module, span)) = declaration_of(world, member) {
            if in_workspace(module) {
                found.push(Occurrence {
                    module,
                    span,
                    declaration: true,
                });
            }
        }
    }
    // a declaration first among equals, so that the dedup keeps it
    found.sort_by_key(|occurrence| {
        (
            occurrence.module,
            occurrence.span.start,
            !occurrence.declaration,
        )
    });
    found.dedup_by_key(|occurrence| {
        (
            occurrence.module,
            occurrence.span.start,
            occurrence.span.end,
        )
    });
    found
}

/// The spelling class of a name (reference section 1).
#[derive(Clone, Copy)]
pub enum Class {
    /// A function, a method or a constant: a lowercase word.
    Lower,
    /// A type, a variant or an ability: a capitalized word.
    Upper,
}

pub fn class_of(target: &Target) -> Class {
    match target {
        Target::Type(_) | Target::Variant(..) | Target::Ability(_) => Class::Upper,
        _ => Class::Lower,
    }
}

/// Whether a new name is one word of the class and not a reserved word
/// (reference section 17): the lexer decides, as it does for a program.
pub fn check_name(name: &str, class: Class) -> Result<(), String> {
    let lexed = lex(name);
    let one_word = lexed.diagnostics.is_empty()
        && lexed.tokens.len() == 2
        && lexed.tokens[0].span.start == 0
        && lexed.tokens[0].span.end == name.len();
    let kind = lexed.tokens.first().map(|token| &token.kind);
    match (kind, class) {
        (Some(TokenKind::Identifier), Class::Lower) if one_word => Ok(()),
        (Some(TokenKind::TypeName), Class::Upper) if one_word => Ok(()),
        (Some(TokenKind::Word(_)), _) if one_word => {
            Err(format!("`{name}` is a reserved word (reference section 17)"))
        }
        (_, Class::Lower) => Err(format!(
            "`{name}` is not a function, method or constant name: lowercase letters, digits and underscores, starting with a letter"
        )),
        (_, Class::Upper) => Err(format!(
            "`{name}` is not a type, variant or ability name: letters and digits, starting with a capital letter"
        )),
    }
}

/// The project checked again with the edits applied, against the
/// diagnostics it has now: the rename is refused when a file gains an
/// error of a code it did not have as many of before (a name already
/// declared, a binding that would shadow the new name), with the
/// checker's message. `before` gives a file's current diagnostics and
/// `display` its name for the message.
pub fn check_after(
    library: &Library,
    sources: &[SourceFile],
    edits: &BTreeMap<String, Vec<(Span, String)>>,
    before: &dyn Fn(&str) -> Vec<Diagnostic>,
    display: &dyn Fn(&str) -> String,
) -> Result<(), String> {
    let files: Vec<SourceFile> = sources
        .iter()
        .map(|source| match edits.get(&source.name) {
            Some(edits) => with_edits(source, edits),
            None => source.clone(),
        })
        .collect();
    let checked = check_project_in(library, &files, &[]);
    for module in &checked.modules {
        let file = &files[module.file];
        let mut before_counts: BTreeMap<&str, usize> = BTreeMap::new();
        let earlier = before(&file.name);
        for diagnostic in earlier.iter().filter(|d| d.is_error()) {
            *before_counts.entry(diagnostic.code).or_default() += 1;
        }
        let mut after_counts: BTreeMap<&str, usize> = BTreeMap::new();
        for diagnostic in module.diagnostics.iter().filter(|d| d.is_error()) {
            let count = after_counts.entry(diagnostic.code).or_default();
            *count += 1;
            if *count > before_counts.get(diagnostic.code).copied().unwrap_or(0) {
                let (line, _) = position_of(&file.text, diagnostic.span.start);
                let mut message = format!(
                    "the rename would make an error in `{}` at line {}: {}",
                    display(&file.name),
                    line + 1,
                    diagnostic.message
                );
                if let Some(fix) = &diagnostic.fix {
                    message.push_str(&format!(" (fix: {fix})"));
                }
                return Err(message);
            }
        }
    }
    Ok(())
}

/// A file with the edits applied, its tags kept.
fn with_edits(source: &SourceFile, edits: &[(Span, String)]) -> SourceFile {
    let mut edits: Vec<&(Span, String)> = edits.iter().collect();
    edits.sort_by_key(|(span, _)| std::cmp::Reverse(span.start));
    let mut text = source.text.clone();
    for (span, new_text) in edits {
        text.replace_range(span.start..span.end, new_text);
    }
    let mut file = SourceFile::new(source.name.clone(), text);
    file.package = source.package.clone();
    file.foreign = source.foreign.clone();
    file.python = source.python.clone();
    file
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_name_is_one_word_of_its_class_and_not_reserved() {
        assert!(check_name("surface_2", Class::Lower).is_ok());
        assert!(check_name("Surface2", Class::Upper).is_ok());
        assert!(check_name("Surface", Class::Lower)
            .unwrap_err()
            .contains("not a function"));
        assert!(check_name("surface", Class::Upper)
            .unwrap_err()
            .contains("not a type"));
        assert!(check_name("end", Class::Lower)
            .unwrap_err()
            .contains("reserved word"));
        assert!(check_name("is not", Class::Lower)
            .unwrap_err()
            .contains("reserved word"));
        assert!(check_name("two words", Class::Lower).is_err());
        assert!(check_name("", Class::Lower).is_err());
        assert!(check_name("a.b", Class::Lower).is_err());
    }

    #[test]
    fn edits_are_applied_from_the_end_and_the_tags_kept() {
        let source = SourceFile::new("a.ry", "let x be f(x)".to_string());
        let edited = with_edits(
            &source,
            &[
                (Span::new(4, 5), "yy".to_string()),
                (Span::new(11, 12), "yy".to_string()),
            ],
        );
        assert_eq!(edited.text, "let yy be f(yy)");
        assert_eq!(edited.name, "a.ry");
        assert_eq!(edited.package, None);
    }
}
