//! One draft per definition: what the syntax tree and the checker's tables
//! say about it before its edges, metrics and hash are known. Signatures
//! are spelled with the formatter's helpers, so that they read as the
//! canonical source does.

use renyi_check::effects::Capability;
use renyi_check::types::Ty;
use renyi_check::world::head_type;
use renyi_check::{AbilityId, FunctionId, ModuleId, Target, TypeId, World};
use renyi_syntax::ast;
use renyi_syntax::{capabilities_text, for_any_text, type_text, SourceFile, Span};

use crate::metrics;
use crate::{Implements, Kind};

/// The identity of a definition while the map is built.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Function(FunctionId),
    Type(TypeId),
    Ability(AbilityId),
    Implementation(usize),
    Constant(ModuleId, String),
    Test(ModuleId, usize),
}

/// A definition before its edges, metrics and hash are known.
pub struct Draft {
    pub key: Key,
    pub module: ModuleId,
    pub file: usize,
    pub kind: Kind,
    pub name: String,
    pub public: bool,
    pub signature: String,
    pub docs: ast::Docs,
    pub span: Span,
    pub name_span: Option<Span>,
    pub depth: usize,
    pub branches: usize,
    pub needs: Vec<Capability>,
    pub fails: Vec<Ty>,
    /// Functions whose bodies this definition reaches without a reference:
    /// the methods of an implementation.
    pub starts: Vec<FunctionId>,
    pub refs: Vec<(Target, Span)>,
    pub implements: Option<Implements>,
}

pub fn function_draft(
    world: &World,
    module: ModuleId,
    file: usize,
    id: FunctionId,
    function: &ast::Function,
    mut refs: Vec<(Target, Span)>,
    implements: Option<Implements>,
) -> Draft {
    let info = &world.functions[id];
    signature_refs(world, module, function, &mut refs);
    let (depth, branches) = function
        .body
        .as_ref()
        .map(metrics::block_metrics)
        .unwrap_or((0, 0));
    let name = match receiver_type_name(world, id) {
        Some(receiver) => format!("{receiver}.{}", info.name),
        None => info.name.clone(),
    };
    Draft {
        key: Key::Function(id),
        module,
        file,
        kind: if info.is_method {
            Kind::Method
        } else {
            Kind::Function
        },
        name,
        public: info.public,
        signature: function_signature(function),
        docs: function.docs.clone(),
        span: function.span,
        name_span: Some(function.name.span),
        depth,
        branches,
        needs: info.needs.clone(),
        fails: info.fails.clone(),
        starts: Vec::new(),
        refs,
        implements,
    }
}

pub fn type_draft(
    world: &World,
    module: ModuleId,
    file: usize,
    text: &SourceFile,
    def: &ast::TypeDef,
) -> Draft {
    let id = world.modules[module].types[&def.name.text];
    let mut refs = Vec::new();
    match &def.kind {
        ast::TypeKind::Record { fields, derives } => {
            for field in fields {
                type_refs(world, module, &field.ty, &mut refs);
            }
            derive_refs(world, module, derives, &mut refs);
        }
        ast::TypeKind::Sum { variants, derives } => {
            for variant in variants {
                for field in &variant.fields {
                    type_refs(world, module, &field.ty, &mut refs);
                }
            }
            derive_refs(world, module, derives, &mut refs);
        }
        ast::TypeKind::Subtype { base, .. } => type_refs(world, module, base, &mut refs),
    }
    Draft {
        key: Key::Type(id),
        module,
        file,
        kind: Kind::Type,
        name: def.name.text.clone(),
        public: def.public,
        signature: type_signature(def, text),
        docs: def.docs.clone(),
        span: def.span,
        name_span: Some(def.name.span),
        depth: 0,
        branches: 0,
        needs: Vec::new(),
        fails: Vec::new(),
        starts: Vec::new(),
        refs,
        implements: None,
    }
}

