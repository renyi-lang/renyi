//! Completion, signature help and workspace symbols (decision AW44, stage
//! 35c). The context of a position is read from the tokens of its line
//! before it: an `import` path, a `needs` clause, a name after a dot (an
//! import's namespace, or a value whose type the checker recorded ending
//! at the dot), else a bare name. The candidates come from the checked
//! world: at a bare position the bindings declared before the position in
//! the enclosing item (the checker records every declaration), the
//! module's functions, constants and types, the imports' namespaces and
//! exposed names, the prelude's functions that are not methods, and the
//! words that start a statement, in that order; after a dot the fields
//! and the methods of the type, the abilities' included. A call being
//! written is found by the innermost parenthesis still open on the line,
//! its callee resolved as a name is, and its signature spelled as hover
//! spells it, one parameter per label. A line the parser refuses (a dot
//! without a member, a call not closed, a bare name) keeps the whole file
//! out of the world, so the position's line is repaired for the request
//! (`repair`): a placeholder where an operand is missing, the brackets
//! closed, `ignore` before a bare expression, else the line blanked; a
//! receiver the repair took away is then found by its name.

use std::collections::BTreeSet;

use renyi_check::check::Target;
use renyi_check::effects::TREE;
use renyi_check::types::Ty;
use renyi_check::world::{head_type, TypeKindInfo};
use renyi_check::{AbilityId, CheckedProject, FunctionId, ModuleId, TypeId, World};
use renyi_index::function_signature;
use renyi_syntax::ast::{Field, Item, Param, TypeKind};
use renyi_syntax::{lex, parse, type_text, Diagnostic, SourceFile, Span, Token, TokenKind, Word};

use super::describe::{ability_item, describe_target, function_item, symbols_of, type_item};

/// The completion item kinds of the protocol.
const ITEM_METHOD: u8 = 2;
const ITEM_FUNCTION: u8 = 3;
const ITEM_FIELD: u8 = 5;
const ITEM_VARIABLE: u8 = 6;
const ITEM_CLASS: u8 = 7;
const ITEM_INTERFACE: u8 = 8;
const ITEM_MODULE: u8 = 9;
const ITEM_VALUE: u8 = 12;
const ITEM_KEYWORD: u8 = 14;
const ITEM_CONSTANT: u8 = 21;

/// The operand a repaired line gets where one is missing: two letters,
/// since the lexer refuses a single-letter name.
const PLACEHOLDER: &str = "xx";

/// The words that start a statement (the grammar's `Statement`).
const STATEMENT_WORDS: &[&str] = &[
    "let",
    "change",
    "if",
    "match",
    "for each",
    "repeat until",
    "run concurrently",
    "return",
    "fail",
    "crash with",
    "break",
    "continue",
    "ignore",
    "check",
];

/// One completion item: its label, its kind, what hover would show (the
/// signature as the detail, the purpose as the documentation), the group
/// the editor sorts by first, and the range it replaces when that is not
/// the word at the cursor (an import path).
pub struct Candidate {
    pub label: String,
    pub kind: u8,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub group: u8,
    pub edit: Option<Span>,
}

/// What the text before a position asks for.
#[derive(Debug, PartialEq)]
pub enum Context {
    /// After `import`: where the path starts and the path typed so far.
    Import { start: usize, typed: String },
    /// In a `needs` clause: a capability.
    Needs,
    /// After a dot: the one identifier before it, when the receiver is a
    /// name alone (an import's namespace, or a binding), and the dot's
    /// offset.
    Dotted { name: Option<String>, dot: usize },
    /// A name on its own.
    Bare,
}

/// The call being written at a position: its callee and the parameter
/// the cursor is at (the commas before it).
pub struct Call {
    pub callee: Callee,
    pub active: usize,
}

pub enum Callee {
    /// `name(`: a function of the module or of the prelude.
    Bare(String),
    /// `one.name(`: a function of the import `one`, or a method of the
    /// binding `one`.
    Qualified {
        first: String,
        dot: usize,
        name: String,
    },
    /// `expression.name(`: a method of the receiver ending at the dot.
    Method { dot: usize, name: String },
    /// `Name(`: a record or a variant built.
    Construct(String),
}

/// A signature for signature help: its label as hover spells it, its
/// purpose, and one label per parameter the caller writes (`self` left
/// out), each a part of the label.
pub struct Signature {
    pub label: String,
    pub documentation: Option<String>,
    pub parameters: Vec<String>,
}

/// A definition found for `workspace/symbol`: its name, its kind (a
/// symbol kind of the protocol), the module or the item it belongs to,
/// its module and the span of its name.
pub struct Found {
    pub name: String,
    pub kind: u8,
    pub container: String,
    pub module: ModuleId,
    pub span: Span,
}

// ---------------------------------------------------------------- tokens

