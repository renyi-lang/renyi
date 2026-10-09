//! What the language server says about a position: the reference or the
//! declaration under it, described by its signature and its purpose and
//! located at its declared name; a file's items as document symbols; and
//! positions as the protocol counts them (zero-based lines, UTF-16 code
//! units).

use renyi_check::check::Target;
use renyi_check::{BodyLocation, CheckedProject, FunctionId, ModuleId, World};
use renyi_index::{function_signature, type_signature};
use renyi_syntax::ast::{AbilityDecl, Constant, Function, Item, Module, TypeDef, TypeKind};
use renyi_syntax::{type_text, SourceFile, Span};

/// A declaration described: what hover shows and where definition goes.
pub struct Described {
    pub signature: String,
    pub purpose: Option<String>,
    /// The module declaring it.
    pub module: ModuleId,
    /// The declared name, where definition goes.
    pub name: Span,
}

/// One item of a file for the outline: its name, what it is (a symbol
/// kind of the protocol), its whole span, its name's span and its parts.
pub struct Symbol {
    pub name: String,
    pub detail: Option<String>,
    pub kind: u8,
    pub range: Span,
    pub selection: Span,
    pub children: Vec<Symbol>,
}

const KIND_METHOD: u8 = 6;
const KIND_FIELD: u8 = 8;
const KIND_ENUM: u8 = 10;
const KIND_INTERFACE: u8 = 11;
const KIND_FUNCTION: u8 = 12;
const KIND_CONSTANT: u8 = 14;
const KIND_ENUM_MEMBER: u8 = 22;
const KIND_STRUCT: u8 = 23;
const KIND_EVENT: u8 = 24;
const KIND_OBJECT: u8 = 19;

// ------------------------------------------------------------- positions

/// A byte offset as the protocol counts it: the zero-based line and the
/// UTF-16 code units before it on that line.
pub fn position_of(text: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(text.len());
    let before = &text[..offset];
    let line = before.matches('\n').count();
    let start = before.rfind('\n').map_or(0, |index| index + 1);
    (line, before[start..].encode_utf16().count())
}

/// The byte offset of a position the protocol gives; none past the last
/// line. A character past the end of its line is the end of the line.
pub fn offset_of(text: &str, line: usize, character: usize) -> Option<usize> {
    let mut start = 0;
    for _ in 0..line {
        start = text[start..].find('\n').map(|index| start + index + 1)?;
    }
    let mut units = 0;
    let mut offset = start;
    for ch in text[start..].chars() {
        if ch == '\n' || units >= character {
            break;
        }
        units += ch.len_utf16();
        offset += ch.len_utf8();
    }
    Some(offset)
}

fn contains(span: Span, offset: usize) -> bool {
    span.start <= offset && offset < span.end.max(span.start + 1)
}

// ------------------------------------------------------------ describing

/// What is at a byte offset of a module's file: the reference there (the
/// innermost, when they nest), else the declaration whose name is there;
/// with the span of what was found. `source_of` gives a module's file,
/// for the signature of a type.
pub fn what_is_at<'a>(
    checked: &'a CheckedProject,
    source_of: &dyn Fn(ModuleId) -> Option<&'a SourceFile>,
    module: ModuleId,
    offset: usize,
) -> Option<(Span, Described)> {
    let world = &checked.world;
    let reference = checked
        .references
        .iter()
        .filter(|(owner, _, reference)| *owner == module && contains(reference.span, offset))
        .min_by_key(|(_, _, reference)| reference.span.len())
        .map(|(_, _, reference)| reference);
    if let Some(reference) = reference {
        if let Some(described) = describe_target(world, source_of, &reference.target) {
            return Some((reference.span, described));
        }
    }
    declaration_at(world, source_of, module, offset)
}

fn describe_target<'a>(
    world: &'a World,
    source_of: &dyn Fn(ModuleId) -> Option<&'a SourceFile>,
    target: &Target,
) -> Option<Described> {
    match target {
        Target::Function(id) => {
            let (module, function) = function_item(world, *id)?;
            Some(describe_function(module, function))
        }
        Target::AbilityMethod(ability, index) => {
            let (module, decl) = ability_item(world, *ability)?;
            Some(describe_function(module, decl.functions.get(*index)?))
        }
        Target::Constant(module, name) => {
            let info = world.modules[*module].constants.get(name)?;
            match world.modules[*module].ast.items.get(info.item_index)? {
                Item::Constant(constant) => Some(describe_constant(*module, constant)),
                _ => None,
            }
        }
        Target::Type(id) => {
            let (module, def) = type_item(world, *id)?;
            Some(describe_type(module, def, source_of(module), None))
        }
        Target::Variant(id, index) => {
            let (module, def) = type_item(world, *id)?;
            Some(describe_type(module, def, source_of(module), Some(*index)))
        }
        Target::Ability(id) => {
            let (module, decl) = ability_item(world, *id)?;
            Some(describe_ability(module, decl))
        }
        Target::Number(_) | Target::Result(_) | Target::Typed(_) | Target::Otherwise { .. } => None,
    }
}

