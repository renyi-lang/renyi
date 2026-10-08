//! Declarations: every module's types, abilities, implementations, functions
//! and constants, resolved into the checker's tables. The library modules and
//! the user's modules go through the same pass; bodies are checked afterwards
//! by `check`.

use std::collections::HashMap;

use renyi_syntax::ast::{self, Item, TypeKind};
use renyi_syntax::{Diagnostic, ForeignModule, Package, PythonBinding, Span};

use crate::effects::Capability;
use crate::suggest::{closest, foreign_type, quoted};
use crate::types::*;

pub struct ModuleInfo {
    /// The dotted module name, `std.json` or `invoice`; for a module of a
    /// dependency the package name and the path from the package's root,
    /// `greeting.words`, or the package name alone for its root module.
    pub name: String,
    pub is_library: bool,
    /// The dependency the module belongs to (decision AC1).
    pub package: Option<Package>,
    /// The foreign module the file declares (decision AF1).
    pub foreign: Option<ForeignModule>,
    /// The Python module the file declares (decision AL1).
    pub python: Option<PythonBinding>,
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
    /// The module's source text, for fixes that quote it; empty for a
    /// library module.
    pub source: String,
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
    /// `ability X where self can Y`: what every implementing type must have,
    /// with the ability's own parameters in scope for the arguments.
    pub requirements: Vec<AbilityRef>,
    pub methods: Vec<AbilityMethod>,
    pub span: Span,
    /// Whether `resolve_abilities` has given it its parameters.
    pub resolved: bool,
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
    /// The ability's type arguments: `ability Iterable of Card for Deck`
    /// gives `[Card]`.
    pub ability_args: Vec<Ty>,
    pub type_params: Vec<ParamId>,
    pub module: ModuleId,
    pub functions: Vec<FunctionId>,
    pub span: Span,
}

/// An ability with its type arguments, as a constraint names one (`Bag can
/// Iterable of Item`) or a requirement does (`self can Iterable of Item`),
/// decision AB1. An ability without parameters has no arguments.
#[derive(Clone, Debug, PartialEq)]
pub struct AbilityRef {
    pub ability: AbilityId,
    pub args: Vec<Ty>,
}

pub struct ParamInfo {
    pub name: String,
    pub constraints: Vec<AbilityRef>,
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
    /// `Iterable of Item`: what `for each` walks besides the collections
    /// (decision V10).
    pub iterable: AbilityId,
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
            package: None,
            foreign: None,
            python: None,
            ast: module,
            types: HashMap::new(),
            abilities: HashMap::new(),
            functions: HashMap::new(),
            constants: HashMap::new(),
            imports: HashMap::new(),
            exposed_types: HashMap::new(),
            exposed_abilities: HashMap::new(),
            line_starts: Vec::new(),
            source: String::new(),
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
                        requirements: Vec::new(),
                        resolved: false,
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

    /// Tag a module as a dependency's (decision AC1) and name it as the
    /// program imports it: the package name and the declared name, or the
    /// package name alone for the root module.
    pub fn set_package(&mut self, id: ModuleId, package: Package) {
        if self.modules[id].name != package.name {
            self.modules[id].name = format!("{}.{}", package.name, self.modules[id].name);
        }
        self.modules[id].package = Some(package);
    }

    pub fn set_foreign(&mut self, id: ModuleId, foreign: ForeignModule) {
        self.modules[id].foreign = Some(foreign);
    }

    pub fn set_python(&mut self, id: ModuleId, python: PythonBinding) {
        self.modules[id].python = Some(python);
    }

    /// The module an import names, from a module: inside a dependency an
    /// own module first (`import words` is `greeting.words`), then the
    /// name as written.
    fn imported_module(&self, from: ModuleId, name: &str) -> Option<ModuleId> {
        if let Some(package) = &self.modules[from].package {
            if let Some(id) = self.module_id(&format!("{}.{name}", package.name)) {
                return Some(id);
            }
        }
        self.module_id(name)
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
        self.modules[id].source = text.to_string();
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
        self.check_requirements();
        for &id in &modules {
            self.check_see_also(id);
        }
    }