/// The tokens of the position's line before the position, their spans in
/// the whole text; a prefix that ends inside a text literal is read from
/// the hole being written.
fn line_tokens(text: &str, offset: usize) -> Vec<Token> {
    let start = text[..offset].rfind('\n').map_or(0, |index| index + 1);
    let tokens = tokens_from(text, start, offset);
    if let Some(last) = tokens.last() {
        if last.kind == TokenKind::Error {
            if let Some(brace) = text[last.span.start..offset].rfind('{') {
                return tokens_from(text, last.span.start + brace + 1, offset);
            }
        }
    }
    tokens
}

/// The tokens of a part of the text, their spans moved to the whole.
fn tokens_from(text: &str, start: usize, end: usize) -> Vec<Token> {
    lex(&text[start..end])
        .tokens
        .into_iter()
        .filter(|token| {
            !matches!(
                token.kind,
                TokenKind::Eof | TokenKind::Newline | TokenKind::Comment
            )
        })
        .map(|token| {
            Token::new(
                token.kind,
                Span::new(token.span.start + start, token.span.end + start),
            )
        })
        .collect()
}

/// What the position asks for, from the tokens of its line before it.
pub fn context_at(text: &str, offset: usize) -> Context {
    let tokens = line_tokens(text, offset);
    if tokens
        .first()
        .is_some_and(|token| token.kind == TokenKind::Word(Word::Import))
    {
        let start = tokens.get(1).map_or(offset, |token| token.span.start);
        return Context::Import {
            start,
            typed: text[start..offset].to_string(),
        };
    }
    if in_needs_clause(&tokens) {
        return Context::Needs;
    }
    if let Some(dot) = dot_at_end(&tokens) {
        let name = match dot.checked_sub(1).map(|index| &tokens[index]) {
            Some(token) if token.kind == TokenKind::Identifier && !after_dot(&tokens, dot - 1) => {
                Some(text[token.span.start..token.span.end].to_string())
            }
            _ => None,
        };
        return Context::Dotted {
            name,
            dot: tokens[dot].span.start,
        };
    }
    Context::Bare
}

/// Whether the last reserved word of the line is `needs`, followed by
/// names alone.
fn in_needs_clause(tokens: &[Token]) -> bool {
    let Some(position) = tokens
        .iter()
        .rposition(|token| matches!(token.kind, TokenKind::Word(_)))
    else {
        return false;
    };
    tokens[position].kind == TokenKind::Word(Word::Needs)
        && tokens[position + 1..].iter().all(|token| {
            matches!(
                token.kind,
                TokenKind::Identifier | TokenKind::Member | TokenKind::Dot | TokenKind::Comma
            )
        })
}

/// The index of the dot the line ends with, a member after it or not.
fn dot_at_end(tokens: &[Token]) -> Option<usize> {
    let last = tokens.len().checked_sub(1)?;
    match tokens[last].kind {
        TokenKind::Dot => Some(last),
        TokenKind::Member if last >= 1 && tokens[last - 1].kind == TokenKind::Dot => Some(last - 1),
        _ => None,
    }
}

/// Whether the token at the index follows a dot.
fn after_dot(tokens: &[Token], index: usize) -> bool {
    index >= 1 && tokens[index - 1].kind == TokenKind::Dot
}

/// The call the position is inside, by the innermost parenthesis still
/// open on its line.
pub fn call_at(text: &str, offset: usize) -> Option<Call> {
    let tokens = line_tokens(text, offset);
    let mut depth = 0usize;
    let mut commas = 0;
    let mut open = None;
    for (index, token) in tokens.iter().enumerate().rev() {
        match token.kind {
            TokenKind::RightParen | TokenKind::RightBracket | TokenKind::RightBrace => depth += 1,
            TokenKind::LeftParen => {
                if depth == 0 {
                    open = Some(index);
                    break;
                }
                depth -= 1;
            }
            TokenKind::LeftBracket | TokenKind::LeftBrace => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
            }
            TokenKind::Comma if depth == 0 => commas += 1,
            _ => {}
        }
    }
    let before = &tokens[..open?];
    let last = before.len().checked_sub(1)?;
    let word = |index: usize| text[before[index].span.start..before[index].span.end].to_string();
    let callee = match before[last].kind {
        TokenKind::TypeName if !after_dot(before, last) => Callee::Construct(word(last)),
        TokenKind::Identifier if !after_dot(before, last) => Callee::Bare(word(last)),
        TokenKind::Member if after_dot(before, last) => {
            let dot = before[last - 1].span.start;
            match last.checked_sub(2) {
                Some(first)
                    if before[first].kind == TokenKind::Identifier && !after_dot(before, first) =>
                {
                    Callee::Qualified {
                        first: word(first),
                        dot,
                        name: word(last),
                    }
                }
                _ => Callee::Method {
                    dot,
                    name: word(last),
                },
            }
        }
        _ => return None,
    };
    Some(Call {
        callee,
        active: commas,
    })
}

// ---------------------------------------------------------------- repair

