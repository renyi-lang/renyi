//! Declarations: every module's types, abilities, implementations, functions
//! and constants, resolved into the checker's tables. The library modules and
//! the user's modules go through the same pass; bodies are checked afterwards
//! by `check`.

use std::collections::HashMap;

use renyi_syntax::ast::{self, Item, TypeKind};
use renyi_syntax::{Diagnostic, Span};

use crate::effects::Capability;
use crate::types::*;

pub struct ModuleInfo {
    /// The dotted module name, `std.json` or `invoice`.
    pub name: String,
    pub is_library: bool,
    pub ast: ast::Module,
    pub types: HashMap<String, TypeId>,
    pub abilities: HashMap<String, AbilityId>,
    /// Functions by name; several methods may share a name on different
    /// receiver types.
    pub functions: HashMap<String, Vec<FunctionId>>,
    pub constants: HashMap<String, ConstantInfo>,
    /// Namespace (the last segment or the `as` name) to module.
    pub imports: HashMap<String, ModuleId>,
    pub exposed_types: HashMap<String, TypeId>,
    pub exposed_abilities: HashMap<String, AbilityId>,
    /// The byte offset of every line start of the module's file, for the
    /// body-length rule (decision V5); empty for a library module.
    pub line_starts: Vec<usize>,
}

pub struct ConstantInfo {
    pub ty: Ty,
    pub public: bool,
    pub item_index: usize,
    pub deprecated: Option<String>,
    pub span: Span,
}

pub struct TypeInfo {
    pub name: String,
    pub module: ModuleId,
    pub params: Vec<ParamId>,
    pub kind: TypeKindInfo,
    /// Abilities derived with `can`.
    pub derives: Vec<AbilityId>,
    pub public: bool,
    pub deprecated: Option<String>,
    pub span: Span,
}

pub enum TypeKindInfo {
    /// A base type of the prelude: Integer, Text, List, ...
    Opaque,
    Record(Vec<FieldInfo>),
    Sum(Vec<VariantInfo>),
    Subtype {
        base: Ty,
        refinement: Option<ast::Expr>,
    },
    Unresolved,
}

#[derive(Clone)]
pub struct FieldInfo {
    pub name: String,
    pub ty: Ty,
    pub refinement: Option<ast::Expr>,
    /// `as "type"`: the key JSON and database decoders use (decision J14).
    pub external_name: Option<String>,
    pub span: Span,
}

#[derive(Clone)]
pub struct VariantInfo {
    pub name: String,
    pub fields: Vec<FieldInfo>,
    pub span: Span,
}

pub struct FunctionInfo {
    pub name: String,
    pub module: ModuleId,
    pub public: bool,
    pub params: Vec<(String, Ty)>,
    /// The first parameter is `self`: a method, called after a dot.
    pub is_method: bool,
    pub type_params: Vec<ParamId>,
    pub returns: Option<Ty>,
    pub fails: Vec<Ty>,
    pub needs: Vec<Capability>,
    pub is_library: bool,
    /// Where the body lives: an item of the module, or a function of an
    /// implementation item.
    pub body: BodyLocation,
    pub has_examples: bool,
    /// The text of the `deprecated:` clause (decision C8c).
    pub deprecated: Option<String>,
    /// `expose as tool` (decision D6).
    pub expose_as_tool: bool,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BodyLocation {
    None,
    Item(usize),
    Implementation(usize, usize),
    /// The `example` lines of a function: the item, the function's index when
    /// the item is an implementation, and the example's index.
    Example {
        item: usize,
        method: Option<usize>,
        example: usize,
    },
    /// A refinement condition of a type item: `where value ...` of a subtype
    /// (no variant, no field), or the condition of a field of a record or of
    /// a variant.
    Condition {
        item: usize,
        variant: Option<usize>,
        field: Option<usize>,
    },
}

pub struct AbilityInfo {
    pub name: String,
    pub module: ModuleId,
    pub public: bool,
    /// The ability's own type parameters, in scope in its method signatures.
    pub params: Vec<ParamId>,
    pub methods: Vec<AbilityMethod>,
    pub span: Span,
}

pub struct AbilityMethod {
    pub name: String,
    /// Parameters after `self`.
    pub params: Vec<(String, Ty)>,
    pub returns: Option<Ty>,
    pub fails: Vec<Ty>,
}

pub struct ImplInfo {
    pub ability: AbilityId,
    pub target: Ty,
    pub type_params: Vec<ParamId>,
    pub module: ModuleId,
    pub functions: Vec<FunctionId>,
    pub span: Span,
}

pub struct ParamInfo {
    pub name: String,
    pub constraints: Vec<AbilityId>,
}

/// The ids of the prelude's types and abilities, looked up once.
#[derive(Default, Clone)]
pub struct Builtins {
    pub integer: TypeId,
    pub decimal: TypeId,
    pub float: TypeId,
    pub boolean: TypeId,
    pub text: TypeId,
    pub bytes: TypeId,
    pub list: TypeId,
    pub map: TypeId,
    pub set: TypeId,
    pub range: TypeId,
    pub pair: TypeId,
    pub duration: TypeId,
    pub ordering: TypeId,
    pub timed_out: TypeId,
    pub constraint_violation: TypeId,
    /// The error of a value sent past its guard (decision P3).
    pub guarded: TypeId,
    pub equal: AbilityId,
    pub compare: AbilityId,
    pub hash: AbilityId,
    pub to_text: AbilityId,
}

#[derive(Default)]
pub struct World {
    pub modules: Vec<ModuleInfo>,
    pub types: Vec<TypeInfo>,
    pub functions: Vec<FunctionInfo>,
    pub abilities: Vec<AbilityInfo>,
    pub impls: Vec<ImplInfo>,
    pub params: Vec<ParamInfo>,
    pub builtins: Builtins,
    pub prelude: Option<ModuleId>,
    /// Methods by the head type of their `self` parameter.
    pub method_index: HashMap<(TypeId, String), Vec<FunctionId>>,
    /// Diagnostics raised while declaring, per module.
    pub diagnostics: Vec<(ModuleId, Diagnostic)>,
}

impl World {
    pub fn new() -> World {
        World::default()
    }