    /// `ability X where self can Y`: every implementation's type has `Y`
    /// (sketch section 5), checked once every implementation is known.
    fn check_requirements(&mut self) {
        let mut problems = Vec::new();
        for implementation in &self.impls {
            let ability = &self.abilities[implementation.ability];
            for required in &ability.requirements {
                // the requirement's arguments, with the ability's parameters
                // read as this implementation's arguments (decision AB1)
                let expected: Vec<Ty> = required
                    .args
                    .iter()
                    .map(|arg| {
                        arg.substitute(&|p: ParamId| {
                            ability
                                .params
                                .iter()
                                .position(|&q| q == p)
                                .and_then(|index| implementation.ability_args.get(index).cloned())
                        })
                    })
                    .collect();
                let has = self.implemented_args(&implementation.target, required.ability)
                    == Some(expected.clone());
                if !has {
                    problems.push((
                        implementation.module,
                        implementation.span,
                        self.show(&implementation.target),
                        ability.name.clone(),
                        self.show_ability(required.ability, &expected),
                        expected.is_empty(),
                    ));
                }
            }
        }
        for (module, span, target, ability, required, derivable) in problems {
            let fix = if derivable {
                format!("add `can {required}` to `{target}`, or implement `{required}` for it")
            } else {
                format!("implement `{required}` for `{target}`")
            };
            self.error_with_fix(
                module,
                "missing-ability",
                format!(
                    "`{target}` lacks `{required}`, which `{ability}` requires of its implementations"
                ),
                span,
                fix,
            );
        }
    }

    /// Whether a declared type has an ability: derived, implemented, or
    /// inherited from the base of a subtype; a type parameter has what its
    /// constraints give it. `Equal` belongs to every type but functions.
    pub fn type_has(&self, ty: &Ty, ability: AbilityId) -> bool {
        if ability == self.builtins.equal {
            return !matches!(ty, Ty::Function(_));
        }
        match ty {
            Ty::App(id, _) => {
                let info = &self.types[*id];
                info.derives.contains(&ability)
                    || self
                        .impls
                        .iter()
                        .any(|i| i.ability == ability && head_type(&i.target) == Some(*id))
                    || matches!(&info.kind, TypeKindInfo::Subtype { base, .. } if self.type_has(base, ability))
            }
            Ty::Param(id) => self.params[*id]
                .constraints
                .iter()
                .any(|constraint| constraint.ability == ability),
            _ => false,
        }
    }