/// Where an offset of the editor's text lies in the text the world read:
/// the same, when the file parsed as the editor has it; moved by what the
/// repair put before it on its line; the line's start, when the repair
/// blanked the line.
#[derive(Clone, Copy)]
pub struct Mapping {
    line_start: usize,
    shift: usize,
    blanked: bool,
}

impl Mapping {
    pub fn same() -> Mapping {
        Mapping {
            line_start: usize::MAX,
            shift: 0,
            blanked: false,
        }
    }

    pub fn at(&self, offset: usize) -> usize {
        if offset < self.line_start {
            offset
        } else if self.blanked {
            self.line_start
        } else {
            offset + self.shift
        }
    }
}

/// The text with the position's line repaired, and the mapping of the
/// editor's offsets into it.
pub struct Repaired {
    pub text: String,
    pub mapping: Mapping,
}

/// The position's line repaired so that the file parses: first completed
/// (a placeholder where an operand is missing, the brackets closed,
/// `ignore` before a bare expression), else blanked. None when neither
/// parses, or when the line is blank.
pub fn repair(text: &str, offset: usize) -> Option<Repaired> {
    let line_start = text[..offset].rfind('\n').map_or(0, |index| index + 1);
    let line_end = text[offset..]
        .find('\n')
        .map_or(text.len(), |index| offset + index);
    let prefix = &text[line_start..offset];
    let suffix = &text[offset..line_end];
    if prefix.trim().is_empty() && suffix.trim().is_empty() {
        return None;
    }
    let before = tokens_from(text, line_start, offset);
    let after = tokens_from(text, offset, line_end);
    let indent = &prefix[..prefix.len() - prefix.trim_start().len()];
    let bare = before.first().or(after.first()).is_some_and(|token| {
        matches!(
            token.kind,
            TokenKind::Identifier
                | TokenKind::TypeName
                | TokenKind::Integer
                | TokenKind::Decimal
                | TokenKind::Text { .. }
                | TokenKind::RawText(_)
                | TokenKind::LeftParen
                | TokenKind::LeftBracket
                | TokenKind::Minus
        )
    });
    let operand_missing = before.last().is_some_and(|token| {
        matches!(
            token.kind,
            TokenKind::Dot
                | TokenKind::Comma
                | TokenKind::LeftParen
                | TokenKind::LeftBracket
                | TokenKind::Colon
                | TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Word(_)
        )
    });
    let mut line = String::from(indent);
    let shift = if bare {
        line.push_str("ignore ");
        "ignore ".len()
    } else {
        0
    };
    line.push_str(prefix.trim_start());
    let operand_follows = suffix
        .trim_start()
        .starts_with(|c: char| c.is_alphanumeric() || c == '_' || c == '"');
    if operand_missing && !operand_follows {
        line.push_str(PLACEHOLDER);
    }
    line.push_str(suffix);
    let mut open = Vec::new();
    for token in before.iter().chain(after.iter()) {
        match token.kind {
            TokenKind::LeftParen => open.push(')'),
            TokenKind::LeftBracket => open.push(']'),
            TokenKind::LeftBrace => open.push('}'),
            TokenKind::RightParen | TokenKind::RightBracket | TokenKind::RightBrace => {
                open.pop();
            }
            _ => {}
        }
    }
    while let Some(closer) = open.pop() {
        line.push(closer);
    }
    let completed = format!("{}{}{}", &text[..line_start], line, &text[line_end..]);
    if parses(&completed) {
        return Some(Repaired {
            text: completed,
            mapping: Mapping {
                line_start,
                shift,
                blanked: false,
            },
        });
    }
    let blanked = format!("{}{}", &text[..line_start], &text[line_end..]);
    let blanked_mapping = Mapping {
        line_start,
        shift: 0,
        blanked: true,
    };
    parses(&blanked).then_some(Repaired {
        text: blanked,
        mapping: blanked_mapping,
    })
}

fn parses(text: &str) -> bool {
    !parse(text).diagnostics.iter().any(Diagnostic::is_error)
}

// ------------------------------------------------------------ candidates

/// What the server knows at a request: the checked project, the file's
/// module (none when the file does not parse), the file's text as the
/// world read it, the position in the editor's text and the mapping of
/// the editor's offsets.
pub struct At<'a> {
    pub checked: &'a CheckedProject,
    pub module: Option<ModuleId>,
    pub text: &'a str,
    pub offset: usize,
    pub mapping: Mapping,
}