    // ------------------------------------------------------------ registering

    /// Register a module's names. Details are resolved by `resolve_all` once
    /// every module is registered, so that modules may import each other in
    /// any order.
    pub fn add_module(&mut self, module: ast::Module, is_library: bool) -> ModuleId {
        let id = self.modules.len();
        let name: Vec<&str> = module.name.iter().map(|n| n.text.as_str()).collect();
        let name = name.join(".");
        let mut info = ModuleInfo {
            name: name.clone(),
            is_library,
            ast: module,
            types: HashMap::new(),
            abilities: HashMap::new(),
            functions: HashMap::new(),
            constants: HashMap::new(),
            imports: HashMap::new(),
            exposed_types: HashMap::new(),
            exposed_abilities: HashMap::new(),
            line_starts: Vec::new(),
        };
        if name == "std.prelude" {
            self.prelude = Some(id);
        }
        // first the type and ability names, so that signatures can mention them
        for item in &info.ast.items {
            match item {
                Item::Type(def) => {
                    let type_id = self.types.len();
                    self.types.push(TypeInfo {
                        name: def.name.text.clone(),
                        module: id,
                        params: Vec::new(),
                        kind: TypeKindInfo::Unresolved,
                        derives: Vec::new(),
                        public: def.public,
                        deprecated: def.docs.deprecated.clone(),
                        span: def.span,
                    });
                    if info.types.insert(def.name.text.clone(), type_id).is_some() {
                        self.diagnostics.push((
                            id,
                            Diagnostic::error(
                                "duplicate-name",
                                format!("the type `{}` is declared twice", def.name.text),
                                def.name.span,
                            ),
                        ));
                    }
                }
                Item::Ability(ability) => {
                    let ability_id = self.abilities.len();
                    self.abilities.push(AbilityInfo {
                        name: ability.name.text.clone(),
                        module: id,
                        public: ability.public,
                        params: Vec::new(),
                        methods: Vec::new(),
                        span: ability.span,
                    });
                    if info
                        .abilities
                        .insert(ability.name.text.clone(), ability_id)
                        .is_some()
                    {
                        self.diagnostics.push((
                            id,
                            Diagnostic::error(
                                "duplicate-name",
                                format!("the ability `{}` is declared twice", ability.name.text),
                                ability.name.span,
                            ),
                        ));
                    }
                }
                _ => {}
            }
        }
        self.modules.push(info);
        id
    }

    pub fn module_id(&self, name: &str) -> Option<ModuleId> {
        self.modules.iter().position(|m| m.name == name)
    }

    /// Remember where the lines of a module's file start, so that the body
    /// checker can count lines.
    pub fn set_source_lines(&mut self, id: ModuleId, text: &str) {
        let mut starts = vec![0];
        starts.extend(
            text.bytes()
                .enumerate()
                .filter(|(_, byte)| *byte == b'\n')
                .map(|(offset, _)| offset + 1),
        );
        self.modules[id].line_starts = starts;
    }

    /// Resolve imports, type details, abilities, implementations, functions
    /// and constants of every module that is still unresolved.
    pub fn resolve_all(&mut self) {
        if self.builtins.integer == 0 && self.prelude.is_some() {
            self.find_builtins();
        }
        let modules: Vec<ModuleId> = (0..self.modules.len()).collect();
        for &id in &modules {
            self.resolve_imports(id);
        }
        for &id in &modules {
            self.resolve_types(id);
        }
        for &id in &modules {
            self.resolve_abilities(id);
        }
        for &id in &modules {
            self.resolve_functions(id);
        }
    }

    fn find_builtins(&mut self) {
        let prelude = self.prelude.expect("prelude");
        let types = &self.modules[prelude].types;
        let abilities = &self.modules[prelude].abilities;
        let get = |name: &str| {
            *types
                .get(name)
                .unwrap_or_else(|| panic!("prelude type {name}"))
        };
        let get_ability = |name: &str| {
            *abilities
                .get(name)
                .unwrap_or_else(|| panic!("prelude ability {name}"))
        };
        self.builtins = Builtins {
            integer: get("Integer"),
            decimal: get("Decimal"),
            float: get("Float"),
            boolean: get("Boolean"),
            text: get("Text"),
            bytes: get("Bytes"),
            list: get("List"),
            map: get("Map"),
            set: get("Set"),
            range: get("Range"),
            pair: get("Pair"),
            duration: get("Duration"),
            ordering: get("Ordering"),
            timed_out: get("TimedOut"),
            constraint_violation: get("ConstraintViolation"),
            guarded: get("Guarded"),
            equal: get_ability("Equal"),
            compare: get_ability("Compare"),
            hash: get_ability("Hash"),
            to_text: get_ability("ToText"),
        };
    }

    fn error(&mut self, module: ModuleId, code: &'static str, message: String, span: Span) {
        self.diagnostics
            .push((module, Diagnostic::error(code, message, span)));
    }