/// The declaration whose name is at the offset: a function, a method, a
/// type, a variant, an ability, an ability's method or a constant.
fn declaration_at<'a>(
    world: &'a World,
    source_of: &dyn Fn(ModuleId) -> Option<&'a SourceFile>,
    module: ModuleId,
    offset: usize,
) -> Option<(Span, Described)> {
    for item in &world.modules[module].ast.items {
        match item {
            Item::Function(function) => {
                if contains(function.name.span, offset) {
                    return Some((function.name.span, describe_function(module, function)));
                }
            }
            Item::Implementation(implementation) => {
                for function in &implementation.functions {
                    if contains(function.name.span, offset) {
                        return Some((function.name.span, describe_function(module, function)));
                    }
                }
            }
            Item::Type(def) => {
                if contains(def.name.span, offset) {
                    return Some((
                        def.name.span,
                        describe_type(module, def, source_of(module), None),
                    ));
                }
                if let TypeKind::Sum { variants, .. } = &def.kind {
                    for (index, variant) in variants.iter().enumerate() {
                        if contains(variant.name.span, offset) {
                            return Some((
                                variant.name.span,
                                describe_type(module, def, source_of(module), Some(index)),
                            ));
                        }
                    }
                }
            }
            Item::Ability(decl) => {
                if contains(decl.name.span, offset) {
                    return Some((decl.name.span, describe_ability(module, decl)));
                }
                for function in &decl.functions {
                    if contains(function.name.span, offset) {
                        return Some((function.name.span, describe_function(module, function)));
                    }
                }
            }
            Item::Constant(constant) => {
                if contains(constant.name.span, offset) {
                    return Some((constant.name.span, describe_constant(module, constant)));
                }
            }
            Item::Test(_) => {}
        }
    }
    None
}

/// The syntax of a function the world declared, by where its body lives.
fn function_item(world: &World, id: FunctionId) -> Option<(ModuleId, &Function)> {
    let info = &world.functions[id];
    let items = &world.modules[info.module].ast.items;
    let function = match info.body {
        BodyLocation::Item(index) => match items.get(index)? {
            Item::Function(function) => function,
            _ => return None,
        },
        BodyLocation::Implementation(index, method) => match items.get(index)? {
            Item::Implementation(implementation) => implementation.functions.get(method)?,
            _ => return None,
        },
        _ => return None,
    };
    Some((info.module, function))
}

fn type_item(world: &World, id: renyi_check::TypeId) -> Option<(ModuleId, &TypeDef)> {
    let info = &world.types[id];
    let def = world.modules[info.module]
        .ast
        .items
        .iter()
        .find_map(|item| match item {
            Item::Type(def) if def.span == info.span => Some(def),
            _ => None,
        })?;
    Some((info.module, def))
}

fn ability_item(world: &World, id: renyi_check::AbilityId) -> Option<(ModuleId, &AbilityDecl)> {
    let info = &world.abilities[id];
    let decl = world.modules[info.module]
        .ast
        .items
        .iter()
        .find_map(|item| match item {
            Item::Ability(decl) if decl.span == info.span => Some(decl),
            _ => None,
        })?;
    Some((info.module, decl))
}

fn describe_function(module: ModuleId, function: &Function) -> Described {
    Described {
        signature: function_signature(function),
        purpose: function.docs.purpose.clone(),
        module,
        name: function.name.span,
    }
}

fn describe_constant(module: ModuleId, constant: &Constant) -> Described {
    Described {
        signature: format!("{}: {}", constant.name.text, type_text(&constant.ty)),
        purpose: constant.docs.purpose.clone(),
        module,
        name: constant.name.span,
    }
}

/// A type, or one of its variants: the type's signature either way, the
/// variant's name as the place to go.
fn describe_type(
    module: ModuleId,
    def: &TypeDef,
    source: Option<&SourceFile>,
    variant: Option<usize>,
) -> Described {
    let signature = match source {
        Some(source) => type_signature(def, source),
        None => def.name.text.clone(),
    };
    let name = match (variant, &def.kind) {
        (Some(index), TypeKind::Sum { variants, .. }) => variants
            .get(index)
            .map_or(def.name.span, |variant| variant.name.span),
        _ => def.name.span,
    };
    Described {
        signature,
        purpose: def.docs.purpose.clone(),
        module,
        name,
    }
}