/// The completion items of a context; `library_modules` names every
/// module of the library, for an import.
pub fn candidates<'a>(
    at: &At<'a>,
    source_of: &dyn Fn(ModuleId) -> Option<&'a SourceFile>,
    context: &Context,
    library_modules: &[String],
) -> Vec<Candidate> {
    let world = &at.checked.world;
    let mut found = match context {
        Context::Import { start, typed } => {
            let project = world
                .modules
                .iter()
                .filter(|info| !info.is_library)
                .map(|info| (info.name.clone(), info.ast.docs.purpose.clone()));
            library_modules
                .iter()
                .map(|name| (name.clone(), None))
                .chain(project)
                .filter(|(name, _)| name.starts_with(typed.as_str()))
                .map(|(name, purpose)| Candidate {
                    label: name,
                    kind: ITEM_MODULE,
                    detail: None,
                    documentation: purpose,
                    group: 0,
                    edit: Some(Span::new(*start, *start + typed.len())),
                })
                .collect()
        }
        Context::Needs => TREE
            .iter()
            .map(|name| Candidate {
                label: name.to_string(),
                kind: ITEM_VALUE,
                detail: Some("capability".to_string()),
                documentation: None,
                group: 0,
                edit: None,
            })
            .collect(),
        Context::Dotted { name, dot } => {
            let Some(module) = at.module else {
                return Vec::new();
            };
            let info = &world.modules[module];
            match name.as_deref().and_then(|name| info.imports.get(name)) {
                Some(&target) => namespace_members(world, source_of, target),
                None => match receiver_type(at, module, *dot, name.as_deref()) {
                    Some(ty) => members_of(world, source_of, &ty),
                    None => Vec::new(),
                },
            }
        }
        Context::Bare => match at.module {
            Some(module) => names_in_scope(at, source_of, module),
            None => Vec::new(),
        },
    };
    found.sort_by(|a, b| (a.group, &a.label, a.kind).cmp(&(b.group, &b.label, b.kind)));
    found
}

/// A candidate described as hover describes its target.
fn described<'a>(
    world: &'a World,
    source_of: &dyn Fn(ModuleId) -> Option<&'a SourceFile>,
    label: &str,
    kind: u8,
    target: Target,
    group: u8,
) -> Candidate {
    let described = describe_target(world, source_of, &target);
    Candidate {
        label: label.to_string(),
        kind,
        detail: described.as_ref().map(|d| d.signature.clone()),
        documentation: described.and_then(|d| d.purpose),
        group,
        edit: None,
    }
}

/// The public functions that are not methods, the constants and the types
/// of a module, after its namespace.
fn namespace_members<'a>(
    world: &'a World,
    source_of: &dyn Fn(ModuleId) -> Option<&'a SourceFile>,
    target: ModuleId,
) -> Vec<Candidate> {
    let info = &world.modules[target];
    let mut found = Vec::new();
    for (name, ids) in &info.functions {
        for &id in ids {
            let function = &world.functions[id];
            if function.public && !function.is_method {
                found.push(described(
                    world,
                    source_of,
                    name,
                    ITEM_FUNCTION,
                    Target::Function(id),
                    0,
                ));
            }
        }
    }
    for (name, constant) in &info.constants {
        if constant.public {
            let target = Target::Constant(target, name.clone());
            found.push(described(world, source_of, name, ITEM_CONSTANT, target, 0));
        }
    }
    for (name, &id) in &info.types {
        if world.types[id].public {
            found.push(described(
                world,
                source_of,
                name,
                ITEM_CLASS,
                Target::Type(id),
                0,
            ));
        }
    }
    found
}

/// The names in scope at a bare position: the bindings declared before it
/// in the enclosing item, the module's functions, constants and types, the
/// imports' namespaces and exposed names, the prelude's functions that are
/// not methods, and the statement words inside a body.
fn names_in_scope<'a>(
    at: &At<'a>,
    source_of: &dyn Fn(ModuleId) -> Option<&'a SourceFile>,
    module: ModuleId,
) -> Vec<Candidate> {
    let world = &at.checked.world;
    let info = &world.modules[module];
    let position = at.mapping.at(at.offset);
    let mut found = Vec::new();
    let item = info
        .ast
        .items
        .iter()
        .find(|item| contains(item.span(), position));
    if let Some(item) = item {
        let span = item.span();
        let mut names = BTreeSet::new();
        for (owner, _, reference) in &at.checked.references {
            if let Target::Local(declared) = reference.target {
                if *owner == module
                    && declared.start >= span.start
                    && declared.end <= position.min(span.end)
                {
                    names.insert(at.text[declared.start..declared.end].to_string());
                }
            }
        }
        found.extend(names.into_iter().map(|name| Candidate {
            label: name,
            kind: ITEM_VARIABLE,
            detail: None,
            documentation: None,
            group: 0,
            edit: None,
        }));
    }
    for (name, ids) in &info.functions {
        for &id in ids {
            if !world.functions[id].is_method {
                found.push(described(
                    world,
                    source_of,
                    name,
                    ITEM_FUNCTION,
                    Target::Function(id),
                    1,
                ));
            }
        }
    }
    for name in info.constants.keys() {
        let target = Target::Constant(module, name.clone());
        found.push(described(world, source_of, name, ITEM_CONSTANT, target, 1));
    }
    for (name, &id) in &info.types {
        found.push(described(
            world,
            source_of,
            name,
            ITEM_CLASS,
            Target::Type(id),
            1,
        ));
    }
    for (namespace, &target) in &info.imports {
        let imported = &world.modules[target];
        found.push(Candidate {
            label: namespace.clone(),
            kind: ITEM_MODULE,
            detail: Some(imported.name.clone()),
            documentation: imported.ast.docs.purpose.clone(),
            group: 2,
            edit: None,
        });
    }
    for (name, &id) in &info.exposed_types {
        found.push(described(
            world,
            source_of,
            name,
            ITEM_CLASS,
            Target::Type(id),
            2,
        ));
    }
    for (name, &id) in &info.exposed_abilities {
        found.push(described(
            world,
            source_of,
            name,
            ITEM_INTERFACE,
            Target::Ability(id),
            2,
        ));
    }
    if let Some(prelude) = world.prelude {
        for (name, ids) in &world.modules[prelude].functions {
            for &id in ids {
                let function = &world.functions[id];
                if function.public && !function.is_method {
                    found.push(described(
                        world,
                        source_of,
                        name,
                        ITEM_FUNCTION,
                        Target::Function(id),
                        3,
                    ));
                }
            }
        }
    }
    if matches!(
        item,
        Some(Item::Function(_) | Item::Implementation(_) | Item::Test(_))
    ) {
        found.extend(STATEMENT_WORDS.iter().map(|word| Candidate {
            label: word.to_string(),
            kind: ITEM_KEYWORD,
            detail: None,
            documentation: None,
            group: 4,
            edit: None,
        }));
    }
    found
}