    fn error_with_fix(
        &mut self,
        module: ModuleId,
        code: &'static str,
        message: String,
        span: Span,
        fix: String,
    ) {
        self.diagnostics
            .push((module, Diagnostic::error(code, message, span).with_fix(fix)));
    }

    fn warning_with_fix(
        &mut self,
        module: ModuleId,
        code: &'static str,
        message: String,
        span: Span,
        fix: String,
    ) {
        self.diagnostics.push((
            module,
            Diagnostic::warning(code, message, span).with_fix(fix),
        ));
    }

    fn resolve_imports(&mut self, id: ModuleId) {
        if !self.modules[id].imports.is_empty() {
            return; // already resolved
        }
        let imports = self.modules[id].ast.imports.clone();
        for import in &imports {
            let path: Vec<&str> = import.path.iter().map(|n| n.text.as_str()).collect();
            let name = path.join(".");
            let Some(target) = self.module_id(&name) else {
                self.error(
                    id,
                    "unknown-module",
                    format!("no module named `{name}`"),
                    import.span,
                );
                continue;
            };
            let namespace = import
                .alias
                .as_ref()
                .map(|a| a.text.clone())
                .unwrap_or_else(|| path.last().unwrap().to_string());
            if self.modules[id]
                .imports
                .insert(namespace.clone(), target)
                .is_some()
            {
                self.error_with_fix(
                    id,
                    "duplicate-name",
                    format!("the namespace `{namespace}` is imported twice"),
                    import.span,
                    "rename one import with `as`".into(),
                );
            }
            for exposed in &import.exposing {
                let text = exposed.text.clone();
                if let Some(&type_id) = self.modules[target].types.get(&text) {
                    if !self.types[type_id].public {
                        self.error(
                            id,
                            "private-name",
                            format!("`{text}` is not public in `{name}`"),
                            exposed.span,
                        );
                    }
                    self.modules[id].exposed_types.insert(text, type_id);
                } else if let Some(&ability_id) = self.modules[target].abilities.get(&text) {
                    if !self.abilities[ability_id].public {
                        self.error(
                            id,
                            "private-name",
                            format!("`{text}` is not public in `{name}`"),
                            exposed.span,
                        );
                    }
                    self.modules[id].exposed_abilities.insert(text, ability_id);
                } else {
                    self.error(
                        id,
                        "unknown-name",
                        format!("`{name}` has no type or ability named `{text}`"),
                        exposed.span,
                    );
                }
            }
        }
    }

    // --------------------------------------------------------------- lookups

    pub fn lookup_type(&self, module: ModuleId, name: &str) -> Option<TypeId> {
        let info = &self.modules[module];
        if let Some(&id) = info.types.get(name) {
            return Some(id);
        }
        if let Some(&id) = info.exposed_types.get(name) {
            return Some(id);
        }
        self.prelude
            .and_then(|prelude| self.modules[prelude].types.get(name).copied())
    }

    pub fn lookup_ability(&self, module: ModuleId, name: &str) -> Option<AbilityId> {
        let info = &self.modules[module];
        if let Some(&id) = info.abilities.get(name) {
            return Some(id);
        }
        if let Some(&id) = info.exposed_abilities.get(name) {
            return Some(id);
        }
        self.prelude
            .and_then(|prelude| self.modules[prelude].abilities.get(name).copied())
    }

    /// Where a type or ability of that name could come from: a fix for an
    /// unknown name.
    pub fn suggest_import(&self, module: ModuleId, name: &str) -> Option<String> {
        let info = &self.modules[module];
        for (namespace, &target) in &info.imports {
            let other = &self.modules[target];
            if other.types.contains_key(name) || other.abilities.contains_key(name) {
                let _ = namespace;
                return Some(format!(
                    "add `{name}` to the `exposing` list of `import {}`",
                    other.name
                ));
            }
        }
        for other in &self.modules {
            if other.is_library
                && (other.types.contains_key(name) || other.abilities.contains_key(name))
            {
                return Some(format!("write `import {} exposing {name}`", other.name));
            }
        }
        None
    }

    /// The sum types in scope that have a variant of this name.
    pub fn lookup_variant(&self, module: ModuleId, name: &str) -> Vec<(TypeId, usize)> {
        let info = &self.modules[module];
        let mut candidates: Vec<TypeId> = info.types.values().copied().collect();
        candidates.extend(info.exposed_types.values().copied());
        if let Some(prelude) = self.prelude {
            candidates.extend(self.modules[prelude].types.values().copied());
        }
        candidates.sort_unstable();
        candidates.dedup();
        let mut found = Vec::new();
        for type_id in candidates {
            if let TypeKindInfo::Sum(variants) = &self.types[type_id].kind {
                if let Some(index) = variants.iter().position(|v| v.name == name) {
                    found.push((type_id, index));
                }
            }
        }
        found
    }

    /// The module's own functions of that name that are not methods.
    pub fn lookup_function(&self, module: ModuleId, name: &str) -> Option<FunctionId> {
        self.modules[module]
            .functions
            .get(name)?
            .iter()
            .copied()
            .find(|&id| !self.functions[id].is_method)
    }