pub fn ability_draft(
    world: &World,
    module: ModuleId,
    file: usize,
    ability: &ast::AbilityDecl,
) -> Draft {
    let id = world.modules[module].abilities[&ability.name.text];
    let mut refs = Vec::new();
    for requirement in &ability.requirements {
        if let ast::Type::Named { name, .. } = requirement {
            if let Some(required) = world.lookup_ability(module, &name.text) {
                refs.push((Target::Ability(required), name.span));
            }
        }
    }
    for function in &ability.functions {
        signature_refs(world, module, function, &mut refs);
    }
    let mut signature = ability.name.text.clone();
    if !ability.type_params.is_empty() {
        let params: Vec<&str> = ability
            .type_params
            .iter()
            .map(|p| p.text.as_str())
            .collect();
        signature.push_str(&format!(" of {}", params.join(", ")));
    }
    if !ability.requirements.is_empty() {
        let requirements: Vec<String> = ability.requirements.iter().map(type_text).collect();
        signature.push_str(&format!(" where self can {}", requirements.join(" and ")));
    }
    if !ability.functions.is_empty() {
        let methods: Vec<String> = ability.functions.iter().map(function_signature).collect();
        signature.push_str(&format!(": {}", methods.join("; ")));
    }
    Draft {
        key: Key::Ability(id),
        module,
        file,
        kind: Kind::Ability,
        name: ability.name.text.clone(),
        public: ability.public,
        signature,
        docs: ability.docs.clone(),
        span: ability.span,
        name_span: Some(ability.name.span),
        depth: 0,
        branches: 0,
        needs: Vec::new(),
        fails: Vec::new(),
        starts: Vec::new(),
        refs,
        implements: None,
    }
}

/// The implementation as a whole; what its methods refer to, it refers to,
/// and its depth and branches are the maximum and the sum over them.
pub fn implementation_draft(
    world: &World,
    module: ModuleId,
    file: usize,
    impl_index: usize,
    implementation: &ast::AbilityImpl,
    implements: Implements,
    methods: &[Draft],
) -> Draft {
    let info = &world.impls[impl_index];
    let mut refs = Vec::new();
    if let ast::Type::Named { name, .. } = &implementation.ability {
        refs.push((Target::Ability(info.ability), name.span));
    }
    type_refs(world, module, &implementation.target, &mut refs);
    if let Some(for_any) = &implementation.type_params {
        for_any_refs(world, module, for_any, &mut refs);
    }
    for method in methods {
        refs.extend(method.refs.iter().cloned());
    }
    let name = format!(
        "{} for {}",
        type_text(&implementation.ability),
        type_text(&implementation.target)
    );
    let mut signature = name.clone();
    if let Some(for_any) = &implementation.type_params {
        signature.push(' ');
        signature.push_str(&for_any_text(for_any));
    }
    let (depth, branches) = methods
        .iter()
        .fold((0, 0), |(d, b), m| (d.max(m.depth), b + m.branches));
    Draft {
        key: Key::Implementation(impl_index),
        module,
        file,
        kind: Kind::Implementation,
        name,
        public: false,
        signature,
        docs: ast::Docs::default(),
        span: implementation.span,
        name_span: None,
        depth,
        branches,
        needs: Vec::new(),
        fails: Vec::new(),
        starts: info.functions.clone(),
        refs,
        implements: Some(implements),
    }
}

pub fn constant_draft(
    world: &World,
    module: ModuleId,
    file: usize,
    constant: &ast::Constant,
    mut refs: Vec<(Target, Span)>,
) -> Draft {
    type_refs(world, module, &constant.ty, &mut refs);
    let (depth, branches) = metrics::expr_metrics(&constant.value);
    Draft {
        key: Key::Constant(module, constant.name.text.clone()),
        module,
        file,
        kind: Kind::Constant,
        name: constant.name.text.clone(),
        public: constant.public,
        signature: format!("{}: {}", constant.name.text, type_text(&constant.ty)),
        docs: constant.docs.clone(),
        span: constant.span,
        name_span: Some(constant.name.span),
        depth,
        branches,
        needs: Vec::new(),
        fails: Vec::new(),
        starts: Vec::new(),
        refs,
        implements: None,
    }
}