fn contains(span: Span, offset: usize) -> bool {
    span.start <= offset && offset <= span.end
}

/// The type of the receiver ending at the dot: the longest expression the
/// checker typed ending there; else, when the receiver is one name, a
/// parameter of the enclosing function by that name, else a binding by
/// the type of one of its uses (the line was repaired away, or the
/// receiver did not check).
fn receiver_type(at: &At, module: ModuleId, dot: usize, name: Option<&str>) -> Option<Ty> {
    let end = at.mapping.at(dot);
    if let Some(ty) = typed_ending_at(at.checked, module, end) {
        return Some(ty);
    }
    let name = name?;
    let world = &at.checked.world;
    let info = &world.modules[module];
    let item = info
        .ast
        .items
        .iter()
        .find(|item| contains(item.span(), end))?;
    let function = world
        .functions
        .iter()
        .find(|function| function.module == module && contains(function.span, end));
    if let Some((_, ty)) = function.and_then(|function| {
        function
            .params
            .iter()
            .find(|(parameter, _)| parameter == name)
    }) {
        return Some(ty.clone());
    }
    let span = item.span();
    let declared =
        at.checked
            .references
            .iter()
            .find_map(|(owner, _, reference)| match reference.target {
                Target::Local(declared)
                    if *owner == module
                        && declared.start >= span.start
                        && declared.end <= span.end
                        && &at.text[declared.start..declared.end] == name =>
                {
                    Some(declared)
                }
                _ => None,
            })?;
    at.checked
        .references
        .iter()
        .filter(|(owner, _, reference)| {
            *owner == module
                && reference.target == Target::Local(declared)
                && reference.span != declared
        })
        .find_map(|(_, _, reference)| typed_at(at.checked, module, reference.span))
}

/// The longest expression of the module the checker typed ending at the
/// offset.
fn typed_ending_at(checked: &CheckedProject, module: ModuleId, end: usize) -> Option<Ty> {
    checked
        .references
        .iter()
        .filter(|(owner, _, reference)| *owner == module && reference.span.end == end)
        .filter_map(|(_, _, reference)| match &reference.target {
            Target::Typed(ty) => Some((reference.span.len(), ty)),
            _ => None,
        })
        .max_by_key(|(length, _)| *length)
        .map(|(_, ty)| ty.clone())
}

/// The type the checker recorded for the expression at the span.
fn typed_at(checked: &CheckedProject, module: ModuleId, span: Span) -> Option<Ty> {
    checked
        .references
        .iter()
        .filter(|(owner, _, reference)| *owner == module && reference.span == span)
        .find_map(|(_, _, reference)| match &reference.target {
            Target::Typed(ty) => Some(ty.clone()),
            _ => None,
        })
}