    /// The `purpose:` clause of a declared function, read from its item.
    pub fn function_purpose(&self, id: FunctionId) -> Option<String> {
        let info = &self.functions[id];
        let items = &self.modules[info.module].ast.items;
        match info.body {
            BodyLocation::Item(item) => match items.get(item) {
                Some(Item::Function(function)) => function.docs.purpose.clone(),
                _ => None,
            },
            BodyLocation::Implementation(item, index) => match items.get(item) {
                Some(Item::Implementation(implementation)) => implementation
                    .functions
                    .get(index)
                    .and_then(|function| function.docs.purpose.clone()),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn methods_of(&self, type_id: TypeId, name: &str) -> Vec<FunctionId> {
        self.method_index
            .get(&(type_id, name.to_string()))
            .cloned()
            .unwrap_or_default()
    }

    pub fn type_name(&self, id: TypeId) -> String {
        self.types[id].name.clone()
    }

    pub fn param_name(&self, id: ParamId) -> String {
        self.params[id].name.clone()
    }

    pub fn show(&self, ty: &Ty) -> String {
        let type_name = |id: TypeId| self.type_name(id);
        let param_name = |id: ParamId| self.param_name(id);
        TyDisplay {
            ty,
            type_name: &type_name,
            param_name: &param_name,
        }
        .to_string()
    }

    /// The abilities a type parameter must have.
    pub fn constraints(&self, param: ParamId) -> &[AbilityId] {
        &self.params[param].constraints
    }

    // ------------------------------------------------------------ resolution

    /// Turn a syntactic type into a `Ty` in the scope of a module and a list
    /// of type parameters in scope.
    pub fn resolve_type(
        &mut self,
        module: ModuleId,
        params: &[(String, ParamId)],
        ty: &ast::Type,
    ) -> Ty {
        match ty {
            ast::Type::Maybe(inner, _) => Ty::maybe(self.resolve_type(module, params, inner)),
            ast::Type::Function {
                params: param_types,
                returns,
                fails,
                needs,
                ..
            } => {
                let function = FunctionTy {
                    params: param_types
                        .iter()
                        .map(|p| self.resolve_type(module, params, p))
                        .collect(),
                    returns: returns
                        .as_ref()
                        .map(|r| self.resolve_type(module, params, r)),
                    fails: fails
                        .iter()
                        .map(|f| self.resolve_type(module, params, f))
                        .collect(),
                    needs: needs.iter().map(Capability::from_ast).collect(),
                };
                Ty::Function(Box::new(function))
            }
            ast::Type::Named { name, args, span } => {
                if name.text == "Self" {
                    return Ty::SelfType;
                }
                if let Some((_, id)) = params.iter().find(|(n, _)| *n == name.text) {
                    if !args.is_empty() {
                        self.error(
                            module,
                            "type-arity",
                            format!("`{}` is a type parameter and takes no arguments", name.text),
                            *span,
                        );
                    }
                    return Ty::Param(*id);
                }
                let Some(type_id) = self.lookup_type(module, &name.text) else {
                    let fix = self.suggest_import(module, &name.text);
                    let message = format!("unknown type `{}`", name.text);
                    match fix {
                        Some(fix) => {
                            self.error_with_fix(module, "unknown-type", message, name.span, fix)
                        }
                        None => self.error(module, "unknown-type", message, name.span),
                    }
                    return Ty::Error;
                };
                let expected = self.types[type_id].params.len();
                if expected != args.len() {
                    let message = if expected == 0 {
                        format!("`{}` takes no type arguments", name.text)
                    } else {
                        format!(
                            "`{}` takes {expected} type argument{}, found {}",
                            name.text,
                            if expected == 1 { "" } else { "s" },
                            args.len()
                        )
                    };
                    self.error(module, "type-arity", message, *span);
                    return Ty::Error;
                }
                let args = args
                    .iter()
                    .map(|a| self.resolve_type(module, params, a))
                    .collect();
                Ty::App(type_id, args)
            }
        }
    }

    fn new_params(&mut self, names: &[ast::TypeName]) -> Vec<(String, ParamId)> {
        names
            .iter()
            .map(|name| {
                let id = self.params.len();
                self.params.push(ParamInfo {
                    name: name.text.clone(),
                    constraints: Vec::new(),
                });
                (name.text.clone(), id)
            })
            .collect()
    }

    /// Type parameters of a `for any` clause with their constraints.
    fn for_any_params(
        &mut self,
        module: ModuleId,
        for_any: &ast::ForAny,
    ) -> Vec<(String, ParamId)> {
        let params = self.new_params(&for_any.params);
        for constraint in &for_any.constraints {
            let Some((_, param)) = params.iter().find(|(n, _)| *n == constraint.param.text) else {
                self.error(
                    module,
                    "unknown-type",
                    format!(
                        "`{}` is not one of the type parameters",
                        constraint.param.text
                    ),
                    constraint.param.span,
                );
                continue;
            };
            let param = *param;
            if let ast::Type::Named { name, .. } = &constraint.ability {
                match self.lookup_ability(module, &name.text) {
                    Some(ability) => self.params[param].constraints.push(ability),
                    None => {
                        let message = format!("unknown ability `{}`", name.text);
                        match self.suggest_import(module, &name.text) {
                            Some(fix) => self.error_with_fix(
                                module,
                                "unknown-ability",
                                message,
                                name.span,
                                fix,
                            ),
                            None => self.error(module, "unknown-ability", message, name.span),
                        }
                    }
                }
            }
        }
        params
    }

    fn resolve_fields(
        &mut self,
        module: ModuleId,
        params: &[(String, ParamId)],
        fields: &[ast::Field],
    ) -> Vec<FieldInfo> {
        fields
            .iter()
            .map(|field| FieldInfo {
                name: field.name.text.clone(),
                ty: self.resolve_type(module, params, &field.ty),
                refinement: field.refinement.clone(),
                external_name: field.external_name.clone(),
                span: field.span,
            })
            .collect()
    }

    fn resolve_derives(
        &mut self,
        module: ModuleId,
        derives: &[ast::Derive],
        type_id: TypeId,
    ) -> Vec<AbilityId> {
        let mut out = Vec::new();
        for derive in derives {
            match self.lookup_ability(module, &derive.ability.text) {
                Some(ability) => {
                    out.push(ability);
                    if !derive.by.is_empty() {
                        // `can Compare by field, field`: every field must exist
                        if let TypeKindInfo::Record(fields) = &self.types[type_id].kind {
                            let missing: Vec<&ast::Name> = derive
                                .by
                                .iter()
                                .filter(|name| !fields.iter().any(|f| f.name == name.text))
                                .collect();
                            for name in missing {
                                let message = format!(
                                    "`{}` has no field named `{}`",
                                    self.types[type_id].name, name.text
                                );
                                self.error(module, "unknown-field", message, name.span);
                            }
                        }
                    }
                }
                None => {
                    let message = format!("unknown ability `{}`", derive.ability.text);
                    match self.suggest_import(module, &derive.ability.text) {
                        Some(fix) => self.error_with_fix(
                            module,
                            "unknown-ability",
                            message,
                            derive.ability.span,
                            fix,
                        ),
                        None => self.error(module, "unknown-ability", message, derive.ability.span),
                    }
                }
            }
        }
        out
    }

    fn resolve_types(&mut self, id: ModuleId) {
        let items = self.modules[id].ast.items.clone();
        for item in &items {
            let Item::Type(def) = item else { continue };
            let type_id = self.modules[id].types[&def.name.text];
            if !matches!(self.types[type_id].kind, TypeKindInfo::Unresolved) {
                continue;
            }
            let params = self.new_params(&def.type_params);
            self.types[type_id].params = params.iter().map(|(_, p)| *p).collect();
            let is_library = self.modules[id].is_library;
            let kind = match &def.kind {
                TypeKind::Record { fields, .. } if fields.is_empty() && is_library => {
                    TypeKindInfo::Opaque
                }
                TypeKind::Record { fields, .. } => {
                    TypeKindInfo::Record(self.resolve_fields(id, &params, fields))
                }
                TypeKind::Sum { variants, .. } => TypeKindInfo::Sum(
                    variants
                        .iter()
                        .map(|variant| VariantInfo {
                            name: variant.name.text.clone(),
                            fields: self.resolve_fields(id, &params, &variant.fields),
                            span: variant.span,
                        })
                        .collect(),
                ),
                TypeKind::Subtype { base, refinement } => TypeKindInfo::Subtype {
                    base: self.resolve_type(id, &params, base),
                    refinement: refinement.clone(),
                },
            };
            self.types[type_id].kind = kind;
            let derives = match &def.kind {
                TypeKind::Record { derives, .. } | TypeKind::Sum { derives, .. } => {
                    self.resolve_derives(id, derives, type_id)
                }
                TypeKind::Subtype { .. } => Vec::new(),
            };
            self.types[type_id].derives = derives;
            if def.public && def.docs.purpose.is_none() && !is_library {
                self.error_with_fix(
                    id,
                    "purpose-missing",
                    format!("the public type `{}` has no purpose", def.name.text),
                    def.name.span,
                    "add a `purpose:` clause after the head line".into(),
                );
            }
        }
    }

    fn resolve_abilities(&mut self, id: ModuleId) {
        let items = self.modules[id].ast.items.clone();
        for item in &items {
            let Item::Ability(ability) = item else {
                continue;
            };
            let ability_id = self.modules[id].abilities[&ability.name.text];
            if !self.abilities[ability_id].methods.is_empty() {
                continue;
            }
            let params = self.new_params(&ability.type_params);
            let mut methods = Vec::new();
            for function in &ability.functions {
                let mut method_params = Vec::new();
                for (index, param) in function.params.iter().enumerate() {
                    if index == 0 && param.name.text == "self" {
                        continue;
                    }
                    let ty = match &param.ty {
                        Some(ty) => self.resolve_type(id, &params, ty),
                        None => Ty::Error,
                    };
                    method_params.push((param.name.text.clone(), ty));
                }
                methods.push(AbilityMethod {
                    name: function.name.text.clone(),
                    params: method_params,
                    returns: function
                        .returns
                        .as_ref()
                        .map(|r| self.resolve_type(id, &params, r)),
                    fails: function
                        .fails
                        .iter()
                        .map(|f| self.resolve_type(id, &params, f))
                        .collect(),
                });
            }
            self.abilities[ability_id].params = params.iter().map(|(_, p)| *p).collect();
            self.abilities[ability_id].methods = methods;
            if ability.public && ability.docs.purpose.is_none() && !self.modules[id].is_library {
                self.error_with_fix(
                    id,
                    "purpose-missing",
                    format!("the public ability `{}` has no purpose", ability.name.text),
                    ability.name.span,
                    "add a `purpose:` clause after the head line".into(),
                );
            }
        }
    }

    fn resolve_functions(&mut self, id: ModuleId) {
        let items = self.modules[id].ast.items.clone();
        let is_library = self.modules[id].is_library;
        for (index, item) in items.iter().enumerate() {
            match item {
                Item::Function(function) => {
                    if self.function_registered(id, function) {
                        continue;
                    }
                    self.declare_function(id, function, &[], BodyLocation::Item(index), is_library);
                }
                Item::Implementation(implementation) => {
                    let Some(ability_name) = named(&implementation.ability) else {
                        continue;
                    };
                    let Some(ability) = self.lookup_ability(id, &ability_name.text) else {
                        let message = format!("unknown ability `{}`", ability_name.text);
                        match self.suggest_import(id, &ability_name.text) {
                            Some(fix) => self.error_with_fix(
                                id,
                                "unknown-ability",
                                message,
                                ability_name.span,
                                fix,
                            ),
                            None => self.error(id, "unknown-ability", message, ability_name.span),
                        }
                        continue;
                    };
                    if self
                        .impls
                        .iter()
                        .any(|i| i.module == id && i.span == implementation.span)
                    {
                        continue;
                    }
                    let params = match &implementation.type_params {
                        Some(for_any) => self.for_any_params(id, for_any),
                        None => Vec::new(),
                    };
                    let target = self.resolve_type(id, &params, &implementation.target);
                    let mut functions = Vec::new();
                    for (function_index, function) in implementation.functions.iter().enumerate() {
                        let function_id = self.declare_function(
                            id,
                            function,
                            &params,
                            BodyLocation::Implementation(index, function_index),
                            is_library,
                        );
                        // the method's `self` is the implementation's target
                        if let Some(function_id) = function_id {
                            if let Some(first) = self.functions[function_id].params.first_mut() {
                                if first.0 == "self" {
                                    first.1 = target.clone();
                                }
                            }
                            self.functions[function_id].is_method = true;
                            if let Some(head) = head_type(&target) {
                                self.method_index
                                    .entry((head, function.name.text.clone()))
                                    .or_default()
                                    .push(function_id);
                            }
                            self.check_method_signature(
                                id,
                                ability,
                                &target,
                                function_id,
                                function,
                            );
                            functions.push(function_id);
                        }
                    }
                    // every method of the ability must be implemented, and nothing else
                    let expected: Vec<String> = self.abilities[ability]
                        .methods
                        .iter()
                        .map(|m| m.name.clone())
                        .collect();
                    for name in &expected {
                        if !implementation
                            .functions
                            .iter()
                            .any(|f| &f.name.text == name)
                        {
                            let message = format!(
                                "the implementation of `{}` lacks `{name}`",
                                ability_name.text
                            );
                            self.error(id, "missing-method", message, implementation.span);
                        }
                    }
                    for function in &implementation.functions {
                        if !expected.contains(&function.name.text) {
                            let message = format!(
                                "`{}` is not a method of `{}`",
                                function.name.text, ability_name.text
                            );
                            self.error(id, "unknown-method", message, function.name.span);
                        }
                    }
                    self.impls.push(ImplInfo {
                        ability,
                        target,
                        type_params: params.iter().map(|(_, p)| *p).collect(),
                        module: id,
                        functions,
                        span: implementation.span,
                    });
                }
                Item::Constant(constant) => {
                    if self.modules[id].constants.contains_key(&constant.name.text) {
                        continue;
                    }
                    let ty = self.resolve_type(id, &[], &constant.ty);
                    if constant.public && constant.docs.purpose.is_none() && !is_library {
                        self.error_with_fix(
                            id,
                            "purpose-missing",
                            format!(
                                "the public constant `{}` has no purpose",
                                constant.name.text
                            ),
                            constant.name.span,
                            "add a `purpose:` clause".into(),
                        );
                    }
                    self.modules[id].constants.insert(
                        constant.name.text.clone(),
                        ConstantInfo {
                            ty,
                            public: constant.public,
                            item_index: index,
                            deprecated: constant.docs.deprecated.clone(),
                            span: constant.span,
                        },
                    );
                }
                _ => {}
            }
        }
    }

    fn function_registered(&self, module: ModuleId, function: &ast::Function) -> bool {
        self.modules[module]
            .functions
            .get(&function.name.text)
            .is_some_and(|ids| {
                ids.iter()
                    .any(|&id| self.functions[id].span == function.span)
            })
    }

    fn declare_function(
        &mut self,
        module: ModuleId,
        function: &ast::Function,
        outer_params: &[(String, ParamId)],
        body: BodyLocation,
        is_library: bool,
    ) -> Option<FunctionId> {
        let mut params = outer_params.to_vec();
        if let Some(for_any) = &function.type_params {
            params.extend(self.for_any_params(module, for_any));
        }
        let mut param_types = Vec::new();
        let mut names: Vec<&str> = Vec::new();
        for (index, param) in function.params.iter().enumerate() {
            if names.contains(&param.name.text.as_str()) {
                self.error(
                    module,
                    "duplicate-name",
                    format!("the parameter `{}` is declared twice", param.name.text),
                    param.name.span,
                );
            }
            names.push(&param.name.text);
            let ty = match &param.ty {
                Some(ty) => self.resolve_type(module, &params, ty),
                None if index == 0 && param.name.text == "self" => Ty::SelfType,
                None => Ty::Error,
            };
            if index > 0 && param.name.text == "self" {
                self.error(
                    module,
                    "self-position",
                    "`self` must be the first parameter".into(),
                    param.name.span,
                );
            }
            param_types.push((param.name.text.clone(), ty));
        }
        let is_method = function
            .params
            .first()
            .is_some_and(|p| p.name.text == "self");
        let returns = function
            .returns
            .as_ref()
            .map(|r| self.resolve_type(module, &params, r));
        let fails = function
            .fails
            .iter()
            .map(|f| self.resolve_type(module, &params, f))
            .collect();
        let needs: Vec<Capability> = function.needs.iter().map(Capability::from_ast).collect();
        for (capability, syntax) in needs.iter().zip(&function.needs) {
            // decision V6: `process` and `foreign` wait for the package manager
            let unavailable = std::iter::once(&capability.path)
                .chain(capability.only_to.iter().map(|(path, _)| path))
                .find(|path| crate::effects::unavailable(path));
            if let Some(path) = unavailable {
                self.error_with_fix(
                    module,
                    "capability-unavailable",
                    crate::effects::unavailable_message(path),
                    syntax.span,
                    "remove it from `needs`".into(),
                );
                continue;
            }
            if capability.has_grant_clauses() && function.name.text != "main" {
                self.error_with_fix(
                    module,
                    "grant-clause",
                    "a budget (`at most`) or a guard (`only to`) belongs to the program's grant"
                        .into(),
                    syntax.span,
                    "move it to the `needs` of `main` or of a `test`".into(),
                );
            }
            for (sink_path, _) in &capability.only_to {
                let sink = Capability {
                    path: sink_path.clone(),
                    scope: None,
                    budget: None,
                    only_to: Vec::new(),
                };
                if !sink.is_known() {
                    self.error(
                        module,
                        "unknown-capability",
                        format!(
                            "unknown capability `{}` after `only to`",
                            sink_path.join(".")
                        ),
                        syntax.span,
                    );
                }
            }
            if capability.budget.is_some() {
                let budgeted = matches!(
                    capability.path.first().map(String::as_str),
                    Some("network") | Some("process") | Some("filesystem")
                );
                if !budgeted {
                    self.error(module, "grant-clause", format!("`{}` takes no budget; budgets apply to network, process and filesystem", capability.path.join(".")), syntax.span);
                }
            }
            // a guard whose sinks the grant itself never allows keeps the
            // data from ever leaving (06-runtime-guarantees.md section 3.3)
            if !capability.only_to.is_empty() && function.name.text == "main" {
                let reachable = capability.only_to.iter().any(|(path, scope)| {
                    let sink = Capability {
                        path: path.clone(),
                        scope: scope.clone(),
                        budget: None,
                        only_to: Vec::new(),
                    };
                    needs
                        .iter()
                        .any(|granted| granted.covers(&sink, true) || sink.covers(granted, true))
                });
                if !reachable {
                    self.warning_with_fix(
                        module,
                        "guard-no-sink",
                        format!(
                            "no sink after `only to` is in the grant, so what enters through `{}` can never leave",
                            capability.spelling()
                        ),
                        syntax.span,
                        "add the sink to `needs`, or drop `only to` if nothing should leave".into(),
                    );
                }
            }
            if !capability.is_known() {
                let message = format!("unknown capability `{}`", capability.path.join("."));
                self.error_with_fix(
                    module,
                    "unknown-capability",
                    message,
                    syntax.span,
                    format!("one of {}", crate::effects::TREE.join(", ")),
                );
            } else if capability.scope.is_some() && !crate::effects::takes_scope(&capability.path) {
                self.error(
                    module,
                    "capability-scope",
                    format!("`{}` takes no scope argument", capability.path.join(".")),
                    syntax.span,
                );
            }
        }
        if function.public
            && function.docs.purpose.is_none()
            && !is_library
            && matches!(body, BodyLocation::Item(_))
        {
            self.error_with_fix(
                module,
                "purpose-missing",
                format!(
                    "the public function `{}` has no purpose",
                    function.name.text
                ),
                function.name.span,
                "add a `purpose:` clause after the signature".into(),
            );
        }
        if !function.docs.examples.is_empty() && !needs.is_empty() {
            self.error_with_fix(
                module,
                "example-effects",
                format!(
                    "`{}` has `example:` lines but needs {}; examples run only on pure functions",
                    function.name.text,
                    needs
                        .iter()
                        .map(Capability::spelling)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                function.name.span,
                "move the checks into a `test` block that declares its `needs`".into(),
            );
        }
        let id = self.functions.len();
        self.functions.push(FunctionInfo {
            name: function.name.text.clone(),
            module,
            public: function.public,
            params: param_types,
            is_method,
            type_params: params.iter().map(|(_, p)| *p).collect(),
            returns,
            fails,
            needs,
            is_library,
            body,
            has_examples: !function.docs.examples.is_empty(),
            deprecated: function.docs.deprecated.clone(),
            expose_as_tool: function.docs.expose_as_tool,
            span: function.span,
        });
        if is_method {
            if let Some(head) = head_type(&self.functions[id].params[0].1) {
                // decision K1: a method is declared in the module of its type;
                // the methods of an implementation answer to the ability instead
                if matches!(body, BodyLocation::Item(_)) && self.types[head].module != module {
                    let type_name = self.types[head].name.clone();
                    let owner = self.modules[self.types[head].module].name.clone();
                    self.error_with_fix(
                        module,
                        "method-module",
                        format!(
                            "`{}` is a method of `{type_name}`, which `{owner}` declares; a method is declared in the module of its type",
                            function.name.text
                        ),
                        function.params[0].span,
                        format!(
                            "make it a function whose first parameter is `{}: {type_name}`",
                            type_name.to_lowercase()
                        ),
                    );
                }
                self.method_index
                    .entry((head, function.name.text.clone()))
                    .or_default()
                    .push(id);
            }
        } else if let Some(existing) = self.lookup_function(module, &function.name.text) {
            let _ = existing;
            self.error(
                module,
                "duplicate-name",
                format!("the function `{}` is declared twice", function.name.text),
                function.name.span,
            );
        }
        self.modules[module]
            .functions
            .entry(function.name.text.clone())
            .or_default()
            .push(id);
        Some(id)
    }

    /// A method of an implementation carries the ability's signature: the
    /// same parameters by name and type, the same result and the same
    /// failures, with `Self` read as the target type.
    fn check_method_signature(
        &mut self,
        module: ModuleId,
        ability: AbilityId,
        target: &Ty,
        function_id: FunctionId,
        function: &ast::Function,
    ) {
        let ability_name = self.abilities[ability].name.clone();
        let Some(method) = self.abilities[ability]
            .methods
            .iter()
            .find(|m| m.name == function.name.text)
        else {
            return;
        };
        let expected: Vec<(String, Ty)> = method
            .params
            .iter()
            .map(|(n, t)| (n.clone(), t.with_self(target)))
            .collect();
        let expected_returns = method.returns.as_ref().map(|r| r.with_self(target));
        let expected_fails: Vec<Ty> = method.fails.iter().map(|f| f.with_self(target)).collect();
        let info = &self.functions[function_id];
        let skip = usize::from(
            function
                .params
                .first()
                .is_some_and(|p| p.name.text == "self"),
        );
        let actual: Vec<(String, Ty)> = info
            .params
            .iter()
            .skip(skip)
            .map(|(n, t)| (n.clone(), t.with_self(target)))
            .collect();
        let actual_returns = info.returns.as_ref().map(|r| r.with_self(target));
        let actual_fails: Vec<Ty> = info.fails.iter().map(|f| f.with_self(target)).collect();
        let mut problems = Vec::new();
        if actual.len() != expected.len() {
            problems.push(format!(
                "it takes {} parameter{} after `self`; the ability declares {}",
                actual.len(),
                if actual.len() == 1 { "" } else { "s" },
                expected.len()
            ));
        } else {
            for ((name, ty), (expected_name, expected_ty)) in actual.iter().zip(&expected) {
                if name != expected_name || !conforms(expected_ty, ty) {
                    problems.push(format!(
                        "its parameter `{name}: {}` should be `{expected_name}: {}`",
                        self.show(ty),
                        self.show(expected_ty)
                    ));
                }
            }
        }
        let returns_ok = match (&actual_returns, &expected_returns) {
            (Some(a), Some(e)) => conforms(e, a),
            (None, None) => true,
            _ => false,
        };
        if !returns_ok {
            problems.push(format!(
                "it returns {}; the ability declares {}",
                self.spell_result(&actual_returns),
                self.spell_result(&expected_returns)
            ));
        }
        let fails_ok = actual_fails.len() == expected_fails.len()
            && actual_fails
                .iter()
                .all(|a| expected_fails.iter().any(|e| conforms(e, a)));
        if !fails_ok {
            problems.push(format!(
                "it fails with {}; the ability declares {}",
                self.spell_fails(&actual_fails),
                self.spell_fails(&expected_fails)
            ));
        }
        if problems.is_empty() {
            return;
        }
        let mut signature = format!("{}(self", function.name.text);
        for (name, ty) in &expected {
            signature.push_str(&format!(", {name}: {}", self.show(ty)));
        }
        signature.push(')');
        if let Some(returns) = &expected_returns {
            signature.push_str(&format!(" returns {}", self.show(returns)));
        }
        if !expected_fails.is_empty() {
            let fails: Vec<String> = expected_fails.iter().map(|f| self.show(f)).collect();
            signature.push_str(&format!(" or fails with {}", fails.join(" or ")));
        }
        self.error_with_fix(
            module,
            "method-signature",
            format!(
                "`{}` does not match `{ability_name}.{}`: {}",
                function.name.text,
                function.name.text,
                problems.join("; ")
            ),
            function.name.span,
            format!("declare it as `{signature}`"),
        );
    }

    fn spell_result(&self, ty: &Option<Ty>) -> String {
        match ty {
            Some(ty) => format!("`{}`", self.show(ty)),
            None => "nothing".to_string(),
        }
    }

    fn spell_fails(&self, fails: &[Ty]) -> String {
        if fails.is_empty() {
            return "nothing".to_string();
        }
        fails
            .iter()
            .map(|f| format!("`{}`", self.show(f)))
            .collect::<Vec<_>>()
            .join(" or ")
    }
}

/// Whether an implementation's type carries the ability's: the same shape,
/// with a type parameter on either side standing for anything.
fn conforms(expected: &Ty, actual: &Ty) -> bool {
    match (expected, actual) {
        (Ty::Param(_), _) | (_, Ty::Param(_)) | (Ty::Error, _) | (_, Ty::Error) => true,
        (Ty::App(a, x), Ty::App(b, y)) => {
            a == b && x.len() == y.len() && x.iter().zip(y).all(|(p, q)| conforms(p, q))
        }
        (Ty::Maybe(a), Ty::Maybe(b)) => conforms(a, b),
        (Ty::Function(a), Ty::Function(b)) => {
            a.params.len() == b.params.len()
                && a.params.iter().zip(&b.params).all(|(p, q)| conforms(p, q))
                && match (&a.returns, &b.returns) {
                    (Some(p), Some(q)) => conforms(p, q),
                    (None, None) => true,
                    _ => false,
                }
                && a.fails.len() == b.fails.len()
                && a.fails
                    .iter()
                    .all(|p| b.fails.iter().any(|q| conforms(p, q)))
        }
        (Ty::Union(a), Ty::Union(b)) => {
            a.len() == b.len() && a.iter().all(|p| b.iter().any(|q| conforms(p, q)))
        }
        _ => expected == actual,
    }
}

fn named(ty: &ast::Type) -> Option<&ast::TypeName> {
    match ty {
        ast::Type::Named { name, .. } => Some(name),
        _ => None,
    }
}

/// The nominal type at the head of a type, for method lookup.
pub fn head_type(ty: &Ty) -> Option<TypeId> {
    match ty {
        Ty::App(id, _) => Some(*id),
        _ => None,
    }
}