fn describe_ability(module: ModuleId, decl: &AbilityDecl) -> Described {
    let mut signature = format!("ability {}", decl.name.text);
    if !decl.type_params.is_empty() {
        let params: Vec<&str> = decl.type_params.iter().map(|p| p.text.as_str()).collect();
        signature.push_str(&format!(" of {}", params.join(", ")));
    }
    Described {
        signature,
        purpose: decl.docs.purpose.clone(),
        module,
        name: decl.name.span,
    }
}

// --------------------------------------------------------------- symbols

/// A file's items for the outline, in source order: functions, types
/// with their fields or variants, abilities with their methods,
/// implementations with their methods, constants and tests.
pub fn symbols_of(module: &Module, file: &SourceFile) -> Vec<Symbol> {
    module
        .items
        .iter()
        .map(|item| match item {
            Item::Function(function) => function_symbol(function, KIND_FUNCTION),
            Item::Type(def) => {
                let (kind, children) = match &def.kind {
                    TypeKind::Record { fields, .. } => (
                        KIND_STRUCT,
                        fields
                            .iter()
                            .map(|field| Symbol {
                                name: field.name.text.clone(),
                                detail: Some(type_text(&field.ty)),
                                kind: KIND_FIELD,
                                range: field.span,
                                selection: field.name.span,
                                children: Vec::new(),
                            })
                            .collect(),
                    ),
                    TypeKind::Sum { variants, .. } => (
                        KIND_ENUM,
                        variants
                            .iter()
                            .map(|variant| Symbol {
                                name: variant.name.text.clone(),
                                detail: None,
                                kind: KIND_ENUM_MEMBER,
                                range: variant.span,
                                selection: variant.name.span,
                                children: Vec::new(),
                            })
                            .collect(),
                    ),
                    TypeKind::Subtype { .. } => (KIND_STRUCT, Vec::new()),
                };
                Symbol {
                    name: def.name.text.clone(),
                    detail: Some(type_signature(def, file)),
                    kind,
                    range: def.span,
                    selection: def.name.span,
                    children,
                }
            }
            Item::Ability(decl) => Symbol {
                name: decl.name.text.clone(),
                detail: None,
                kind: KIND_INTERFACE,
                range: decl.span,
                selection: decl.name.span,
                children: decl
                    .functions
                    .iter()
                    .map(|function| function_symbol(function, KIND_METHOD))
                    .collect(),
            },
            Item::Implementation(implementation) => Symbol {
                name: format!(
                    "{} for {}",
                    type_text(&implementation.ability),
                    type_text(&implementation.target)
                ),
                detail: None,
                kind: KIND_OBJECT,
                range: implementation.span,
                selection: implementation.ability.span(),
                children: implementation
                    .functions
                    .iter()
                    .map(|function| function_symbol(function, KIND_METHOD))
                    .collect(),
            },
            Item::Constant(constant) => Symbol {
                name: constant.name.text.clone(),
                detail: Some(type_text(&constant.ty)),
                kind: KIND_CONSTANT,
                range: constant.span,
                selection: constant.name.span,
                children: Vec::new(),
            },
            Item::Test(test) => Symbol {
                name: format!("test {:?}", test.name),
                detail: None,
                kind: KIND_EVENT,
                range: test.span,
                selection: test.span,
                children: Vec::new(),
            },
        })
        .collect()
}

fn function_symbol(function: &Function, kind: u8) -> Symbol {
    Symbol {
        name: function.name.text.clone(),
        detail: Some(function_signature(function)),
        kind,
        range: function.span,
        selection: function.name.span,
        children: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_count_utf16_units_and_round_trip() {
        let text = "ab\n😀c\n";
        assert_eq!(position_of(text, 0), (0, 0));
        assert_eq!(position_of(text, 2), (0, 2));
        assert_eq!(position_of(text, 3), (1, 0));
        // the emoji is four bytes and two UTF-16 units
        assert_eq!(position_of(text, 7), (1, 2));
        assert_eq!(offset_of(text, 1, 2), Some(7));
        assert_eq!(offset_of(text, 1, 3), Some(8));
        assert_eq!(offset_of(text, 1, 99), Some(8), "clamped to the line's end");
        assert_eq!(offset_of(text, 2, 0), Some(9), "the empty last line");
        assert_eq!(offset_of(text, 3, 0), None);
    }
}