/// The members of a type after a dot, each name once: the fields of a
/// record, the methods declared on the type, the methods of the abilities
/// it has, and the same of its base when it is a subtype; the methods of
/// its constraints' abilities for a type parameter.
fn members_of<'a>(
    world: &'a World,
    source_of: &dyn Fn(ModuleId) -> Option<&'a SourceFile>,
    ty: &Ty,
) -> Vec<Candidate> {
    let mut members = Members {
        found: Vec::new(),
        seen: BTreeSet::new(),
    };
    let mut current = ty.clone();
    loop {
        match &current {
            Ty::App(id, _) => {
                let id = *id;
                if let TypeKindInfo::Record(fields) = &world.types[id].kind {
                    for field in fields {
                        members.add(Candidate {
                            label: field.name.clone(),
                            kind: ITEM_FIELD,
                            detail: Some(format!("{}: {}", field.name, world.show(&field.ty))),
                            documentation: None,
                            group: 0,
                            edit: None,
                        });
                    }
                }
                let mut methods: Vec<(&String, &Vec<FunctionId>)> = world
                    .method_index
                    .iter()
                    .filter(|((head, _), _)| *head == id)
                    .map(|((_, name), ids)| (name, ids))
                    .collect();
                methods.sort();
                for (name, ids) in methods {
                    for &function in ids {
                        let target = Target::Function(function);
                        members.add(described(world, source_of, name, ITEM_METHOD, target, 1));
                    }
                }
                for ability in abilities_of_type(world, id) {
                    members.ability(world, source_of, ability);
                }
                match &world.types[id].kind {
                    TypeKindInfo::Subtype { base, .. } => current = base.clone(),
                    _ => break,
                }
            }
            Ty::Param(param) => {
                for ability in abilities_of_param(world, *param) {
                    members.ability(world, source_of, ability);
                }
                break;
            }
            _ => break,
        }
    }
    members.found
}

struct Members {
    found: Vec<Candidate>,
    seen: BTreeSet<String>,
}

impl Members {
    fn add(&mut self, candidate: Candidate) {
        if self.seen.insert(candidate.label.clone()) {
            self.found.push(candidate);
        }
    }

    fn ability<'a>(
        &mut self,
        world: &'a World,
        source_of: &dyn Fn(ModuleId) -> Option<&'a SourceFile>,
        ability: AbilityId,
    ) {
        for (index, method) in world.abilities[ability].methods.iter().enumerate() {
            let target = Target::AbilityMethod(ability, index);
            self.add(described(
                world,
                source_of,
                &method.name,
                ITEM_METHOD,
                target,
                2,
            ));
        }
    }
}

/// The abilities a type has: `Equal`, which every data type has, what it
/// derives, and what is implemented for it.
fn abilities_of_type(world: &World, id: TypeId) -> Vec<AbilityId> {
    let mut found = vec![world.builtins.equal];
    found.extend(world.types[id].derives.iter().copied());
    found.extend(
        world
            .impls
            .iter()
            .filter(|implementation| head_type(&implementation.target) == Some(id))
            .map(|implementation| implementation.ability),
    );
    found.sort_unstable();
    found.dedup();
    found
}

/// The abilities a type parameter has: its constraints' and what they
/// require.
fn abilities_of_param(world: &World, param: renyi_check::types::ParamId) -> Vec<AbilityId> {
    let mut found = vec![world.builtins.equal];
    for constraint in world.constraints(param) {
        found.push(constraint.ability);
        found.extend(
            world.abilities[constraint.ability]
                .requirements
                .iter()
                .map(|required| required.ability),
        );
    }
    found.sort_unstable();
    found.dedup();
    found
}

// ------------------------------------------------------------ signatures

/// The signatures of the callee being written.
pub fn signatures(at: &At, callee: &Callee) -> Vec<Signature> {
    let Some(module) = at.module else {
        return Vec::new();
    };
    let world = &at.checked.world;
    let info = &world.modules[module];
    match callee {
        Callee::Bare(name) => world
            .lookup_function(module, name)
            .or_else(|| {
                world
                    .prelude
                    .and_then(|prelude| world.lookup_function(prelude, name))
            })
            .map(|id| function_signatures(world, &[id]))
            .unwrap_or_default(),
        Callee::Qualified { first, dot, name } => match info.imports.get(first) {
            Some(&target) => match world.lookup_function(target, name) {
                Some(id) => function_signatures(world, &[id]),
                None => world.modules[target]
                    .types
                    .get(name)
                    .and_then(|&id| construction(world, id, None))
                    .into_iter()
                    .collect(),
            },
            None => receiver_type(at, module, *dot, Some(first))
                .map(|ty| methods_named(world, &ty, name))
                .unwrap_or_default(),
        },
        Callee::Method { dot, name } => receiver_type(at, module, *dot, None)
            .map(|ty| methods_named(world, &ty, name))
            .unwrap_or_default(),
        Callee::Construct(name) => constructions(world, module, name),
    }
}

/// The signature of each function, its parameters after `self` for a
/// method.
fn function_signatures(world: &World, ids: &[FunctionId]) -> Vec<Signature> {
    ids.iter()
        .filter_map(|&id| {
            let (_, function) = function_item(world, id)?;
            let skip = usize::from(world.functions[id].is_method);
            Some(Signature {
                label: function_signature(function),
                documentation: function.docs.purpose.clone(),
                parameters: function.params.iter().skip(skip).map(param_text).collect(),
            })
        })
        .collect()
}

/// The signature of an ability's method, its parameters after `self`.
fn ability_method_signature(world: &World, ability: AbilityId, index: usize) -> Option<Signature> {
    let (_, decl) = ability_item(world, ability)?;
    let function = decl.functions.get(index)?;
    Some(Signature {
        label: function_signature(function),
        documentation: function.docs.purpose.clone(),
        parameters: function.params.iter().skip(1).map(param_text).collect(),
    })
}