pub fn test_draft(
    module: ModuleId,
    file: usize,
    index: usize,
    test: &ast::Test,
    refs: Vec<(Target, Span)>,
) -> Draft {
    let (depth, branches) = metrics::block_metrics(&test.body);
    let mut signature = format!("{:?}", test.name);
    if !test.needs.is_empty() {
        signature.push_str(&format!(" needs {}", capabilities_text(&test.needs)));
    }
    if let Some(replays) = &test.replays {
        signature.push_str(&format!(" replays {replays:?}"));
    }
    Draft {
        key: Key::Test(module, index),
        module,
        file,
        kind: Kind::Test,
        name: format!("test:{}", test.name),
        public: false,
        signature,
        docs: ast::Docs::default(),
        span: test.span,
        name_span: None,
        depth,
        branches,
        needs: test.needs.iter().map(Capability::from_ast).collect(),
        fails: Vec::new(),
        starts: Vec::new(),
        refs,
        implements: None,
    }
}

// ------------------------------------------------------------- signatures

/// `name(params) returns R or fails with E needs C for any T`, one line.
pub fn function_signature(function: &ast::Function) -> String {
    let params: Vec<String> = function
        .params
        .iter()
        .map(|param| match &param.ty {
            Some(ty) => format!("{}: {}", param.name.text, type_text(ty)),
            None => param.name.text.clone(),
        })
        .collect();
    let mut out = format!("{}({})", function.name.text, params.join(", "));
    if let Some(returns) = &function.returns {
        out.push_str(&format!(" returns {}", type_text(returns)));
    }
    if !function.fails.is_empty() {
        let fails: Vec<String> = function.fails.iter().map(type_text).collect();
        out.push_str(&format!(" or fails with {}", fails.join(" or ")));
    }
    if !function.needs.is_empty() {
        out.push_str(&format!(" needs {}", capabilities_text(&function.needs)));
    }
    if let Some(for_any) = &function.type_params {
        out.push(' ');
        out.push_str(&for_any_text(for_any));
    }
    out
}

/// A record as `Name(field: Type, ...)`, a sum type as `Name is A(...) or
/// B`, a subtype as `Name is Base where condition`; field conditions are
/// kept, since they decide what can be constructed.
fn type_signature(def: &ast::TypeDef, text: &SourceFile) -> String {
    let mut head = def.name.text.clone();
    if !def.type_params.is_empty() {
        let params: Vec<&str> = def.type_params.iter().map(|p| p.text.as_str()).collect();
        head.push_str(&format!(" of {}", params.join(", ")));
    }
    match &def.kind {
        ast::TypeKind::Record { fields, .. } => {
            format!("{head}({})", fields_text(fields, text))
        }
        ast::TypeKind::Sum { variants, .. } => {
            let variants: Vec<String> = variants
                .iter()
                .map(|variant| {
                    if variant.fields.is_empty() {
                        variant.name.text.clone()
                    } else {
                        format!(
                            "{}({})",
                            variant.name.text,
                            fields_text(&variant.fields, text)
                        )
                    }
                })
                .collect();
            format!("{head} is {}", variants.join(" or "))
        }
        ast::TypeKind::Subtype { base, refinement } => {
            let mut out = format!("{head} is {}", type_text(base));
            if let Some(condition) = refinement {
                out.push_str(" where ");
                out.push_str(&collapse(text.slice(condition.span)));
            }
            out
        }
    }
}