    /// The type arguments with which a type has an ability (decision AB1):
    /// those of its implementation, with the implementation's parameters
    /// read as the type's arguments (`Iterable of Item for Stack of Item`
    /// gives `Stack of Text` the arguments `[Text]`); none for a derived
    /// ability; a type parameter's from its constraints, or from what a
    /// constraint's ability requires; a subtype's from its base. `None`
    /// when the type lacks the ability.
    pub fn implemented_args(&self, ty: &Ty, ability: AbilityId) -> Option<Vec<Ty>> {
        if ability == self.builtins.equal {
            return (!matches!(ty, Ty::Function(_))).then(Vec::new);
        }
        match ty {
            Ty::App(id, args) => {
                let info = &self.types[*id];
                if info.derives.contains(&ability) {
                    return Some(Vec::new());
                }
                let found = self
                    .impls
                    .iter()
                    .find(|i| i.ability == ability && head_type(&i.target) == Some(*id));
                if let Some(implementation) = found {
                    let bound: Vec<(ParamId, Ty)> = match &implementation.target {
                        Ty::App(_, declared) => declared
                            .iter()
                            .zip(args)
                            .filter_map(|(declared, actual)| match declared {
                                Ty::Param(param) => Some((*param, actual.clone())),
                                _ => None,
                            })
                            .collect(),
                        _ => Vec::new(),
                    };
                    let substituted = implementation
                        .ability_args
                        .iter()
                        .map(|arg| {
                            arg.substitute(&|p: ParamId| {
                                bound.iter().find(|(q, _)| *q == p).map(|(_, t)| t.clone())
                            })
                        })
                        .collect();
                    return Some(substituted);
                }
                match &info.kind {
                    TypeKindInfo::Subtype { base, .. } => self.implemented_args(base, ability),
                    _ => None,
                }
            }
            Ty::Param(id) => {
                for constraint in &self.params[*id].constraints {
                    if constraint.ability == ability {
                        return Some(constraint.args.clone());
                    }
                    let through = &self.abilities[constraint.ability];
                    if let Some(requirement) =
                        through.requirements.iter().find(|r| r.ability == ability)
                    {
                        let substituted = requirement
                            .args
                            .iter()
                            .map(|arg| {
                                arg.substitute(&|p: ParamId| {
                                    through
                                        .params
                                        .iter()
                                        .position(|&q| q == p)
                                        .and_then(|index| constraint.args.get(index).cloned())
                                })
                            })
                            .collect();
                        return Some(substituted);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// An ability with its type arguments, as a message shows it:
    /// `Iterable of Integer`.
    pub fn show_ability(&self, ability: AbilityId, args: &[Ty]) -> String {
        let name = &self.abilities[ability].name;
        if args.is_empty() {
            return name.clone();
        }
        let shown: Vec<String> = args.iter().map(|arg| self.show(arg)).collect();
        format!("{name} of {}", shown.join(", "))
    }

    /// `type-arity` when an ability is named with the wrong number of type
    /// arguments; `before` and `after` surround the ability in the fix
    /// (`ability ... for ...`, `Bag can ...`, `self can ...`). Whether the
    /// count is right.
    fn check_ability_arity(
        &mut self,
        module: ModuleId,
        ability: AbilityId,
        found: usize,
        span: Span,
        before: &str,
        after: &str,
    ) -> bool {
        let expected = self.abilities[ability].params.len();
        if found == expected {
            return true;
        }
        let name = self.abilities[ability].name.clone();
        let names: Vec<String> = self.abilities[ability]
            .params
            .iter()
            .map(|&p| self.param_name(p))
            .collect();
        let fix = if expected == 0 {
            format!("write `{before}{name}{after}`")
        } else {
            format!("write `{before}{name} of {}{after}`", names.join(", "))
        };
        self.error_with_fix(
            module,
            "type-arity",
            format!(
                "`{name}` takes {expected} type argument{}, found {found}",
                if expected == 1 { "" } else { "s" }
            ),
            span,
            fix,
        );
        false
    }

    /// `see also:` names definitions (decision C8b): one of this module's,
    /// `module.name` of an import, or `Type.method`.
    fn check_see_also(&mut self, id: ModuleId) {
        if self.modules[id].is_library {
            return;
        }
        let mut problems: Vec<(String, Span)> = Vec::new();
        {
            let module = &self.modules[id];
            let mut note = |docs: &ast::Docs, span: Span| {
                for name in &docs.see_also {
                    if !self.reference_exists(id, name) {
                        problems.push((name.clone(), span));
                    }
                }
            };
            for item in &module.ast.items {
                match item {
                    Item::Function(function) => note(&function.docs, function.name.span),
                    Item::Implementation(implementation) => {
                        for function in &implementation.functions {
                            note(&function.docs, function.name.span);
                        }
                    }
                    Item::Type(def) => note(&def.docs, def.name.span),
                    Item::Ability(ability) => note(&ability.docs, ability.name.span),
                    Item::Constant(constant) => note(&constant.docs, constant.name.span),
                    Item::Test(_) => {}
                }
            }
        }
        for (name, span) in problems {
            self.error_with_fix(
                id,
                "unknown-reference",
                format!("`see also: {name}` names nothing in scope"),
                span,
                "name a function, type, constant or ability of this module, `module.name` of an import, or `Type.method`".into(),
            );
        }
    }

    fn reference_exists(&self, module: ModuleId, name: &str) -> bool {
        let info = &self.modules[module];
        if let Some((head, rest)) = name.split_once('.') {
            if head.starts_with(|c: char| c.is_ascii_uppercase()) {
                return self
                    .lookup_type(module, head)
                    .is_some_and(|type_id| !self.methods_of(type_id, rest).is_empty());
            }
            let Some(&target) = info.imports.get(head) else {
                return false;
            };
            let other = &self.modules[target];
            return other.functions.contains_key(rest)
                || other.types.contains_key(rest)
                || other.constants.contains_key(rest)
                || other.abilities.contains_key(rest);
        }
        info.functions.contains_key(name)
            || info.types.contains_key(name)
            || info.constants.contains_key(name)
            || info.abilities.contains_key(name)
            || info.exposed_types.contains_key(name)
            || info.exposed_abilities.contains_key(name)
            || self.prelude.is_some_and(|prelude| {
                let prelude = &self.modules[prelude];
                prelude.functions.contains_key(name) || prelude.types.contains_key(name)
            })
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
            iterable: get_ability("Iterable"),
        };
    }

    pub(crate) fn error_with_fix(
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
            let Some(target) = self.imported_module(id, &name) else {
                let fix = match closest(&name, self.modules.iter().map(|m| m.name.as_str())) {
                    Some(close) => format!("did you mean `{close}`?"),
                    None => format!(
                        "create `{}.ry` with `module {name}` as its first line",
                        name.replace('.', "/")
                    ),
                };
                self.error_with_fix(
                    id,
                    "unknown-module",
                    format!("no module named `{name}`"),
                    import.span,
                    fix,
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
                        self.error_with_fix(
                            id,
                            "private-name",
                            format!("`{text}` is not public in `{name}`"),
                            exposed.span,
                            format!("add `public` to its declaration in `{name}`"),
                        );
                    }
                    self.modules[id].exposed_types.insert(text, type_id);
                } else if let Some(&ability_id) = self.modules[target].abilities.get(&text) {
                    if !self.abilities[ability_id].public {
                        self.error_with_fix(
                            id,
                            "private-name",
                            format!("`{text}` is not public in `{name}`"),
                            exposed.span,
                            format!("add `public` to its declaration in `{name}`"),
                        );
                    }
                    self.modules[id].exposed_abilities.insert(text, ability_id);
                } else {
                    let candidates: Vec<&str> = self.modules[target]
                        .types
                        .keys()
                        .chain(self.modules[target].abilities.keys())
                        .map(String::as_str)
                        .collect();
                    let fix = match closest(&text, candidates.into_iter()) {
                        Some(close) => format!("did you mean `{close}`?"),
                        None => format!("expose a type or ability that `{name}` declares"),
                    };
                    self.error_with_fix(
                        id,
                        "unknown-name",
                        format!("`{name}` has no type or ability named `{text}`"),
                        exposed.span,
                        fix,
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

    /// The fix for an unknown type: the Renyi name of a foreign one, an
    /// import, the closest name in scope, or a declaration.
    pub fn suggest_type(&self, module: ModuleId, name: &str) -> String {
        if let Some(fix) = foreign_type(name) {
            return fix.to_string();
        }
        if let Some(fix) = self.suggest_import(module, name) {
            return fix;
        }
        let info = &self.modules[module];
        let names: Vec<&str> = info
            .types
            .keys()
            .chain(info.exposed_types.keys())
            .chain(
                self.prelude
                    .iter()
                    .flat_map(|&prelude| self.modules[prelude].types.keys()),
            )
            .map(String::as_str)
            .collect();
        match closest(name, names.into_iter()) {
            Some(close) => format!("did you mean `{close}`?"),
            None => format!("declare `type {name}`, or import the module that declares it"),
        }
    }

    /// The fix for an unknown ability: an import, the closest name in
    /// scope, or a declaration.
    pub fn suggest_ability(&self, module: ModuleId, name: &str) -> String {
        if let Some(fix) = self.suggest_import(module, name) {
            return fix;
        }
        let info = &self.modules[module];
        let names: Vec<&str> = info
            .abilities
            .keys()
            .chain(info.exposed_abilities.keys())
            .chain(
                self.prelude
                    .iter()
                    .flat_map(|&prelude| self.modules[prelude].abilities.keys()),
            )
            .map(String::as_str)
            .collect();
        match closest(name, names.into_iter()) {
            Some(close) => format!("did you mean `{close}`?"),
            None => format!("declare `ability {name}`, or import the module that declares it"),
        }
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

    /// The abilities a type parameter must have, with their arguments.
    pub fn constraints(&self, param: ParamId) -> &[AbilityRef] {
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
                        self.error_with_fix(
                            module,
                            "type-arity",
                            format!("`{}` is a type parameter and takes no arguments", name.text),
                            *span,
                            "drop the arguments".into(),
                        );
                    }
                    return Ty::Param(*id);
                }
                let Some(type_id) = self.lookup_type(module, &name.text) else {
                    let fix = self.suggest_type(module, &name.text);
                    let message = format!("unknown type `{}`", name.text);
                    self.error_with_fix(module, "unknown-type", message, name.span, fix);
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
                    let fix = if expected == 0 {
                        "drop the type arguments".to_string()
                    } else {
                        let params: Vec<String> = self.types[type_id]
                            .params
                            .iter()
                            .map(|&p| self.param_name(p))
                            .collect();
                        let joined = if params.len() == 2 {
                            params.join(" to ")
                        } else {
                            params.join(", ")
                        };
                        format!("write `{} of {joined}`", name.text)
                    };
                    self.error_with_fix(module, "type-arity", message, *span, fix);
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
                let declared: Vec<String> = params.iter().map(|(n, _)| n.clone()).collect();
                self.error_with_fix(
                    module,
                    "unknown-type",
                    format!(
                        "`{}` is not one of the type parameters",
                        constraint.param.text
                    ),
                    constraint.param.span,
                    format!("name one of {}", quoted(&declared)),
                );
                continue;
            };
            let param = *param;
            let ast::Type::Named { name, args, span } = &constraint.ability else {
                continue;
            };
            let Some(ability) = self.lookup_ability(module, &name.text) else {
                let message = format!("unknown ability `{}`", name.text);
                let fix = self.suggest_ability(module, &name.text);
                self.error_with_fix(module, "unknown-ability", message, name.span, fix);
                continue;
            };
            // the arguments are any types in scope, the clause's own
            // parameters among them (decision AB1)
            let mut args: Vec<Ty> = args
                .iter()
                .map(|arg| self.resolve_type(module, &params, arg))
                .collect();
            let before = format!("{} can ", constraint.param.text);
            if !self.check_ability_arity(module, ability, args.len(), *span, &before, "") {
                args.resize(self.abilities[ability].params.len(), Ty::Error);
            }
            self.params[param]
                .constraints
                .push(AbilityRef { ability, args });
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
                    // decision K10: JSON spells the variant's name under `kind`
                    if matches!(self.abilities[ability].name.as_str(), "ToJson" | "FromJson") {
                        let mut clashes: Vec<(String, Span)> = Vec::new();
                        if let TypeKindInfo::Sum(variants) = &self.types[type_id].kind {
                            for variant in variants {
                                for field in &variant.fields {
                                    if field.name == "kind" {
                                        clashes.push((variant.name.clone(), field.span));
                                    }
                                }
                            }
                        }
                        for (variant, span) in clashes {
                            self.error_with_fix(
                                module,
                                "kind-field",
                                format!(
                                    "the variant `{variant}` has a field named `kind`, the key JSON uses for the variant's name (decision K10)"
                                ),
                                span,
                                "rename the field".into(),
                            );
                        }
                    }
                    if !derive.by.is_empty() {
                        // `can Compare by field, field`: every field must exist
                        if let TypeKindInfo::Record(fields) = &self.types[type_id].kind {
                            let missing: Vec<&ast::Name> = derive
                                .by
                                .iter()
                                .filter(|name| !fields.iter().any(|f| f.name == name.text))
                                .collect();
                            let field_names: Vec<String> =
                                fields.iter().map(|f| f.name.clone()).collect();
                            for name in missing {
                                let message = format!(
                                    "`{}` has no field named `{}`",
                                    self.types[type_id].name, name.text
                                );
                                let fix = match closest(
                                    &name.text,
                                    field_names.iter().map(String::as_str),
                                ) {
                                    Some(close) => format!("did you mean `{close}`?"),
                                    None => format!("name one of {}", quoted(&field_names)),
                                };
                                self.error_with_fix(
                                    module,
                                    "unknown-field",
                                    message,
                                    name.span,
                                    fix,
                                );
                            }
                        }
                    }
                }
                None => {
                    let message = format!("unknown ability `{}`", derive.ability.text);
                    let fix = self.suggest_ability(module, &derive.ability.text);
                    self.error_with_fix(
                        module,
                        "unknown-ability",
                        message,
                        derive.ability.span,
                        fix,
                    );
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
        // every ability's parameters first, so that a requirement can count
        // the arguments of an ability declared later in the module
        let mut pending = Vec::new();
        for item in &items {
            let Item::Ability(ability) = item else {
                continue;
            };
            let ability_id = self.modules[id].abilities[&ability.name.text];
            if !self.abilities[ability_id].methods.is_empty() || self.abilities[ability_id].resolved
            {
                continue;
            }
            let params = self.new_params(&ability.type_params);
            self.abilities[ability_id].params = params.iter().map(|(_, p)| *p).collect();
            self.abilities[ability_id].resolved = true;
            pending.push((ability, ability_id, params));
        }
        for (ability, ability_id, params) in pending {
            let mut requirements = Vec::new();
            for requirement in &ability.requirements {
                let ast::Type::Named { name, args, span } = requirement else {
                    continue;
                };
                let Some(required) = self.lookup_ability(id, &name.text) else {
                    let message = format!("unknown ability `{}`", name.text);
                    let fix = self.suggest_ability(id, &name.text);
                    self.error_with_fix(id, "unknown-ability", message, name.span, fix);
                    continue;
                };
                // the arguments may name the ability's own parameters
                // (decision AB1)
                let mut args: Vec<Ty> = args
                    .iter()
                    .map(|arg| self.resolve_type(id, &params, arg))
                    .collect();
                let count = args.len();
                if !self.check_ability_arity(id, required, count, *span, "self can ", "") {
                    args.resize(self.abilities[required].params.len(), Ty::Error);
                }
                requirements.push(AbilityRef {
                    ability: required,
                    args,
                });
            }
            self.abilities[ability_id].requirements = requirements;
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
                        let fix = self.suggest_ability(id, &ability_name.text);
                        self.error_with_fix(id, "unknown-ability", message, ability_name.span, fix);
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
                    let ability_args: Vec<Ty> = match &implementation.ability {
                        ast::Type::Named { args, .. } => args
                            .iter()
                            .map(|arg| self.resolve_type(id, &params, arg))
                            .collect(),
                        _ => Vec::new(),
                    };
                    self.check_ability_arity(
                        id,
                        ability,
                        ability_args.len(),
                        ability_name.span,
                        "ability ",
                        " for ...",
                    );
                    // one implementation of an ability per type, whatever the
                    // arguments: dispatch is by the value's type alone, and
                    // a loop needs one item type (decision AB1)
                    if let Some(head) = head_type(&target) {
                        let twice = self.impls.iter().any(|i| {
                            i.ability == ability
                                && head_type(&i.target) == Some(head)
                                && !(i.module == id && i.span == implementation.span)
                        });
                        if twice {
                            let type_name = self.type_name(head);
                            self.error_with_fix(
                                id,
                                "duplicate-implementation",
                                format!("`{type_name}` already implements `{}`", ability_name.text),
                                ability_name.span,
                                format!(
                                    "keep one implementation of `{}` for `{type_name}`",
                                    ability_name.text
                                ),
                            );
                        }
                    }
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
                            // the ability's visibility covers its methods
                            // (reference section 5), so that a public
                            // ability's method is called from any module
                            self.functions[function_id].public = self.abilities[ability].public;
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
                                &ability_args,
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
                            let fix =
                                format!("add `function {name}(self, ...)` to the implementation");
                            self.error_with_fix(
                                id,
                                "missing-method",
                                message,
                                implementation.span,
                                fix,
                            );
                        }
                    }
                    for function in &implementation.functions {
                        if !expected.contains(&function.name.text) {
                            let message = format!(
                                "`{}` is not a method of `{}`",
                                function.name.text, ability_name.text
                            );
                            self.error_with_fix(
                                id,
                                "unknown-method",
                                message,
                                function.name.span,
                                "remove it, or declare it in the ability".into(),
                            );
                        }
                    }
                    self.impls.push(ImplInfo {
                        ability,
                        target,
                        ability_args,
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

    /// Decision AF1: a function of a foreign module is a C function. It
    /// needs `foreign` and nothing else, declares no failures and no type
    /// parameters, and its parameters and result are types of the boundary,
    /// at most six words of them.
    fn check_foreign_signature(
        &mut self,
        module: ModuleId,
        function: &ast::Function,
        params: &[(String, Ty)],
        returns: Option<&Ty>,
        fails: &[Ty],
        needs: &[Capability],
    ) {
        let name = function.name.text.clone();
        let only_foreign = needs.len() == 1
            && needs[0].path == ["foreign"]
            && needs[0].scope.is_none()
            && !needs[0].has_grant_clauses();
        if !only_foreign {
            self.error_with_fix(
                module,
                "foreign-signature",
                format!(
                    "`{name}` is declared in a foreign module; it needs `foreign` and nothing else"
                ),
                function.name.span,
                "write `needs foreign`".into(),
            );
        }
        if !fails.is_empty() {
            self.error_with_fix(
                module,
                "foreign-signature",
                format!(
                    "`{name}` is declared in a foreign module; a C function declares no failures"
                ),
                function.name.span,
                "drop `or fails with`".into(),
            );
        }
        if function.type_params.is_some() {
            self.error_with_fix(
                module,
                "foreign-signature",
                format!(
                    "`{name}` is declared in a foreign module; a C function takes no type parameters"
                ),
                function.name.span,
                "drop `for any`".into(),
            );
        }
        let mut words = 0;
        for (index, (param_name, ty)) in params.iter().enumerate() {
            match self.c_type_of(ty) {
                Some(c_type) => words += c_type.words(),
                None => {
                    let span = function
                        .params
                        .get(index)
                        .map(|param| param.name.span)
                        .unwrap_or(function.name.span);
                    self.error_with_fix(
                        module,
                        "foreign-type",
                        format!(
                            "the parameter `{param_name}` of `{name}` has type `{}`; across the boundary a parameter is {}",
                            self.show(ty),
                            crate::foreign::PARAMETER_TYPES
                        ),
                        span,
                        "declare it with a type of `std.foreign`".into(),
                    );
                }
            }
        }
        if self.c_result_of(returns).is_none() {
            let shown = returns.map(|ty| self.show(ty)).unwrap_or_default();
            let span = function
                .returns
                .as_ref()
                .map(|ty| ty.span())
                .unwrap_or(function.name.span);
            self.error_with_fix(
                module,
                "foreign-type",
                format!(
                    "`{name}` returns `{shown}`; across the boundary a result is {}",
                    crate::foreign::RESULT_TYPES
                ),
                span,
                "declare it with a type of `std.foreign`, or drop `returns`".into(),
            );
        }
        if words > crate::foreign::MAX_WORDS {
            self.error_with_fix(
                module,
                "foreign-arity",
                format!(
                    "`{name}` takes {words} words across the boundary; the limit is six (`Bytes` counts two)"
                ),
                function.name.span,
                "pass fewer arguments".into(),
            );
        }
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
                self.error_with_fix(
                    module,
                    "duplicate-name",
                    format!("the parameter `{}` is declared twice", param.name.text),
                    param.name.span,
                    "rename one of them".into(),
                );
            }
            names.push(&param.name.text);
            let ty = match &param.ty {
                Some(ty) => self.resolve_type(module, &params, ty),
                None if index == 0 && param.name.text == "self" => Ty::SelfType,
                None => Ty::Error,
            };
            if index > 0 && param.name.text == "self" {
                self.error_with_fix(
                    module,
                    "self-position",
                    "`self` must be the first parameter".into(),
                    param.name.span,
                    "move `self` before the other parameters".into(),
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
        let fails: Vec<Ty> = function
            .fails
            .iter()
            .map(|f| self.resolve_type(module, &params, f))
            .collect();
        let needs: Vec<Capability> = function.needs.iter().map(Capability::from_ast).collect();
        for (capability, syntax) in needs.iter().zip(&function.needs) {
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
                    self.error_with_fix(
                        module,
                        "unknown-capability",
                        format!(
                            "unknown capability `{}` after `only to`",
                            sink_path.join(".")
                        ),
                        syntax.span,
                        format!("one of {}", crate::effects::TREE.join(", ")),
                    );
                }
            }
            if capability.budget.is_some() {
                let budgeted = matches!(
                    capability.path.first().map(String::as_str),
                    Some("network") | Some("process") | Some("filesystem")
                );
                if !budgeted {
                    self.error_with_fix(
                        module,
                        "grant-clause",
                        format!(
                            "`{}` takes no budget; budgets apply to network, process and filesystem",
                            capability.path.join(".")
                        ),
                        syntax.span,
                        "drop the `at most` clause".into(),
                    );
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
                self.error_with_fix(
                    module,
                    "capability-scope",
                    format!("`{}` takes no scope argument", capability.path.join(".")),
                    syntax.span,
                    "drop the scope argument".into(),
                );
            }
        }
        if self.modules[module].foreign.is_some() {
            self.check_foreign_signature(
                module,
                function,
                &param_types,
                returns.as_ref(),
                &fails,
                &needs,
            );
        }
        if self.modules[module].python.is_some() {
            self.check_python_signature(
                module,
                function,
                &param_types,
                returns.as_ref(),
                &fails,
                &needs,
            );
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
            self.error_with_fix(
                module,
                "duplicate-name",
                format!("the function `{}` is declared twice", function.name.text),
                function.name.span,
                "rename one of them".into(),
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
        ability_args: &[Ty],
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
        // the ability's own type parameters are the implementation's arguments
        let ability_params = self.abilities[ability].params.clone();
        let subst = |p: ParamId| {
            ability_params
                .iter()
                .position(|q| *q == p)
                .and_then(|index| ability_args.get(index).cloned())
        };
        let expected: Vec<(String, Ty)> = method
            .params
            .iter()
            .map(|(n, t)| (n.clone(), t.with_self(target).substitute(&subst)))
            .collect();
        let expected_returns = method
            .returns
            .as_ref()
            .map(|r| r.with_self(target).substitute(&subst));
        let expected_fails: Vec<Ty> = method
            .fails
            .iter()
            .map(|f| f.with_self(target).substitute(&subst))
            .collect();
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
        // an ability's method has no effects (decision V10)
        if !self.functions[function_id].needs.is_empty() {
            problems.push("it declares `needs`; an ability's method has no effects".to_string());
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