/// A parameter as the signature spells it.
fn param_text(param: &Param) -> String {
    match &param.ty {
        Some(ty) => format!("{}: {}", param.name.text, type_text(ty)),
        None => param.name.text.clone(),
    }
}

/// The methods of a receiver type by name: the ones declared on the type,
/// else the abilities' it has, else its base's for a subtype.
fn methods_named(world: &World, ty: &Ty, name: &str) -> Vec<Signature> {
    let mut current = ty.clone();
    loop {
        let abilities = match &current {
            Ty::App(id, _) => {
                let declared = world.methods_of(*id, name);
                if !declared.is_empty() {
                    return function_signatures(world, &declared);
                }
                abilities_of_type(world, *id)
            }
            Ty::Param(param) => abilities_of_param(world, *param),
            _ => return Vec::new(),
        };
        for ability in abilities {
            let methods = &world.abilities[ability].methods;
            if let Some(index) = methods.iter().position(|method| method.name == name) {
                return ability_method_signature(world, ability, index)
                    .into_iter()
                    .collect();
            }
        }
        match &current {
            Ty::App(id, _) => match &world.types[*id].kind {
                TypeKindInfo::Subtype { base, .. } => current = base.clone(),
                _ => return Vec::new(),
            },
            _ => return Vec::new(),
        }
    }
}

/// The signature of a record built by its name, or of a variant, among
/// the types in scope: the module's, the exposed ones and the prelude's.
fn constructions(world: &World, module: ModuleId, name: &str) -> Vec<Signature> {
    let info = &world.modules[module];
    let mut in_scope: Vec<TypeId> = info
        .types
        .values()
        .chain(info.exposed_types.values())
        .copied()
        .collect();
    if let Some(prelude) = world.prelude {
        in_scope.extend(world.modules[prelude].types.values().copied());
    }
    if let Some(&id) = in_scope.iter().find(|&&id| world.types[id].name == name) {
        return construction(world, id, None).into_iter().collect();
    }
    in_scope
        .iter()
        .find_map(|&id| match &world.types[id].kind {
            TypeKindInfo::Sum(variants) => variants
                .iter()
                .position(|variant| variant.name == name)
                .and_then(|index| construction(world, id, Some(index))),
            _ => None,
        })
        .into_iter()
        .collect()
}

/// `Name(field: Type, ...)` for a record, or for one variant of a sum.
fn construction(world: &World, id: TypeId, variant: Option<usize>) -> Option<Signature> {
    let (_, def) = type_item(world, id)?;
    let (name, fields): (&str, &[Field]) = match (&def.kind, variant) {
        (TypeKind::Record { fields, .. }, None) => (def.name.text.as_str(), fields.as_slice()),
        (TypeKind::Sum { variants, .. }, Some(index)) => {
            let variant = variants.get(index)?;
            (variant.name.text.as_str(), variant.fields.as_slice())
        }
        _ => return None,
    };
    let parameters: Vec<String> = fields
        .iter()
        .map(|field| format!("{}: {}", field.name.text, type_text(&field.ty)))
        .collect();
    Some(Signature {
        label: format!("{name}({})", parameters.join(", ")),
        documentation: def.docs.purpose.clone(),
        parameters,
    })
}

// --------------------------------------------------------------- symbols