fn fields_text(fields: &[ast::Field], text: &SourceFile) -> String {
    fields
        .iter()
        .map(|field| {
            let mut out = format!("{}: {}", field.name.text, type_text(&field.ty));
            if let Some(condition) = &field.refinement {
                out.push_str(" where ");
                out.push_str(&collapse(text.slice(condition.span)));
            }
            if let Some(external) = &field.external_name {
                out.push_str(&format!(" as {external:?}"));
            }
            out
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn collapse(source: &str) -> String {
    source.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ------------------------------------------------------------- references

/// The types and abilities a signature names, each at its name token.
fn signature_refs(
    world: &World,
    module: ModuleId,
    function: &ast::Function,
    refs: &mut Vec<(Target, Span)>,
) {
    for param in &function.params {
        if let Some(ty) = &param.ty {
            type_refs(world, module, ty, refs);
        }
    }
    if let Some(returns) = &function.returns {
        type_refs(world, module, returns, refs);
    }
    for fails in &function.fails {
        type_refs(world, module, fails, refs);
    }
    if let Some(for_any) = &function.type_params {
        for_any_refs(world, module, for_any, refs);
    }
}

fn type_refs(world: &World, module: ModuleId, ty: &ast::Type, refs: &mut Vec<(Target, Span)>) {
    match ty {
        ast::Type::Named { name, args, .. } => {
            if let Some(id) = world.lookup_type(module, &name.text) {
                refs.push((Target::Type(id), name.span));
            }
            for arg in args {
                type_refs(world, module, arg, refs);
            }
        }
        ast::Type::Maybe(inner, _) => type_refs(world, module, inner, refs),
        ast::Type::Function {
            params,
            returns,
            fails,
            ..
        } => {
            for param in params {
                type_refs(world, module, param, refs);
            }
            if let Some(returns) = returns {
                type_refs(world, module, returns, refs);
            }
            for fails in fails {
                type_refs(world, module, fails, refs);
            }
        }
    }
}

fn for_any_refs(
    world: &World,
    module: ModuleId,
    for_any: &ast::ForAny,
    refs: &mut Vec<(Target, Span)>,
) {
    for constraint in &for_any.constraints {
        if let ast::Type::Named { name, .. } = &constraint.ability {
            if let Some(id) = world.lookup_ability(module, &name.text) {
                refs.push((Target::Ability(id), name.span));
            }
        }
    }
}

fn derive_refs(
    world: &World,
    module: ModuleId,
    derives: &[ast::Derive],
    refs: &mut Vec<(Target, Span)>,
) {
    for derive in derives {
        if let Some(id) = world.lookup_ability(module, &derive.ability.text) {
            refs.push((Target::Ability(id), derive.ability.span));
        }
    }
}

// ------------------------------------------------------------------ names

pub fn qualified_name(world: &World, draft: &Draft) -> String {
    format!("{}.{}", world.modules[draft.module].name, draft.name)
}

/// `module.name`, or `module.Type.name` for a method.
pub fn function_qualified(world: &World, id: FunctionId) -> String {
    let function = &world.functions[id];
    let module = &world.modules[function.module].name;
    match receiver_type_name(world, id) {
        Some(receiver) => format!("{module}.{receiver}.{}", function.name),
        None => format!("{module}.{}", function.name),
    }
}

/// The name of the type a method's `self` has.
fn receiver_type_name(world: &World, id: FunctionId) -> Option<String> {
    let function = &world.functions[id];
    if !function.is_method {
        return None;
    }
    let self_ty = &function.params.first()?.1;
    match head_type(self_ty) {
        Some(type_id) => Some(world.types[type_id].name.clone()),
        None => match self_ty {
            Ty::Param(param) => Some(world.param_name(*param)),
            _ => Some("Self".to_string()),
        },
    }
}

pub fn type_qualified(world: &World, id: TypeId) -> String {
    let info = &world.types[id];
    format!("{}.{}", world.modules[info.module].name, info.name)
}

pub fn ability_qualified(world: &World, id: AbilityId) -> String {
    let info = &world.abilities[id];
    format!("{}.{}", world.modules[info.module].name, info.name)
}

pub fn implements_of(world: &World, impl_index: usize) -> Implements {
    let info = &world.impls[impl_index];
    let target = match head_type(&info.target) {
        Some(type_id) => type_qualified(world, type_id),
        None => world.show(&info.target),
    };
    Implements {
        ability: ability_qualified(world, info.ability),
        target,
    }
}