/// Every item of the files, and every part of one (a field, a variant, a
/// method), whose name contains the query, case aside; every item when
/// the query is empty.
pub fn workspace_symbols(
    world: &World,
    files: &[(ModuleId, &SourceFile)],
    query: &str,
) -> Vec<Found> {
    let query = query.to_lowercase();
    let matched = |name: &str| query.is_empty() || name.to_lowercase().contains(&query);
    let mut found = Vec::new();
    for &(module, file) in files {
        let info = &world.modules[module];
        for symbol in symbols_of(&info.ast, file) {
            if matched(&symbol.name) {
                found.push(Found {
                    name: symbol.name.clone(),
                    kind: symbol.kind,
                    container: info.name.clone(),
                    module,
                    span: symbol.selection,
                });
            }
            for child in &symbol.children {
                if matched(&child.name) {
                    found.push(Found {
                        name: child.name.clone(),
                        kind: child.kind,
                        container: format!("{}.{}", info.name, symbol.name),
                        module,
                        span: child.selection,
                    });
                }
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_context_is_read_from_the_line_before_the_position() {
        let at = |line: &str| context_at(line, line.len());
        assert_eq!(
            at("import std."),
            Context::Import {
                start: 7,
                typed: "std.".to_string()
            }
        );
        assert_eq!(
            at("import "),
            Context::Import {
                start: 7,
                typed: String::new()
            }
        );
        assert_eq!(at("public function main() needs con"), Context::Needs);
        assert_eq!(at("public function main() needs console, "), Context::Needs);
        assert_eq!(
            at("  console.pr"),
            Context::Dotted {
                name: Some("console".to_string()),
                dot: 9
            }
        );
        assert_eq!(
            at("  let total be square."),
            Context::Dotted {
                name: Some("square".to_string()),
                dot: 21
            }
        );
        assert_eq!(
            at("  let a be f(x)."),
            Context::Dotted {
                name: None,
                dot: 15
            }
        );
        assert_eq!(at("  a.b."), Context::Dotted { name: None, dot: 5 });
        // inside the hole of a text literal
        assert_eq!(
            at("  console.print(\"x {square."),
            Context::Dotted {
                name: Some("square".to_string()),
                dot: 26
            }
        );
        assert_eq!(at("  let a be tot"), Context::Bare);
        assert_eq!(at("  "), Context::Bare);
        // the second line of a text
        assert_eq!(
            context_at("import x\n  square.", 18),
            Context::Dotted {
                name: Some("square".to_string()),
                dot: 17
            }
        );
    }

    #[test]
    fn the_call_is_found_by_its_open_parenthesis() {
        let at = |line: &str| call_at(line, line.len());
        let call = at("  let x be area(total, ").expect("a call");
        assert!(matches!(call.callee, Callee::Bare(ref name) if name == "area"));
        assert_eq!(call.active, 1);
        let call = at("  shapes.area(").expect("a call");
        assert!(matches!(
            call.callee,
            Callee::Qualified { ref first, dot: 8, ref name } if first == "shapes" && name == "area"
        ));
        assert_eq!(call.active, 0);
        let call = at("  f(x).label(a, b, ").expect("a call");
        assert!(matches!(call.callee, Callee::Method { dot: 6, ref name } if name == "label"));
        assert_eq!(call.active, 2);
        let call = at("  let s be Square(side: ").expect("a call");
        assert!(matches!(call.callee, Callee::Construct(ref name) if name == "Square"));
        assert_eq!(call.active, 0);
        // an inner call closed counts as one argument
        let call = at("  f(g(a), ").expect("a call");
        assert!(matches!(call.callee, Callee::Bare(ref name) if name == "f"));
        assert_eq!(call.active, 1);
        // inside the hole of a text literal
        let call = at("  console.print(\"{shapes.area(").expect("a call");
        assert!(matches!(call.callee, Callee::Qualified { ref name, .. } if name == "area"));
        assert!(at("  let x be total + 1").is_none());
        assert!(at("  let x be [a, ").is_none());
        assert!(at("  let x be (a + ").is_none());
    }

    fn program(line: &str) -> String {
        format!(
            "module demo
  purpose: A program for the repair test.

public function main()
  purpose: Nothing much.

  let square be 3
{line}
end
"
        )
    }

    #[test]
    fn a_line_the_parser_refuses_is_completed_or_blanked() {
        let text = program("  let total be square.");
        let offset = text.len() - "\nend\n".len();
        let line_start = text.rfind("  let total").expect("the line");
        let repaired = repair(&text, offset).expect("repaired");
        assert!(
            repaired.text.contains("  let total be square.xx\n"),
            "{}",
            repaired.text
        );
        assert_eq!(
            repaired.mapping.at(offset - 1),
            offset - 1,
            "no shift before the cursor"
        );
        assert_eq!(repaired.mapping.at(line_start - 1), line_start - 1);
        // a bare expression is ignored, which moves the line by seven
        let text = program("  square.");
        let offset = text.len() - "\nend\n".len();
        let line_start = text.rfind("  square").expect("the line");
        let repaired = repair(&text, offset).expect("repaired");
        assert!(
            repaired.text.contains("  ignore square.xx\n"),
            "{}",
            repaired.text
        );
        assert_eq!(repaired.mapping.at(line_start + 2), line_start + 9);
        assert_eq!(repaired.mapping.at(line_start - 1), line_start - 1);
        // a call not closed
        let text = program("  let total be area(square,");
        let offset = text.len() - "\nend\n".len();
        let repaired = repair(&text, offset).expect("repaired");
        assert!(
            repaired.text.contains("area(square,xx)\n"),
            "{}",
            repaired.text
        );
        // a line that cannot be completed is blanked
        let text = program("  if square.");
        let offset = text.len() - "\nend\n".len();
        let line_start = text.rfind("  if square").expect("the line");
        let repaired = repair(&text, offset).expect("repaired");
        assert!(!repaired.text.contains("if square"), "{}", repaired.text);
        assert_eq!(repaired.mapping.at(offset), line_start);
        // the cursor on a blank line of a file broken elsewhere: nothing to repair
        let text = program("  square.\n");
        let offset = text.len() - "\nend\n".len();
        assert!(repair(&text, offset).is_none());
        // the cursor in the middle of a complete call repairs nothing it should not
        let text = program("  let total be area(square, 3)");
        let offset = text.rfind("square, 3").expect("the argument");
        assert!(parses(&text));
        let repaired = repair(&text, offset).expect("repaired");
        assert!(
            repaired.text.contains("area(square, 3)\n"),
            "{}",
            repaired.text
        );
    }
}
