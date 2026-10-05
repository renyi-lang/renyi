//! Checking function bodies, tests, examples and constants: names, types,
//! effects, errors and the rules of decisions J8, J9, J15 and M2 to M4.

use std::collections::HashSet;

use renyi_syntax::ast::*;
use renyi_syntax::{Diagnostic, Span};

use crate::effects::{covered, Capability};
use crate::refine::{self, Literal, Verdict};
use crate::types::*;
use crate::world::{head_type, BodyLocation, FunctionInfo, TypeKindInfo, World};

struct VarInfo {
    binding: Option<Ty>,
    kind: VarKind,
}

#[derive(Clone, Debug, PartialEq)]
enum VarKind {
    Any,
    IntegerLiteral(Option<Literal>),
    DecimalLiteral(Option<Literal>),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BindingKind {
    Let,
    Param,
    Loop,
    Pattern,
    Query,
}

struct Binding {
    name: String,
    ty: Ty,
    mutable: bool,
    used: bool,
    span: Span,
    kind: BindingKind,
}

struct Scope {
    bindings: Vec<Binding>,
}

struct Context {
    name: String,
    returns: Option<Ty>,
    fails: Vec<Ty>,
    /// A test: `fail` and `otherwise fail` end the test, whatever the error.
    fails_any: bool,
    needs: Vec<Capability>,
    is_test: bool,
}

enum Deferred {
    Constraint {
        ty: Ty,
        ability: AbilityId,
        span: Span,
        what: String,
    },
}

/// What an expression produced.
#[derive(Clone)]
struct Info {
    ty: Ty,
    /// The error types of a fallible call that nothing has handled yet.
    fails: Vec<Ty>,
    /// The function a bare name or `module.function` refers to.
    function: Option<FunctionId>,
}

impl Info {
    fn plain(ty: Ty) -> Info {
        Info {
            ty,
            fails: Vec::new(),
            function: None,
        }
    }
}

/// How a `match` arm covers its subject, for exhaustiveness.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum Cover {
    Variant(String),
    Some,
    Nothing,
    Success,
    Failure,
    True,
    False,
    Other,
}

/// What a body refers to, as the checker resolved it: for tools that need
/// the reference graph (the project map) rather than diagnostics.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    /// A declared function or method, called or passed by name.
    Function(FunctionId),
    /// A method of an ability, called on a value whose type has the ability.
    AbilityMethod(AbilityId, usize),
    /// A constant of the module.
    Constant(ModuleId, String),
    /// A type named in a construction, a pattern or an annotation inside the
    /// body, at its name token.
    Type(TypeId),
    /// A variant of a sum type, constructed or named bare, at the variant's
    /// name token.
    Variant(TypeId, usize),
    /// An ability named in a signature or an implementation head; the checker
    /// itself never records one, tools that read signatures do.
    Ability(AbilityId),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Reference {
    pub target: Target,
    pub span: Span,
}

pub struct Checker<'w> {
    world: &'w World,
    module: ModuleId,
    pub diagnostics: Vec<Diagnostic>,
    /// Every reference resolved so far, with the body it occurs in.
    pub references: Vec<(BodyLocation, Reference)>,
    /// The body being checked; `None` while checking `example:` lines, whose
    /// references are not body references.
    owner: BodyLocation,
    vars: Vec<VarInfo>,
    scopes: Vec<Scope>,
    context: Context,
    deferred: Vec<Deferred>,
    loop_depth: usize,
    /// Calls that needed a capability so far; `ignore` compares it before
    /// and after the ignored expression (decision R6).
    effect_calls: usize,
    /// Inside `run concurrently`: `break` and `continue` are not allowed.
    concurrent_depth: usize,
    /// The error union of the fallible call a `match` is examining, for its
    /// `failure(...)` arms.
    current_errors: Option<Ty>,
}

impl<'w> Checker<'w> {
    pub fn new(world: &'w World, module: ModuleId) -> Checker<'w> {
        Checker {
            world,
            module,
            diagnostics: Vec::new(),
            references: Vec::new(),
            owner: BodyLocation::None,
            vars: Vec::new(),
            scopes: Vec::new(),
            context: Context {
                name: String::new(),
                returns: None,
                fails: Vec::new(),
                fails_any: false,
                needs: Vec::new(),
                is_test: false,
            },
            deferred: Vec::new(),
            loop_depth: 0,
            effect_calls: 0,
            concurrent_depth: 0,
            current_errors: None,
        }
    }

    // ------------------------------------------------------------ diagnostics

    fn error(&mut self, code: &'static str, message: impl Into<String>, span: Span) {
        self.diagnostics
            .push(Diagnostic::error(code, message, span));
    }

    fn error_fix(
        &mut self,
        code: &'static str,
        message: impl Into<String>,
        span: Span,
        fix: impl Into<String>,
    ) {
        self.diagnostics
            .push(Diagnostic::error(code, message, span).with_fix(fix));
    }

    fn warning(&mut self, code: &'static str, message: impl Into<String>, span: Span) {
        self.diagnostics
            .push(Diagnostic::warning(code, message, span));
    }

    fn show(&self, ty: &Ty) -> String {
        self.world.show(&self.zonk(ty))
    }

    // ------------------------------------------------------------- references

    fn record(&mut self, target: Target, span: Span) {
        if !matches!(self.owner, BodyLocation::None) {
            self.references
                .push((self.owner, Reference { target, span }));
        }
    }

    /// Record every type an annotation names, each at its own name token, so
    /// that a tool can replace the name alone.
    fn record_type_names(&mut self, ty: &Type) {
        match ty {
            Type::Named { name, args, .. } => {
                if let Some(id) = self.world.lookup_type(self.module, &name.text) {
                    self.record(Target::Type(id), name.span);
                }
                for arg in args {
                    self.record_type_names(arg);
                }
            }
            Type::Maybe(inner, _) => self.record_type_names(inner),
            Type::Function {
                params,
                returns,
                fails,
                ..
            } => {
                for param in params {
                    self.record_type_names(param);
                }
                if let Some(returns) = returns {
                    self.record_type_names(returns);
                }
                for fails in fails {
                    self.record_type_names(fails);
                }
            }
        }
    }

    // ------------------------------------------------------------- variables

    fn fresh(&mut self, kind: VarKind) -> Ty {
        let id = self.vars.len();
        self.vars.push(VarInfo {
            binding: None,
            kind,
        });
        Ty::Var(id)
    }

    /// Follow variable bindings at the top.
    fn resolve(&self, ty: &Ty) -> Ty {
        let mut current = ty.clone();
        while let Ty::Var(id) = current {
            match &self.vars[id].binding {
                Some(bound) => current = bound.clone(),
                None => break,
            }
        }
        current
    }

    /// Follow variable bindings everywhere.
    fn zonk(&self, ty: &Ty) -> Ty {
        match self.resolve(ty) {
            Ty::App(id, args) => Ty::App(id, args.iter().map(|a| self.zonk(a)).collect()),
            Ty::Maybe(inner) => Ty::maybe(self.zonk(&inner)),
            Ty::Function(function) => Ty::Function(Box::new(FunctionTy {
                params: function.params.iter().map(|p| self.zonk(p)).collect(),
                returns: function.returns.as_ref().map(|r| self.zonk(r)),
                fails: function.fails.iter().map(|f| self.zonk(f)).collect(),
                needs: function.needs.clone(),
            })),
            Ty::Union(members) => Ty::Union(members.iter().map(|m| self.zonk(m)).collect()),
            other => other,
        }
    }

    fn builtin(&self, id: TypeId) -> Ty {
        Ty::App(id, Vec::new())
    }

    fn is_builtin(&self, ty: &Ty, id: TypeId) -> bool {
        matches!(self.resolve(ty), Ty::App(found, _) if found == id)
    }

    /// The base type a subtype chain ends in, for `type X is Base`.
    fn base_of(&self, ty: &Ty) -> Ty {
        let mut current = self.resolve(ty);
        loop {
            match &current {
                Ty::App(id, _) => match &self.world.types[*id].kind {
                    TypeKindInfo::Subtype { base, .. } => current = self.resolve(base),
                    _ => return current,
                },
                _ => return current,
            }
        }
    }

    fn is_numeric(&self, ty: &Ty) -> bool {
        let b = &self.world.builtins;
        let base = self.base_of(ty);
        self.is_builtin(&base, b.integer)
            || self.is_builtin(&base, b.decimal)
            || self.is_builtin(&base, b.float)
    }

    /// Bind a variable, honoring literal kinds and the occurs check.
    fn bind_var(&mut self, id: VarId, ty: &Ty, span: Span) -> bool {
        let ty = self.resolve(ty);
        if let Ty::Var(other) = &ty {
            if *other == id {
                return true;
            }
            // merging two variables: a literal kind wins over `Any`, two literal
            // kinds keep the more restrictive one (decimal)
            let kind = match (&self.vars[id].kind, &self.vars[*other].kind) {
                (VarKind::Any, k) | (k, VarKind::Any) => k.clone(),
                (VarKind::DecimalLiteral(l), _) | (_, VarKind::DecimalLiteral(l)) => {
                    VarKind::DecimalLiteral(l.clone())
                }
                (VarKind::IntegerLiteral(l), _) => VarKind::IntegerLiteral(l.clone()),
            };
            self.vars[*other].kind = kind;
            self.vars[id].binding = Some(ty);
            return true;
        }
        if ty.mentions_var(id) {
            return false;
        }
        match self.vars[id].kind.clone() {
            VarKind::Any => {}
            VarKind::IntegerLiteral(literal) | VarKind::DecimalLiteral(literal) => {
                if !self.literal_fits(&self.vars[id].kind.clone(), literal.as_ref(), &ty, span) {
                    return false;
                }
            }
        }
        self.vars[id].binding = Some(ty);
        true
    }

    /// Whether a number literal can take the type: a numeric base type (an
    /// Integer literal fits Decimal and Float too, a Decimal literal fits
    /// Float), or a refined subtype of one whose condition the literal meets.
    fn literal_fits(
        &mut self,
        kind: &VarKind,
        literal: Option<&Literal>,
        ty: &Ty,
        span: Span,
    ) -> bool {
        let b = &self.world.builtins;
        let base = self.base_of(ty);
        let fits_base = match kind {
            VarKind::IntegerLiteral(_) => {
                self.is_builtin(&base, b.integer)
                    || self.is_builtin(&base, b.decimal)
                    || self.is_builtin(&base, b.float)
            }
            VarKind::DecimalLiteral(_) => {
                self.is_builtin(&base, b.decimal) || self.is_builtin(&base, b.float)
            }
            VarKind::Any => true,
        };
        if !fits_base {
            return false;
        }
        if let Some(literal) = literal {
            self.check_subtype_refinements(ty, literal, span)
        } else {
            true
        }
    }

    /// Run the refinements of a subtype chain on a literal; false (and a
    /// diagnostic) when one fails.
    fn check_subtype_refinements(&mut self, ty: &Ty, literal: &Literal, span: Span) -> bool {
        let mut current = self.resolve(ty);
        loop {
            let Ty::App(id, _) = &current else {
                return true;
            };
            let TypeKindInfo::Subtype { base, refinement } = &self.world.types[*id].kind else {
                return true;
            };
            if let Some(condition) = refinement {
                match refine::evaluate(condition, "value", literal) {
                    Verdict::Holds | Verdict::Unknown => {}
                    Verdict::Fails => {
                        let name = self.world.types[*id].name.clone();
                        self.error(
                            "constraint-violation",
                            format!("this value does not satisfy the condition of `{name}`"),
                            span,
                        );
                        return false;
                    }
                }
            }
            current = self.resolve(base);
        }
    }

    /// Is `actual` acceptable where `expected` is wanted? Binds variables,
    /// climbs subtypes, wraps into `maybe`.
    fn assign(&mut self, actual: &Ty, expected: &Ty, span: Span) -> bool {
        let actual = self.resolve(actual);
        let expected = self.resolve(expected);
        match (&actual, &expected) {
            (Ty::Error, _) | (_, Ty::Error) | (Ty::Never, _) => true,
            (Ty::Var(a), Ty::Var(e)) if a == e => true,
            (_, Ty::Var(e)) => self.bind_var(*e, &actual, span),
            (Ty::Var(a), Ty::Maybe(inner)) if !matches!(self.vars[*a].kind, VarKind::Any) => {
                let inner = (**inner).clone();
                self.assign(&actual, &inner, span)
            }
            (Ty::Var(a), _) => self.bind_var(*a, &expected, span),
            (_, Ty::Maybe(inner)) => {
                let unwrapped = match &actual {
                    Ty::Maybe(a) => (**a).clone(),
                    other => other.clone(),
                };
                self.assign(&unwrapped, inner, span)
            }
            (Ty::Maybe(_), _) => false,
            (Ty::App(a, a_args), Ty::App(e, e_args)) => {
                if a == e {
                    a_args
                        .iter()
                        .zip(e_args)
                        .all(|(x, y)| self.assign(x, y, span))
                } else if let TypeKindInfo::Subtype { base, .. } = &self.world.types[*a].kind {
                    let base = base.clone();
                    self.assign(&base, &expected, span)
                } else {
                    false
                }
            }
            (Ty::Function(a), Ty::Function(e)) => {
                if a.params.len() != e.params.len() {
                    return false;
                }
                let params_ok = a
                    .params
                    .iter()
                    .zip(&e.params)
                    .all(|(x, y)| self.unify(x, y, span));
                let returns_ok = match (&a.returns, &e.returns) {
                    (None, None) => true,
                    (Some(x), Some(y)) => self.assign(x, y, span),
                    _ => false,
                };
                // the function value may fail only with what the expected type lists
                let fails_ok = a
                    .fails
                    .iter()
                    .all(|f| e.fails.iter().any(|g| self.same_type(f, g)));
                params_ok && returns_ok && fails_ok
            }
            (Ty::Param(a), Ty::Param(e)) => a == e,
            (Ty::Union(a), Ty::Union(e)) => {
                a.iter().all(|x| e.iter().any(|y| self.same_type(x, y)))
            }
            (Ty::Union(a), _) => a.iter().all(|x| self.assign(x, &expected, span)),
            (Ty::Unit, Ty::Unit) => true,
            _ => false,
        }
    }

    fn unify(&mut self, a: &Ty, b: &Ty, span: Span) -> bool {
        let a = self.resolve(a);
        let b = self.resolve(b);
        match (&a, &b) {
            (Ty::Error, _) | (_, Ty::Error) => true,
            (Ty::Var(x), Ty::Var(y)) if x == y => true,
            (Ty::Var(x), _) => self.bind_var(*x, &b, span),
            (_, Ty::Var(y)) => self.bind_var(*y, &a, span),
            (Ty::App(x, xa), Ty::App(y, ya)) => {
                x == y && xa.iter().zip(ya).all(|(p, q)| self.unify(p, q, span))
            }
            (Ty::Maybe(x), Ty::Maybe(y)) => self.unify(x, y, span),
            (Ty::Param(x), Ty::Param(y)) => x == y,
            _ => a == b,
        }
    }

    fn same_type(&self, a: &Ty, b: &Ty) -> bool {
        self.zonk(a) == self.zonk(b)
    }

    /// Report a mismatch with a message that names the usual mistakes.
    fn mismatch(&mut self, actual: &Ty, expected: &Ty, span: Span, what: &str) {
        let shown_actual = self.show(actual);
        let shown_expected = self.show(expected);
        let actual_resolved = self.resolve(actual);
        if let Ty::Maybe(inner) = &actual_resolved {
            if self.resolve(expected) == self.resolve(inner)
                || !matches!(self.resolve(expected), Ty::Maybe(_))
            {
                self.error_fix(
                    "maybe-value",
                    format!("{what} may be nothing (it is `{shown_actual}`) where `{shown_expected}` is needed"),
                    span,
                    "read it out with `otherwise` or a `match`",
                );
                return;
            }
        }
        if matches!(actual_resolved, Ty::Unit) {
            self.error(
                "type-mismatch",
                format!("{what} returns nothing, but `{shown_expected}` is needed"),
                span,
            );
            return;
        }
        self.error(
            "type-mismatch",
            format!("{what} is `{shown_actual}`, but `{shown_expected}` is needed"),
            span,
        );
    }

    fn expect(&mut self, actual: &Ty, expected: &Ty, span: Span, what: &str) {
        if !self.assign(actual, expected, span) {
            self.mismatch(actual, expected, span, what);
        }
    }

    // --------------------------------------------------------------- abilities

    /// Whether a type has an ability. `None` when an unbound variable makes it
    /// unknown for now.
    fn has_ability(&self, ty: &Ty, ability: AbilityId) -> Option<bool> {
        let b = &self.world.builtins;
        let ty = self.resolve(ty);
        if ability == b.equal {
            return Some(!matches!(ty, Ty::Function(_)));
        }
        match &ty {
            Ty::Var(_) => None,
            Ty::Error | Ty::Never => Some(true),
            Ty::Param(id) => Some(self.world.constraints(*id).contains(&ability)),
            Ty::Maybe(inner) => {
                if ability == b.hash || self.is_json_ability(ability) {
                    self.has_ability(inner, ability)
                } else {
                    Some(false)
                }
            }
            Ty::Union(members) => {
                let mut all = true;
                for member in members {
                    match self.has_ability(member, ability) {
                        Some(true) => {}
                        Some(false) => all = false,
                        None => return None,
                    }
                }
                Some(all)
            }
            Ty::App(id, args) => {
                let info = &self.world.types[*id];
                if info.derives.contains(&ability) {
                    return Some(true);
                }
                if self
                    .world
                    .impls
                    .iter()
                    .any(|i| i.ability == ability && head_type(&i.target) == Some(*id))
                {
                    return Some(true);
                }
                if let TypeKindInfo::Subtype { base, .. } = &info.kind {
                    return self.has_ability(base, ability);
                }
                // structural rules for the collection types
                if *id == b.list || *id == b.set {
                    if ability == b.hash || self.is_json_ability(ability) {
                        return self.has_ability(&args[0], ability);
                    }
                    return Some(false);
                }
                if *id == b.map {
                    if self.is_json_ability(ability) {
                        return match (
                            self.has_ability(&args[0], ability),
                            self.has_ability(&args[1], ability),
                        ) {
                            (Some(true), Some(true)) => Some(self.is_builtin(&args[0], b.text)),
                            (None, _) | (_, None) => None,
                            _ => Some(false),
                        };
                    }
                    return Some(false);
                }
                if *id == b.pair {
                    if ability == b.hash {
                        return match (
                            self.has_ability(&args[0], ability),
                            self.has_ability(&args[1], ability),
                        ) {
                            (Some(true), Some(true)) => Some(true),
                            (None, _) | (_, None) => None,
                            _ => Some(false),
                        };
                    }
                    return Some(false);
                }
                // the scalar types carry their abilities as `can` clauses in the
                // prelude; JSON abilities hold for them too
                if self.is_json_ability(ability) {
                    return Some(
                        *id == b.integer
                            || *id == b.decimal
                            || *id == b.float
                            || *id == b.text
                            || *id == b.boolean
                            || *id == b.bytes,
                    );
                }
                Some(false)
            }
            _ => Some(false),
        }
    }

    fn is_json_ability(&self, ability: AbilityId) -> bool {
        let name = &self.world.abilities[ability].name;
        name == "ToJson" || name == "FromJson"
    }

    fn require_ability(&mut self, ty: &Ty, ability: AbilityId, span: Span, what: &str) {
        match self.has_ability(ty, ability) {
            Some(true) => {}
            Some(false) => {
                let name = self.world.abilities[ability].name.clone();
                let shown = self.show(ty);
                self.error(
                    "missing-ability",
                    format!("{what} needs `{name}`, which `{shown}` does not have"),
                    span,
                );
            }
            None => self.deferred.push(Deferred::Constraint {
                ty: ty.clone(),
                ability,
                span,
                what: what.to_string(),
            }),
        }
    }

    // ----------------------------------------------------------------- scopes

    fn push_scope(&mut self) {
        self.scopes.push(Scope {
            bindings: Vec::new(),
        });
    }

    fn pop_scope(&mut self) {
        let scope = self.scopes.pop().expect("a scope");
        for binding in scope.bindings {
            if !binding.used && binding.name != "self" {
                let what = match binding.kind {
                    BindingKind::Param => "the parameter",
                    BindingKind::Loop | BindingKind::Query => "the loop variable",
                    BindingKind::Pattern => "the pattern binding",
                    BindingKind::Let => "the binding",
                };
                let fix = match binding.kind {
                    BindingKind::Query => "remove it, or read it in a clause".to_string(),
                    BindingKind::Pattern => "remove it from the pattern".to_string(),
                    _ => "remove it, or use it".to_string(),
                };
                self.error_fix(
                    "unused-binding",
                    format!("{what} `{}` is never used", binding.name),
                    binding.span,
                    fix,
                );
            }
        }
    }

    fn lookup(&mut self, name: &str) -> Option<&mut Binding> {
        self.scopes
            .iter_mut()
            .rev()
            .find_map(|scope| scope.bindings.iter_mut().find(|b| b.name == name))
    }

    fn bind(&mut self, name: &Name, ty: Ty, mutable: bool, kind: BindingKind) {
        if name.text != "self" {
            if let Some(existing) = self.lookup(&name.text) {
                let earlier = existing.span;
                let _ = earlier;
                self.error_fix(
                    "shadowing",
                    format!("`{}` is already bound in this function", name.text),
                    name.span,
                    "choose another name; a name is bound once per function",
                );
            } else if self.world.modules[self.module]
                .imports
                .contains_key(&name.text)
            {
                self.error_fix(
                    "shadowing",
                    format!("`{}` is the namespace of an import", name.text),
                    name.span,
                    "choose another name, or rename the import with `as`",
                );
            } else if self
                .world
                .lookup_function(self.module, &name.text)
                .is_some()
            {
                self.error_fix(
                    "shadowing",
                    format!("`{}` is a function of this module", name.text),
                    name.span,
                    "choose another name",
                );
            }
        }
        self.scopes
            .last_mut()
            .expect("a scope")
            .bindings
            .push(Binding {
                name: name.text.clone(),
                ty,
                mutable,
                used: false,
                span: name.span,
                kind,
            });
    }

    // ------------------------------------------------------------- functions

    /// Check a function body: parameters, statements, the return, unused
    /// bindings and the deferred constraints.
    pub fn check_function(&mut self, id: FunctionId, function: &Function) {
        let info = &self.world.functions[id];
        self.owner = info.body;
        self.context = Context {
            name: info.name.clone(),
            returns: info.returns.clone(),
            fails: info.fails.clone(),
            fails_any: false,
            needs: info.needs.clone(),
            is_test: false,
        };
        self.vars.clear();
        self.deferred.clear();
        self.push_scope();
        for ((_, ty), param) in info.params.iter().zip(&function.params) {
            let ty = match ty {
                Ty::SelfType => Ty::Error,
                other => other.clone(),
            };
            self.bind(&param.name, ty, false, BindingKind::Param);
        }
        if let Some(body) = &function.body {
            let diverges = self.check_block_in_scope(body);
            if let (Some(returns_ty), false) = (&info.returns, diverges) {
                let end = Span::new(function.span.end.saturating_sub(3), function.span.end);
                let returns = self.show(returns_ty);
                self.error_fix(
                    "missing-return",
                    format!(
                        "`{}` returns `{returns}` but can reach its end without a `return`",
                        function.name.text
                    ),
                    end,
                    "end every path with `return`, `fail with` or `crash with`",
                );
            }
        }
        self.pop_scope();
        self.finish_body();
        self.check_examples(id, function);
    }

    /// A `test` block, the `index`th item of the module.
    pub fn check_test(&mut self, index: usize, test: &Test) {
        self.owner = BodyLocation::Item(index);
        self.context = Context {
            name: format!("test {:?}", test.name),
            returns: None,
            fails: Vec::new(),
            fails_any: true,
            needs: test.needs.iter().map(Capability::from_ast).collect(),
            is_test: true,
        };
        self.vars.clear();
        self.deferred.clear();
        self.push_scope();
        self.check_block_in_scope(&test.body);
        self.pop_scope();
        self.finish_body();
    }

    /// A top-level constant, the `index`th item of the module.
    pub fn check_constant(&mut self, index: usize, constant: &Constant, ty: &Ty) {
        self.owner = BodyLocation::Item(index);
        self.context = Context {
            name: constant.name.text.clone(),
            returns: None,
            fails: Vec::new(),
            fails_any: false,
            needs: Vec::new(),
            is_test: false,
        };
        self.vars.clear();
        self.deferred.clear();
        self.push_scope();
        let info = self.infer(&constant.value, Some(ty));
        self.require_handled(&info, constant.value.span);
        self.expect(&info.ty, ty, constant.value.span, "the value");
        self.pop_scope();
        self.finish_body();
    }

    /// Example clauses: `expression is value` and `expression fails with Pattern`.
    fn check_examples(&mut self, id: FunctionId, function: &Function) {
        let info = &self.world.functions[id];
        let _ = info;
        self.owner = BodyLocation::None;
        for example in &function.docs.examples {
            self.context = Context {
                name: format!("example of {}", function.name.text),
                returns: None,
                fails: Vec::new(),
                fails_any: false,
                needs: Vec::new(),
                is_test: false,
            };
            self.vars.clear();
            self.deferred.clear();
            self.push_scope();
            let result = self.infer(&example.expression, None);
            match &example.outcome {
                ExampleOutcome::Is(value) => {
                    let expected = self.infer(value, Some(&result.ty));
                    self.require_handled(&expected, value.span);
                    let value_ty = match self.resolve(&result.ty) {
                        Ty::Maybe(inner) => Ty::maybe(*inner),
                        other => other,
                    };
                    self.expect(&expected.ty, &value_ty, value.span, "the expected value");
                    self.require_ability(
                        &result.ty,
                        self.world.builtins.equal,
                        example.span,
                        "comparing the result",
                    );
                }
                ExampleOutcome::FailsWith(pattern) => {
                    if result.fails.is_empty() {
                        self.error(
                            "example-fails",
                            "`fails with` on a call that cannot fail",
                            pattern.span(),
                        );
                    } else {
                        let union = Ty::Union(result.fails.clone());
                        self.push_scope();
                        let mut covers = Vec::new();
                        self.check_pattern(pattern, &union, &mut covers);
                        // bindings in an example pattern are not read: allow them
                        if let Some(scope) = self.scopes.last_mut() {
                            for binding in &mut scope.bindings {
                                binding.used = true;
                            }
                        }
                        self.pop_scope();
                    }
                }
            }
            self.pop_scope();
            self.finish_body();
        }
    }

    /// Defaults for literal variables and the deferred ability checks.
    fn finish_body(&mut self) {
        let b = self.world.builtins.clone();
        for id in 0..self.vars.len() {
            if self.vars[id].binding.is_none() {
                let default = match self.vars[id].kind {
                    VarKind::IntegerLiteral(_) => Some(Ty::App(b.integer, Vec::new())),
                    VarKind::DecimalLiteral(_) => Some(Ty::App(b.decimal, Vec::new())),
                    VarKind::Any => None,
                };
                if let Some(default) = default {
                    self.vars[id].binding = Some(default);
                }
            }
        }
        let deferred = std::mem::take(&mut self.deferred);
        for item in deferred {
            match item {
                Deferred::Constraint {
                    ty,
                    ability,
                    span,
                    what,
                } => {
                    if let Some(false) = self.has_ability(&ty, ability) {
                        let name = self.world.abilities[ability].name.clone();
                        let shown = self.show(&ty);
                        self.error(
                            "missing-ability",
                            format!("{what} needs `{name}`, which `{shown}` does not have"),
                            span,
                        );
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------- statements

    fn check_block_in_scope(&mut self, block: &Block) -> bool {
        let mut diverges = false;
        let mut reported_unreachable = false;
        for statement in &block.statements {
            if diverges && !reported_unreachable {
                self.warning(
                    "unreachable",
                    "this statement can never run",
                    statement.span,
                );
                reported_unreachable = true;
            }
            if self.check_statement(statement) {
                diverges = true;
            }
        }
        diverges
    }

    /// A nested block in its own scope; returns whether it always leaves.
    fn check_block(&mut self, block: &Block) -> bool {
        self.push_scope();
        let diverges = self.check_block_in_scope(block);
        self.pop_scope();
        diverges
    }

    /// Returns whether the statement always leaves (return, fail, crash,
    /// break, continue, or a conditional whose every branch does).
    fn check_statement(&mut self, statement: &Stmt) -> bool {
        match &statement.kind {
            StmtKind::Let {
                mutable,
                name,
                ty,
                value,
            } => {
                let declared = ty.as_ref().map(|t| self.resolve_type(t));
                let info = self.infer(value, declared.as_ref());
                self.require_handled(&info, value.span);
                let ty = match declared {
                    Some(declared) => {
                        self.expect(&info.ty, &declared, value.span, "the value");
                        declared
                    }
                    None => {
                        if matches!(self.resolve(&info.ty), Ty::Unit) {
                            self.error(
                                "type-mismatch",
                                "this call returns nothing, so there is no value to bind",
                                value.span,
                            );
                            Ty::Error
                        } else {
                            info.ty.clone()
                        }
                    }
                };
                self.bind(name, ty, *mutable, BindingKind::Let);
                false
            }
            StmtKind::Set { name, value } => {
                let target = match self.lookup(&name.text) {
                    Some(binding) => Some((binding.ty.clone(), binding.mutable)),
                    None => None,
                };
                match target {
                    None => {
                        self.error(
                            "unknown-name",
                            format!("there is no binding named `{}`", name.text),
                            name.span,
                        );
                        self.infer(value, None);
                    }
                    Some((ty, mutable)) => {
                        if !mutable {
                            self.error_fix(
                                "immutable-binding",
                                format!("`{}` cannot be changed", name.text),
                                name.span,
                                format!("declare it with `let mutable {}`", name.text),
                            );
                        }
                        let info = self.infer(value, Some(&ty));
                        self.require_handled(&info, value.span);
                        self.expect(&info.ty, &ty, value.span, "the new value");
                    }
                }
                false
            }
            StmtKind::If {
                branches,
                otherwise,
            } => {
                let mut all_diverge = true;
                for (condition, block) in branches {
                    self.check_condition(condition);
                    if !self.check_block(block) {
                        all_diverge = false;
                    }
                }
                match otherwise {
                    Some(block) => {
                        if !self.check_block(block) {
                            all_diverge = false;
                        }
                        all_diverge
                    }
                    None => false,
                }
            }
            StmtKind::Match {
                subject,
                arms,
                otherwise,
            } => {
                let (subject_ty, covers_needed) = self.match_subject(subject);
                let saved_errors = self.current_errors.take();
                self.current_errors = covers_needed.as_ref().map(|e| Ty::Union(e.clone()));
                let mut all_diverge = true;
                let mut covers = Vec::new();
                for arm in arms {
                    self.push_scope();
                    let mut arm_covers = Vec::new();
                    self.check_pattern(&arm.pattern, &subject_ty, &mut arm_covers);
                    if let Some(guard) = &arm.guard {
                        self.check_condition(guard);
                    } else {
                        covers.extend(arm_covers);
                    }
                    if !self.check_block_in_scope(&arm.body) {
                        all_diverge = false;
                    }
                    self.pop_scope();
                }
                self.current_errors = saved_errors;
                let exhaustive = match otherwise {
                    Some(block) => {
                        if !self.check_block(block) {
                            all_diverge = false;
                        }
                        true
                    }
                    None => {
                        self.check_exhaustive(&subject_ty, &covers, covers_needed, statement.span)
                    }
                };
                exhaustive && all_diverge
            }
            StmtKind::ForEach {
                bindings,
                source,
                filter,
                order,
                body,
            } => {
                let item_types = self.loop_items(bindings, source);
                self.push_scope();
                for (name, ty) in bindings.iter().zip(item_types) {
                    self.bind(name, ty, false, BindingKind::Loop);
                }
                if let Some(filter) = filter {
                    self.check_condition(filter);
                }
                if let Some(order) = order {
                    self.check_ordering(order);
                }
                self.loop_depth += 1;
                self.check_block_in_scope(body);
                self.loop_depth -= 1;
                self.pop_scope();
                false
            }
            StmtKind::RepeatUntil { condition, body } => {
                self.check_condition(condition);
                self.loop_depth += 1;
                self.check_block(body);
                self.loop_depth -= 1;
                false
            }
            StmtKind::RunConcurrently { within, body } => {
                if let Some(within) = within {
                    self.check_within(within);
                }
                // bindings made inside are visible after the block
                self.concurrent_depth += 1;
                let diverges = self.check_block_in_scope(body);
                self.concurrent_depth -= 1;
                diverges
            }
            StmtKind::Return(value) => {
                self.check_return(value.as_ref(), statement.span);
                true
            }
            StmtKind::Fail(value) => {
                self.check_fail(value.as_ref(), statement.span);
                true
            }
            StmtKind::Crash(message) => {
                let text = self.builtin(self.world.builtins.text);
                let info = self.infer(message, Some(&text));
                self.require_handled(&info, message.span);
                self.expect(&info.ty, &text, message.span, "the crash message");
                true
            }
            StmtKind::Break | StmtKind::Continue => {
                let word = if matches!(statement.kind, StmtKind::Break) {
                    "break"
                } else {
                    "continue"
                };
                if self.loop_depth == 0 {
                    self.error(
                        "outside-loop",
                        format!("`{word}` outside a loop"),
                        statement.span,
                    );
                } else if self.concurrent_depth > 0 {
                    self.error(
                        "outside-loop",
                        format!("`{word}` cannot leave a `run concurrently` block"),
                        statement.span,
                    );
                }
                true
            }
            StmtKind::Ignore(value) => {
                let before = self.effect_calls;
                let info = self.infer(value, None);
                self.require_handled(&info, value.span);
                let ty = self.resolve(&info.ty);
                if matches!(ty, Ty::Unit) {
                    self.error_fix(
                        "ignore-nothing",
                        "this call returns nothing; there is no result to ignore",
                        value.span,
                        "call it without `ignore`",
                    );
                } else if self.effect_calls == before && !matches!(ty, Ty::Error | Ty::Never) {
                    // decision R6: a discarded pure result is dead code
                    let fix = self.ignored_pure_fix(value);
                    self.error_fix(
                        "ignore-pure",
                        "this call has no effects, so discarding its result does nothing",
                        value.span,
                        fix,
                    );
                }
                false
            }
            StmtKind::Check(value) => {
                if !self.context.is_test {
                    self.error(
                        "check-outside-test",
                        "`check` belongs in a `test` block",
                        statement.span,
                    );
                }
                self.check_condition(value);
                false
            }
            StmtKind::Expression(value) => {
                let info = self.infer(value, None);
                self.require_handled(&info, value.span);
                let ty = self.resolve(&info.ty);
                if !matches!(ty, Ty::Unit | Ty::Error | Ty::Never) {
                    let shown = self.show(&ty);
                    let fix = self.unused_result_fix(value);
                    self.error_fix(
                        "unused-result",
                        format!("the result (`{shown}`) is not used"),
                        value.span,
                        fix,
                    );
                }
                false
            }
        }
    }

    /// The fix for an unused result: `set x to x.method(...)` when the call is
    /// a method on a mutable binding (decision J15), else `let` or `ignore`.
    fn unused_result_fix(&mut self, value: &Expr) -> String {
        match self.mutable_method_call(value) {
            Some((receiver, method)) => format!(
                "write `set {receiver} to {receiver}.{method}(...)`, or bind it with `let`, or discard it with `ignore`"
            ),
            None => "bind it with `let`, or discard it with `ignore`".to_string(),
        }
    }

    /// The fix for `ignore` of a pure result (decision R6): the same `set`
    /// proposal, else bind or remove.
    fn ignored_pure_fix(&mut self, value: &Expr) -> String {
        match self.mutable_method_call(value) {
            Some((receiver, method)) => {
                format!("write `set {receiver} to {receiver}.{method}(...)`, or remove the call")
            }
            None => "bind the result with `let`, or remove the call".to_string(),
        }
    }

    /// The receiver and method of `x.method(...)` when `x` is a mutable binding.
    fn mutable_method_call(&mut self, value: &Expr) -> Option<(String, String)> {
        if let Some(Expr {
            kind: ExprKind::Member { base, name },
            ..
        }) = callee_of(value)
        {
            if let ExprKind::Name(receiver) = &base.kind {
                if self.lookup(&receiver.text).is_some_and(|b| b.mutable) {
                    return Some((receiver.text.clone(), name.text.clone()));
                }
            }
        }
        None
    }

    fn check_condition(&mut self, condition: &Expr) {
        let boolean = self.builtin(self.world.builtins.boolean);
        let info = self.infer(condition, Some(&boolean));
        self.require_handled(&info, condition.span);
        self.expect(&info.ty, &boolean, condition.span, "the condition");
    }

    fn check_within(&mut self, within: &Expr) {
        let duration = self.builtin(self.world.builtins.duration);
        let info = self.infer(within, Some(&duration));
        self.require_handled(&info, within.span);
        self.expect(&info.ty, &duration, within.span, "the deadline");
        let timed_out = self.builtin(self.world.builtins.timed_out);
        if !self.context.fails_any
            && !self
                .context
                .fails
                .iter()
                .any(|f| self.same_type(f, &timed_out))
        {
            self.error_fix(
                "error-not-declared",
                "a `within` deadline can fail with `TimedOut`, which this function does not declare",
                within.span,
                "add `or fails with TimedOut` to the signature",
            );
        }
    }

    fn check_ordering(&mut self, order: &Ordering) {
        let info = self.infer(&order.key, None);
        self.require_handled(&info, order.key.span);
        self.require_ability(
            &info.ty,
            self.world.builtins.compare,
            order.key.span,
            "`sorted by`",
        );
    }

    fn check_return(&mut self, value: Option<&Expr>, span: Span) {
        match (value, self.context.returns.clone()) {
            (None, None) => {}
            (None, Some(returns)) => {
                let shown = self.show(&returns);
                self.error(
                    "missing-value",
                    format!("`return` needs a value of type `{shown}`"),
                    span,
                );
            }
            (Some(value), None) => {
                self.infer(value, None);
                if self.context.is_test {
                    self.error("return-value", "a test returns nothing", value.span);
                } else {
                    self.error_fix(
                        "return-value",
                        format!(
                            "`{}` returns nothing, so `return` takes no value",
                            self.context.name
                        ),
                        value.span,
                        "add a `returns` clause to the signature, or drop the value",
                    );
                }
            }
            (Some(value), Some(returns)) => {
                let info = self.infer(value, Some(&returns));
                self.require_handled(&info, value.span);
                self.expect(&info.ty, &returns, value.span, "the returned value");
            }
        }
    }

    fn check_fail(&mut self, value: Option<&Expr>, span: Span) {
        match value {
            None => {
                if !self.context.fails_any {
                    self.error_fix(
                        "bare-fail",
                        "`fail` without a value is only allowed after `otherwise`",
                        span,
                        "write `fail with Error(...)`",
                    );
                }
            }
            Some(value) => {
                let info = self.infer(value, None);
                self.require_handled(&info, value.span);
                self.check_error_declared(&info.ty, value.span);
            }
        }
    }

    /// The error value or union must be among the declared failure types.
    fn check_error_declared(&mut self, ty: &Ty, span: Span) {
        if self.context.fails_any {
            return;
        }
        let members: Vec<Ty> = match self.resolve(ty) {
            Ty::Union(members) => members,
            other => vec![other],
        };
        for member in members {
            if matches!(member, Ty::Error) {
                continue;
            }
            let declared = self.context.fails.clone();
            let found = declared.iter().any(|f| self.assign(&member, f, span));
            if !found {
                let shown = self.show(&member);
                let fix = if self.context.is_test {
                    "a test may fail with any error".to_string()
                } else {
                    format!("add `or fails with {shown}` to the signature, or translate it with `otherwise fail with ...`")
                };
                self.error_fix(
                    "error-not-declared",
                    format!(
                        "`{}` does not declare that it fails with `{shown}`",
                        self.context.name
                    ),
                    span,
                    fix,
                );
            }
        }
    }

    /// A fallible call whose errors nothing handled is an error (sketch
    /// section 9: `otherwise` is mandatory).
    fn require_handled(&mut self, info: &Info, span: Span) {
        if !info.fails.is_empty() {
            let shown: Vec<String> = info.fails.iter().map(|f| self.show(f)).collect();
            self.error_fix(
                "missing-otherwise",
                format!("this call can fail with {}", shown.join(" or ")),
                span,
                "add `otherwise fail`, `otherwise <value>`, or handle it with `match`",
            );
        }
    }

    // ------------------------------------------------------------ loops, maps

    /// The item types a loop or query header binds for a source.
    fn loop_items(&mut self, bindings: &[Name], source: &Expr) -> Vec<Ty> {
        let info = self.infer(source, None);
        self.require_handled(&info, source.span);
        let b = self.world.builtins.clone();
        let ty = self.resolve(&info.ty);
        let (item, pair): (Ty, Option<(Ty, Ty)>) = match &ty {
            Ty::App(id, args) if *id == b.list || *id == b.set => {
                let item = args[0].clone();
                let pair = match self.resolve(&item) {
                    Ty::App(pid, pargs) if pid == b.pair => {
                        Some((pargs[0].clone(), pargs[1].clone()))
                    }
                    _ => None,
                };
                (item, pair)
            }
            Ty::App(id, args) if *id == b.map => (
                Ty::App(b.pair, vec![args[0].clone(), args[1].clone()]),
                Some((args[0].clone(), args[1].clone())),
            ),
            Ty::App(id, _) if *id == b.range => (self.builtin(b.integer), None),
            Ty::App(id, _) if *id == b.text => (self.builtin(b.text), None),
            Ty::Error => (Ty::Error, Some((Ty::Error, Ty::Error))),
            Ty::Maybe(_) => {
                self.mismatch(
                    &ty,
                    &Ty::App(b.list, vec![Ty::Error]),
                    source.span,
                    "the source",
                );
                (Ty::Error, Some((Ty::Error, Ty::Error)))
            }
            _ => {
                let shown = self.show(&ty);
                self.error(
                    "type-mismatch",
                    format!("`{shown}` cannot be iterated; a list, set, map, range or text can"),
                    source.span,
                );
                (Ty::Error, Some((Ty::Error, Ty::Error)))
            }
        };
        match bindings.len() {
            1 => vec![item],
            2 => {
                match pair {
                    Some((left, right)) => vec![left, right],
                    None => {
                        let shown = self.show(&item);
                        self.error("type-mismatch", format!("two loop variables need pairs or a map, but the items are `{shown}`"), source.span);
                        vec![Ty::Error, Ty::Error]
                    }
                }
            }
            n => {
                self.error(
                    "loop-variables",
                    "a loop binds one variable, or two for pairs",
                    source.span,
                );
                vec![Ty::Error; n]
            }
        }
    }

    // ------------------------------------------------------------- expressions

    /// Infer an expression. `expected` guides literals and generic results.
    fn infer(&mut self, expr: &Expr, expected: Option<&Ty>) -> Info {
        let b = self.world.builtins.clone();
        match &expr.kind {
            ExprKind::Integer(digits) => {
                let literal = refine::literal_of(expr);
                if let Some(expected) = expected {
                    let resolved = self.resolve(expected);
                    if !matches!(resolved, Ty::Var(_)) && self.is_numeric(&resolved) {
                        if let Some(literal) = &literal {
                            self.check_subtype_refinements(&resolved, literal, expr.span);
                        }
                        let kind = VarKind::IntegerLiteral(literal);
                        if !self.literal_fits(&kind, None, &resolved, expr.span) {
                            let shown = self.show(&resolved);
                            self.error(
                                "type-mismatch",
                                format!(
                                    "the literal `{digits}` is an Integer, but `{shown}` is needed"
                                ),
                                expr.span,
                            );
                        }
                        return Info::plain(resolved);
                    }
                }
                Info::plain(self.fresh(VarKind::IntegerLiteral(literal)))
            }
            ExprKind::Decimal(digits) => {
                let literal = refine::literal_of(expr);
                if let Some(expected) = expected {
                    let resolved = self.resolve(expected);
                    if !matches!(resolved, Ty::Var(_)) && self.is_numeric(&resolved) {
                        let kind = VarKind::DecimalLiteral(literal.clone());
                        if !self.literal_fits(&kind, None, &resolved, expr.span) {
                            let shown = self.show(&resolved);
                            self.error_fix("type-mismatch", format!("the literal `{digits}` is a Decimal, but `{shown}` is needed"), expr.span, "an Integer value is converted with `.to_decimal()`, never the other way");
                        } else if let Some(literal) = &literal {
                            self.check_subtype_refinements(&resolved, literal, expr.span);
                        }
                        return Info::plain(resolved);
                    }
                }
                Info::plain(self.fresh(VarKind::DecimalLiteral(literal)))
            }
            ExprKind::Text { pieces, .. } => {
                for piece in pieces {
                    if let TextPiece::Hole(hole) = piece {
                        let info = self.infer(hole, None);
                        self.require_handled(&info, hole.span);
                        if let Ty::Maybe(_) = self.resolve(&info.ty) {
                            self.mismatch(
                                &info.ty,
                                &self.builtin(b.text),
                                hole.span,
                                "the interpolated value",
                            );
                        } else {
                            self.require_ability(&info.ty, b.to_text, hole.span, "interpolation");
                        }
                    }
                }
                let text = self.builtin(b.text);
                // a text literal is also a literal of a refined Text subtype when the condition holds
                if let Some(expected) = expected {
                    let resolved = self.resolve(expected);
                    if self.is_builtin(&self.base_of(&resolved), b.text)
                        && !self.is_builtin(&resolved, b.text)
                    {
                        if let Some(literal) = refine::literal_of(expr) {
                            if self.check_subtype_refinements(&resolved, &literal, expr.span) {
                                return Info::plain(resolved);
                            }
                            return Info::plain(Ty::Error);
                        }
                    }
                }
                Info::plain(text)
            }
            ExprKind::RawText(value) => {
                if let Some(error) = refine::regex_error(value) {
                    let _ = error;
                }
                Info::plain(self.builtin(b.text))
            }
            ExprKind::Boolean(_) => Info::plain(self.builtin(b.boolean)),
            ExprKind::Nothing => match expected.map(|e| self.resolve(e)) {
                Some(Ty::Maybe(inner)) => Info::plain(Ty::maybe(*inner)),
                _ => {
                    let inner = self.fresh(VarKind::Any);
                    Info::plain(Ty::maybe(inner))
                }
            },
            ExprKind::SelfValue => match self.lookup("self") {
                Some(binding) => {
                    binding.used = true;
                    Info::plain(binding.ty.clone())
                }
                None => {
                    self.error(
                        "unknown-name",
                        "`self` is only bound in a method",
                        expr.span,
                    );
                    Info::plain(Ty::Error)
                }
            },
            ExprKind::Name(name) => self.infer_name(name),
            ExprKind::TypeName(name) => self.infer_bare_type_name(name, expected),
            ExprKind::Member { base, name } => self.infer_member(base, name, expr.span),
            ExprKind::Call { callee, args } => self.infer_call(callee, args, expr.span, expected),
            ExprKind::Construct { name, args } => {
                self.infer_construct(name, args, expr.span, expected)
            }
            ExprKind::List(items) => {
                let item_expected = match expected.map(|e| self.resolve(e)) {
                    Some(Ty::App(id, args)) if id == b.list => Some(args[0].clone()),
                    _ => None,
                };
                let item_ty = item_expected.unwrap_or_else(|| self.fresh(VarKind::Any));
                for item in items {
                    let info = self.infer(item, Some(&item_ty));
                    self.require_handled(&info, item.span);
                    self.expect(&info.ty, &item_ty, item.span, "the item");
                }
                Info::plain(Ty::App(b.list, vec![item_ty]))
            }
            ExprKind::Map(entries) => {
                let (key_ty, value_ty) = match expected.map(|e| self.resolve(e)) {
                    Some(Ty::App(id, args)) if id == b.map => (args[0].clone(), args[1].clone()),
                    _ => (self.fresh(VarKind::Any), self.fresh(VarKind::Any)),
                };
                for (key, value) in entries {
                    let key_info = self.infer(key, Some(&key_ty));
                    self.require_handled(&key_info, key.span);
                    self.expect(&key_info.ty, &key_ty, key.span, "the key");
                    let value_info = self.infer(value, Some(&value_ty));
                    self.require_handled(&value_info, value.span);
                    self.expect(&value_info.ty, &value_ty, value.span, "the value");
                }
                self.require_ability(&key_ty, b.hash, expr.span, "a map key");
                Info::plain(Ty::App(b.map, vec![key_ty, value_ty]))
            }
            ExprKind::Range { from, to, by } => {
                let integer = self.builtin(b.integer);
                for bound in [from, to].into_iter().chain(by.iter()) {
                    let info = self.infer(bound, Some(&integer));
                    self.require_handled(&info, bound.span);
                    self.expect(&info.ty, &integer, bound.span, "the range bound");
                }
                Info::plain(self.builtin(b.range))
            }
            ExprKind::Not(inner) => {
                self.check_condition(inner);
                Info::plain(self.builtin(b.boolean))
            }
            ExprKind::Binary { op, left, right } => {
                self.infer_binary(*op, left, right, expr.span, expected)
            }
            ExprKind::With { base, updates } => {
                let base_info = self.infer(base, expected);
                self.require_handled(&base_info, base.span);
                let ty = self.resolve(&base_info.ty);
                match self.record_fields(&ty) {
                    Some(fields) => {
                        for update in updates {
                            let Some(field_name) = &update.name else {
                                self.error(
                                    "argument-name",
                                    "`with` names every field it changes",
                                    update.span,
                                );
                                self.infer(&update.value, None);
                                continue;
                            };
                            match fields.iter().find(|(n, _, _)| n == &field_name.text) {
                                Some((_, field_ty, refinement)) => {
                                    let info = self.infer(&update.value, Some(field_ty));
                                    self.require_handled(&info, update.value.span);
                                    self.expect(
                                        &info.ty,
                                        field_ty,
                                        update.value.span,
                                        "the new value",
                                    );
                                    if refinement.is_some() {
                                        self.check_field_refinement(
                                            field_name,
                                            refinement.as_ref().unwrap(),
                                            &update.value,
                                            &mut Vec::new(),
                                        );
                                    }
                                }
                                None => {
                                    let shown = self.show(&ty);
                                    self.error(
                                        "unknown-field",
                                        format!(
                                            "`{shown}` has no field named `{}`",
                                            field_name.text
                                        ),
                                        field_name.span,
                                    );
                                    self.infer(&update.value, None);
                                }
                            }
                        }
                    }
                    None => {
                        let shown = self.show(&ty);
                        if !ty.is_error() {
                            self.error(
                                "type-mismatch",
                                format!("`with` updates a record, but this is `{shown}`"),
                                base.span,
                            );
                        }
                        for update in updates {
                            self.infer(&update.value, None);
                        }
                    }
                }
                Info::plain(ty)
            }
            ExprKind::Otherwise { value, fallback } => {
                self.infer_otherwise(value, fallback, expr.span, expected)
            }
            ExprKind::If {
                branches,
                otherwise,
            } => {
                let mut result: Option<Ty> = expected.cloned();
                for (condition, outcome) in branches {
                    self.check_condition(condition);
                    self.check_outcome(outcome, &mut result);
                }
                self.check_outcome(otherwise, &mut result);
                Info::plain(result.unwrap_or(Ty::Never))
            }
            ExprKind::Match {
                subject,
                arms,
                otherwise,
            } => {
                let (subject_ty, covers_needed) = self.match_subject(subject);
                let saved_errors = self.current_errors.take();
                self.current_errors = covers_needed.as_ref().map(|e| Ty::Union(e.clone()));
                let mut result: Option<Ty> = expected.cloned();
                let mut covers = Vec::new();
                for arm in arms {
                    self.push_scope();
                    let mut arm_covers = Vec::new();
                    self.check_pattern(&arm.pattern, &subject_ty, &mut arm_covers);
                    if let Some(guard) = &arm.guard {
                        self.check_condition(guard);
                    } else {
                        covers.extend(arm_covers);
                    }
                    self.check_outcome(&arm.body, &mut result);
                    self.pop_scope();
                }
                self.current_errors = saved_errors;
                match otherwise {
                    Some(outcome) => self.check_outcome(outcome, &mut result),
                    None => {
                        self.check_exhaustive(&subject_ty, &covers, covers_needed, expr.span);
                    }
                }
                Info::plain(result.unwrap_or(Ty::Never))
            }
            ExprKind::Query(query) => self.infer_query(query, expected),
            ExprKind::Paren(inner) => self.infer(inner, expected),
        }
    }

    /// Infer a value: a plain, handled expression of the expected type.
    fn infer_value(&mut self, expr: &Expr, expected: &Ty, what: &str) {
        let info = self.infer(expr, Some(expected));
        self.require_handled(&info, expr.span);
        self.expect(&info.ty, expected, expr.span, what);
    }

    /// A branch of an `if` or `match` expression: a value or a way out.
    fn check_outcome(&mut self, outcome: &Outcome, result: &mut Option<Ty>) {
        match outcome {
            Outcome::Value(value) => {
                let info = self.infer(value, result.as_ref());
                self.require_handled(&info, value.span);
                match result {
                    Some(ty) => {
                        let ty = ty.clone();
                        self.expect(&info.ty, &ty, value.span, "this branch");
                    }
                    None => *result = Some(info.ty),
                }
            }
            Outcome::Fail(value, span) => self.check_fail(value.as_ref(), *span),
            Outcome::Return(value, span) => self.check_return(value.as_ref(), *span),
            Outcome::Crash(message, _) => {
                let text = self.builtin(self.world.builtins.text);
                self.infer_value(message, &text, "the crash message");
            }
            Outcome::Break(span) | Outcome::Continue(span) => {
                if self.loop_depth == 0 {
                    self.error(
                        "outside-loop",
                        "`break` or `continue` outside a loop",
                        *span,
                    );
                }
            }
        }
    }

    fn infer_name(&mut self, name: &Name) -> Info {
        if let Some(binding) = self.lookup(&name.text) {
            binding.used = true;
            return Info::plain(binding.ty.clone());
        }
        if let Some(constant) = self.world.modules[self.module].constants.get(&name.text) {
            let ty = constant.ty.clone();
            self.record(Target::Constant(self.module, name.text.clone()), name.span);
            return Info::plain(ty);
        }
        if let Some(function) = self.world.lookup_function(self.module, &name.text) {
            self.record(Target::Function(function), name.span);
            return Info {
                ty: self.function_type(function),
                fails: Vec::new(),
                function: Some(function),
            };
        }
        if self.world.modules[self.module]
            .imports
            .contains_key(&name.text)
        {
            self.error_fix(
                "unknown-name",
                format!("`{}` is a module, not a value", name.text),
                name.span,
                format!("call one of its functions: `{}.function(...)`", name.text),
            );
            return Info::plain(Ty::Error);
        }
        let message = format!(
            "there is no binding, constant or function named `{}`",
            name.text
        );
        match self.suggest_module_import(&name.text) {
            Some(fix) => self.error_fix("unknown-name", message, name.span, fix),
            None => self.error("unknown-name", message, name.span),
        }
        Info::plain(Ty::Error)
    }

    /// `environment.arguments()` without `import std.environment`: name the import.
    fn suggest_module_import(&self, name: &str) -> Option<String> {
        self.world
            .modules
            .iter()
            .find(|m| m.is_library && m.name.rsplit('.').next() == Some(name))
            .map(|m| format!("write `import {}` at the top of the module", m.name))
    }

    /// The type of a function used as a value (passed by name).
    fn function_type(&self, id: FunctionId) -> Ty {
        let info = &self.world.functions[id];
        Ty::Function(Box::new(FunctionTy {
            params: info.params.iter().map(|(_, t)| t.clone()).collect(),
            returns: info.returns.clone(),
            fails: info.fails.clone(),
            needs: info.needs.clone(),
        }))
    }

    /// A bare `Red` or `Point`: a variant without fields.
    fn infer_bare_type_name(&mut self, name: &TypeName, expected: Option<&Ty>) -> Info {
        let candidates = self.world.lookup_variant(self.module, &name.text);
        let chosen = match candidates.len() {
            0 => None,
            1 => Some(candidates[0]),
            _ => {
                // the expected type decides between sum types sharing a variant name
                let wanted = expected.map(|e| self.resolve(e));
                match wanted {
                    Some(Ty::App(id, _)) if candidates.iter().any(|(t, _)| *t == id) => {
                        candidates.into_iter().find(|(t, _)| *t == id)
                    }
                    _ => {
                        let names: Vec<String> = candidates
                            .iter()
                            .map(|(t, _)| self.world.types[*t].name.clone())
                            .collect();
                        self.error(
                            "ambiguous-variant",
                            format!(
                                "`{}` is a variant of {}; the context does not say which",
                                name.text,
                                names.join(" and ")
                            ),
                            name.span,
                        );
                        return Info::plain(Ty::Error);
                    }
                }
            }
        };
        match chosen {
            Some((type_id, index)) => {
                self.record(Target::Variant(type_id, index), name.span);
                let TypeKindInfo::Sum(variants) = &self.world.types[type_id].kind else {
                    unreachable!()
                };
                if !variants[index].fields.is_empty() {
                    let fields: Vec<String> = variants[index]
                        .fields
                        .iter()
                        .map(|f| format!("{}: ...", f.name))
                        .collect();
                    self.error_fix(
                        "missing-fields",
                        format!("`{}` has fields", name.text),
                        name.span,
                        format!("write `{}({})`", name.text, fields.join(", ")),
                    );
                }
                let args: Vec<Ty> = self.world.types[type_id]
                    .params
                    .iter()
                    .map(|_| self.fresh(VarKind::Any))
                    .collect();
                Info::plain(Ty::App(type_id, args))
            }
            None => {
                if self.world.lookup_type(self.module, &name.text).is_some() {
                    self.error_fix(
                        "type-as-value",
                        format!("`{}` is a type, not a value", name.text),
                        name.span,
                        format!("construct one with `{}(...)`", name.text),
                    );
                } else {
                    let message = format!("unknown name `{}`", name.text);
                    match self.world.suggest_import(self.module, &name.text) {
                        Some(fix) => self.error_fix("unknown-name", message, name.span, fix),
                        None => self.error("unknown-name", message, name.span),
                    }
                }
                Info::plain(Ty::Error)
            }
        }
    }

    /// The fields of a record type with its arguments applied, or of a pair.
    fn record_fields(&self, ty: &Ty) -> Option<Vec<(String, Ty, Option<Expr>)>> {
        let b = &self.world.builtins;
        match self.resolve(ty) {
            Ty::App(id, args) if id == b.pair => Some(vec![
                ("left".to_string(), args[0].clone(), None),
                ("right".to_string(), args[1].clone(), None),
            ]),
            Ty::App(id, args) => {
                let info = &self.world.types[id];
                let TypeKindInfo::Record(fields) = &info.kind else {
                    return None;
                };
                let params = &info.params;
                Some(
                    fields
                        .iter()
                        .map(|f| {
                            let ty = f.ty.substitute(&|p| {
                                params.iter().position(|q| *q == p).map(|i| args[i].clone())
                            });
                            (f.name.clone(), ty, f.refinement.clone())
                        })
                        .collect(),
                )
            }
            _ => None,
        }
    }

    fn infer_member(&mut self, base: &Expr, name: &Name, span: Span) -> Info {
        // a module function used as a value: `module.function`
        if let ExprKind::Name(namespace) = &base.kind {
            if self.lookup(&namespace.text).is_none() {
                if let Some(&target) = self.world.modules[self.module].imports.get(&namespace.text)
                {
                    return match self.world.lookup_function(target, &name.text) {
                        Some(function) => {
                            self.record(Target::Function(function), name.span);
                            Info {
                                ty: self.function_type(function),
                                fails: Vec::new(),
                                function: Some(function),
                            }
                        }
                        None => {
                            let module_name = self.world.modules[target].name.clone();
                            self.error(
                                "unknown-name",
                                format!("`{module_name}` has no function named `{}`", name.text),
                                name.span,
                            );
                            Info::plain(Ty::Error)
                        }
                    };
                }
            }
        }
        let base_info = self.infer(base, None);
        self.require_handled(&base_info, base.span);
        let ty = self.resolve(&base_info.ty);
        if let Ty::Maybe(_) = ty {
            self.mismatch(&ty, &Ty::Error, span, "this value");
            return Info::plain(Ty::Error);
        }
        if ty.is_error() {
            return Info::plain(Ty::Error);
        }
        match self.record_fields(&ty) {
            Some(fields) => match fields.into_iter().find(|(n, _, _)| *n == name.text) {
                Some((_, field_ty, _)) => Info::plain(field_ty),
                None => {
                    let shown = self.show(&ty);
                    let fix = if self.find_method(&ty, &name.text).is_some() {
                        Some(format!(
                            "`{}` is a method: call it with `.{}()`",
                            name.text, name.text
                        ))
                    } else {
                        None
                    };
                    match fix {
                        Some(fix) => self.error_fix(
                            "unknown-field",
                            format!("`{shown}` has no field named `{}`", name.text),
                            name.span,
                            fix,
                        ),
                        None => self.error(
                            "unknown-field",
                            format!("`{shown}` has no field named `{}`", name.text),
                            name.span,
                        ),
                    }
                    Info::plain(Ty::Error)
                }
            },
            None => {
                let shown = self.show(&ty);
                let fix = if self.find_method(&ty, &name.text).is_some() {
                    format!(
                        "`{}` is a method: call it with `.{}()`",
                        name.text, name.text
                    )
                } else {
                    "fields belong to records; other values have methods, called with parentheses"
                        .to_string()
                };
                self.error_fix(
                    "unknown-field",
                    format!("`{shown}` has no field named `{}`", name.text),
                    name.span,
                    fix,
                );
                Info::plain(Ty::Error)
            }
        }
    }

    // ------------------------------------------------------------------ calls

    fn infer_call(
        &mut self,
        callee: &Expr,
        args: &[Arg],
        span: Span,
        expected: Option<&Ty>,
    ) -> Info {
        match &callee.kind {
            ExprKind::Name(name) => {
                // a function-typed binding (a parameter passed by name)
                let binding_ty = self.lookup(&name.text).map(|b| {
                    b.used = true;
                    b.ty.clone()
                });
                if let Some(ty) = binding_ty {
                    return match self.resolve(&ty) {
                        Ty::Function(function) => {
                            self.call_function_type(&function, args, span, &name.text)
                        }
                        Ty::Error => {
                            for arg in args {
                                self.infer(&arg.value, None);
                            }
                            Info::plain(Ty::Error)
                        }
                        other => {
                            let shown = self.show(&other);
                            self.error(
                                "not-callable",
                                format!("`{}` is `{shown}`, not a function", name.text),
                                name.span,
                            );
                            for arg in args {
                                self.infer(&arg.value, None);
                            }
                            Info::plain(Ty::Error)
                        }
                    };
                }
                match self.world.lookup_function(self.module, &name.text) {
                    Some(function) => {
                        self.call_known(function, None, args, span, name.span, expected)
                    }
                    None => {
                        let message =
                            format!("there is no function named `{}` in this module", name.text);
                        let fix = self.suggest_function(&name.text);
                        match fix {
                            Some(fix) => {
                                self.error_fix("unknown-function", message, name.span, fix)
                            }
                            None => self.error("unknown-function", message, name.span),
                        }
                        for arg in args {
                            self.infer(&arg.value, None);
                        }
                        Info::plain(Ty::Error)
                    }
                }
            }
            ExprKind::Member { base, name } => {
                if let ExprKind::Name(namespace) = &base.kind {
                    if self.lookup(&namespace.text).is_none() {
                        if let Some(&target) =
                            self.world.modules[self.module].imports.get(&namespace.text)
                        {
                            return match self.world.lookup_function(target, &name.text) {
                                Some(function) => {
                                    self.call_known(function, None, args, span, name.span, expected)
                                }
                                None => {
                                    let module_name = self.world.modules[target].name.clone();
                                    let fix = self.suggest_module_function(target, &name.text);
                                    match fix {
                                        Some(fix) => self.error_fix(
                                            "unknown-function",
                                            format!(
                                                "`{module_name}` has no function named `{}`",
                                                name.text
                                            ),
                                            name.span,
                                            fix,
                                        ),
                                        None => self.error(
                                            "unknown-function",
                                            format!(
                                                "`{module_name}` has no function named `{}`",
                                                name.text
                                            ),
                                            name.span,
                                        ),
                                    }
                                    for arg in args {
                                        self.infer(&arg.value, None);
                                    }
                                    Info::plain(Ty::Error)
                                }
                            };
                        }
                    }
                }
                if let ExprKind::Name(namespace) = &base.kind {
                    if self.lookup(&namespace.text).is_none()
                        && self
                            .world
                            .lookup_function(self.module, &namespace.text)
                            .is_none()
                        && !self.world.modules[self.module]
                            .constants
                            .contains_key(&namespace.text)
                    {
                        if let Some(fix) = self.suggest_module_import(&namespace.text) {
                            self.error_fix(
                                "unknown-name",
                                format!("`{}` is not imported", namespace.text),
                                namespace.span,
                                fix,
                            );
                            for arg in args {
                                self.infer(&arg.value, None);
                            }
                            return Info::plain(Ty::Error);
                        }
                    }
                }
                let receiver = self.infer(base, None);
                self.require_handled(&receiver, base.span);
                self.infer_method_call(&receiver.ty, base.span, name, args, span, expected)
            }
            _ => {
                self.error(
                    "not-callable",
                    "only a named function or a method can be called",
                    callee.span,
                );
                for arg in args {
                    self.infer(&arg.value, None);
                }
                Info::plain(Ty::Error)
            }
        }
    }

    /// A method call `receiver.name(args)`: a method declared for the
    /// receiver's type (or a base of it), or an ability method.
    fn infer_method_call(
        &mut self,
        receiver_ty: &Ty,
        receiver_span: Span,
        name: &Name,
        args: &[Arg],
        span: Span,
        expected: Option<&Ty>,
    ) -> Info {
        self.default_literal(receiver_ty);
        let ty = self.resolve(receiver_ty);
        if ty.is_error() {
            for arg in args {
                self.infer(&arg.value, None);
            }
            return Info::plain(Ty::Error);
        }
        if let Ty::Maybe(_) = ty {
            self.mismatch(&ty, &Ty::Error, receiver_span, "this value");
            for arg in args {
                self.infer(&arg.value, None);
            }
            return Info::plain(Ty::Error);
        }
        match self.find_method(&ty, &name.text) {
            Some(Method::Declared(function)) => {
                let function = self.choose_overload(function, &ty, &name.text, receiver_span);
                self.call_known(
                    function,
                    Some((ty, receiver_span)),
                    args,
                    span,
                    name.span,
                    expected,
                )
            }
            Some(Method::Ability(ability, index)) => {
                self.record(Target::AbilityMethod(ability, index), name.span);
                let method = &self.world.abilities[ability].methods[index];
                let params: Vec<(String, Ty)> = method
                    .params
                    .iter()
                    .map(|(n, t)| (n.clone(), t.with_self(&ty)))
                    .collect();
                let returns = method.returns.as_ref().map(|r| r.with_self(&ty));
                let fails: Vec<Ty> = method.fails.iter().map(|f| f.with_self(&ty)).collect();
                self.check_args(&params, args, span, &name.text);
                Info {
                    ty: returns.unwrap_or(Ty::Unit),
                    fails,
                    function: None,
                }
            }
            None => {
                let shown = self.show(&ty);
                let fix = self.suggest_method(&ty, &name.text);
                let message = format!("`{shown}` has no method named `{}`", name.text);
                match fix {
                    Some(fix) => self.error_fix("unknown-method", message, name.span, fix),
                    None => self.error("unknown-method", message, name.span),
                }
                for arg in args {
                    self.infer(&arg.value, None);
                }
                Info::plain(Ty::Error)
            }
        }
    }

    /// A number literal whose type nothing has fixed yet takes its default
    /// (Integer or Decimal) when a method is called on it.
    fn default_literal(&mut self, ty: &Ty) {
        if let Ty::Var(id) = self.resolve(ty) {
            let b = &self.world.builtins;
            let default = match self.vars[id].kind {
                VarKind::IntegerLiteral(_) => Some(Ty::App(b.integer, Vec::new())),
                VarKind::DecimalLiteral(_) => Some(Ty::App(b.decimal, Vec::new())),
                VarKind::Any => None,
            };
            if let Some(default) = default {
                self.vars[id].binding = Some(default);
            }
        }
    }

    /// Among several methods of one name on one type (`sum` on lists of
    /// Integer, Decimal and Float), the one whose `self` fits the receiver.
    fn choose_overload(
        &mut self,
        first: FunctionId,
        receiver: &Ty,
        name: &str,
        span: Span,
    ) -> FunctionId {
        let Some(head) = head_type(&self.resolve(receiver)) else {
            return first;
        };
        let candidates = self.world.methods_of(head, name);
        if candidates.len() <= 1 {
            return first;
        }
        for &candidate in &candidates {
            let saved: Vec<Option<Ty>> = self.vars.iter().map(|v| v.binding.clone()).collect();
            let saved_len = self.vars.len();
            let info = &self.world.functions[candidate];
            let instances: Vec<(ParamId, Ty)> = info
                .type_params
                .iter()
                .map(|&p| (p, self.fresh(VarKind::Any)))
                .collect();
            let subst = |p: ParamId| {
                instances
                    .iter()
                    .find(|(q, _)| *q == p)
                    .map(|(_, t)| t.clone())
            };
            let self_ty = info.params[0].1.substitute(&subst);
            let diagnostics_before = self.diagnostics.len();
            let fits = self.assign(receiver, &self_ty, span);
            self.diagnostics.truncate(diagnostics_before);
            // undo the trial bindings
            self.vars.truncate(saved_len);
            for (var, binding) in self.vars.iter_mut().zip(saved) {
                var.binding = binding;
            }
            if fits {
                return candidate;
            }
        }
        first
    }

    /// Find a method for a receiver type: declared methods on the type and
    /// its bases, then ability methods the type has.
    fn find_method(&self, ty: &Ty, name: &str) -> Option<Method> {
        let mut current = self.resolve(ty);
        loop {
            match &current {
                Ty::App(id, _) => {
                    if let Some(&function) = self.world.methods_of(*id, name).first() {
                        return Some(Method::Declared(function));
                    }
                    if let Some(found) = self.ability_method(&current, name) {
                        return Some(found);
                    }
                    match &self.world.types[*id].kind {
                        TypeKindInfo::Subtype { base, .. } => current = self.resolve(base),
                        _ => return None,
                    }
                }
                Ty::Param(_) | Ty::Union(_) => return self.ability_method(&current, name),
                _ => return None,
            }
        }
    }

    fn ability_method(&self, ty: &Ty, name: &str) -> Option<Method> {
        for (ability_id, ability) in self.world.abilities.iter().enumerate() {
            if let Some(index) = ability.methods.iter().position(|m| m.name == name) {
                if self.has_ability(ty, ability_id) == Some(true) {
                    return Some(Method::Ability(ability_id, index));
                }
            }
        }
        None
    }

    /// Call a declared function: instantiate its type parameters, check the
    /// arguments, record the effects, hand back the result and its errors.
    fn call_known(
        &mut self,
        id: FunctionId,
        receiver: Option<(Ty, Span)>,
        args: &[Arg],
        span: Span,
        name_span: Span,
        expected: Option<&Ty>,
    ) -> Info {
        self.record(Target::Function(id), name_span);
        let info: &FunctionInfo = &self.world.functions[id];
        let name = info.name.clone();
        let type_params = info.type_params.clone();
        // instantiate `for any` parameters with fresh variables
        let instances: Vec<(ParamId, Ty)> = type_params
            .iter()
            .map(|&p| (p, self.fresh(VarKind::Any)))
            .collect();
        let subst = |p: ParamId| {
            instances
                .iter()
                .find(|(q, _)| *q == p)
                .map(|(_, t)| t.clone())
        };
        let params: Vec<(String, Ty)> = info
            .params
            .iter()
            .map(|(n, t)| (n.clone(), t.substitute(&subst)))
            .collect();
        let returns = info.returns.as_ref().map(|r| r.substitute(&subst));
        let fails: Vec<Ty> = info.fails.iter().map(|f| f.substitute(&subst)).collect();
        let needs = info.needs.clone();
        let is_library = info.is_library;
        let is_method = info.is_method;
        // the receiver fills `self`
        let mut explicit_params = params.clone();
        match receiver {
            Some((receiver_ty, receiver_span)) => {
                if !is_method {
                    self.error("not-a-method", format!("`{name}` is not a method"), span);
                } else {
                    let self_ty = params[0].1.clone();
                    if !self.assign(&receiver_ty, &self_ty, receiver_span) {
                        self.mismatch(&receiver_ty, &self_ty, receiver_span, "the receiver");
                    }
                    explicit_params.remove(0);
                }
            }
            None => {
                if is_method {
                    self.error_fix(
                        "method-call",
                        format!("`{name}` is a method"),
                        span,
                        format!("call it on a value: `value.{name}(...)`"),
                    );
                    if !explicit_params.is_empty() {
                        explicit_params.remove(0);
                    }
                }
            }
        }
        // the expected type guides a generic result (`let users: List of User be json.parse(text)`)
        if let (Some(expected), Some(returns)) = (expected, &returns) {
            let expected = self.resolve(expected);
            if !matches!(expected, Ty::Var(_)) {
                let _ = self.assign(returns, &expected, span);
            }
        }
        let function_args = self.check_args(&explicit_params, args, span, &name);
        // effects: the callee's needs plus those of function-valued arguments
        let mut needed: Vec<(Capability, bool, String)> = needs
            .into_iter()
            .map(|c| (c, is_library, name.clone()))
            .collect();
        for passed in function_args {
            let passed_info = &self.world.functions[passed];
            for capability in &passed_info.needs {
                needed.push((
                    capability.clone(),
                    passed_info.is_library,
                    passed_info.name.clone(),
                ));
            }
        }
        if !needed.is_empty() {
            self.effect_calls += 1;
        }
        for (capability, runtime_scoped, source) in needed {
            self.require_capability(&capability, runtime_scoped, &source, span);
        }
        // constraints of the instantiated parameters, checked when known
        for (param, ty) in &instances {
            for &ability in self.world.constraints(*param) {
                let what = format!("`{name}`");
                self.require_ability(ty, ability, span, &what);
            }
        }
        Info {
            ty: returns.unwrap_or(Ty::Unit),
            fails,
            function: None,
        }
    }

    /// A call through a function type (a parameter passed by name).
    fn call_function_type(
        &mut self,
        function: &FunctionTy,
        args: &[Arg],
        span: Span,
        name: &str,
    ) -> Info {
        if args.len() != function.params.len() {
            self.error(
                "argument-count",
                format!(
                    "`{name}` takes {} argument{}, found {}",
                    function.params.len(),
                    if function.params.len() == 1 { "" } else { "s" },
                    args.len()
                ),
                span,
            );
        }
        for (arg, param) in args.iter().zip(&function.params) {
            if let Some(label) = &arg.name {
                self.error_fix(
                    "argument-name",
                    "arguments of a function passed by name are positional",
                    label.span,
                    "drop the name",
                );
            }
            self.infer_value(&arg.value, param, "the argument");
        }
        if !function.needs.is_empty() {
            self.effect_calls += 1;
        }
        for capability in &function.needs {
            self.require_capability(capability, false, name, span);
        }
        Info {
            ty: function.returns.clone().unwrap_or(Ty::Unit),
            fails: function.fails.clone(),
            function: None,
        }
    }

    /// Check arguments against parameters: one argument is positional, two
    /// or more are named in declaration order (sketch section 3, M4). Returns
    /// the functions passed by name, for the effects of the call.
    fn check_args(
        &mut self,
        params: &[(String, Ty)],
        args: &[Arg],
        span: Span,
        name: &str,
    ) -> Vec<FunctionId> {
        let mut passed = Vec::new();
        if args.len() != params.len() {
            let wanted: Vec<String> = params.iter().map(|(n, _)| n.clone()).collect();
            let message = if params.is_empty() {
                format!("`{name}` takes no arguments")
            } else {
                format!(
                    "`{name}` takes {} argument{} ({}), found {}",
                    params.len(),
                    if params.len() == 1 { "" } else { "s" },
                    wanted.join(", "),
                    args.len()
                )
            };
            self.error("argument-count", message, span);
            for arg in args {
                self.infer(&arg.value, None);
            }
            return passed;
        }
        if params.len() == 1 {
            if let Some(label) = &args[0].name {
                self.error_fix(
                    "argument-name",
                    "a call with one argument does not name it",
                    label.span,
                    "drop the name",
                );
            }
        } else {
            for (arg, (param_name, _)) in args.iter().zip(params) {
                match &arg.name {
                    None => {
                        self.error_fix(
                            "argument-name",
                            "a call with two or more arguments names every argument",
                            arg.span,
                            format!("write `{param_name}: ...`"),
                        );
                    }
                    Some(label) if label.text != *param_name => {
                        let order: Vec<String> =
                            params.iter().map(|(n, _)| format!("{n}:")).collect();
                        if params.iter().any(|(n, _)| *n == label.text) {
                            self.error_fix(
                                "argument-order",
                                format!("`{}` is out of order", label.text),
                                label.span,
                                format!("the arguments of `{name}` are {}", order.join(" ")),
                            );
                        } else {
                            self.error_fix(
                                "argument-name",
                                format!("`{name}` has no parameter named `{}`", label.text),
                                label.span,
                                format!("the arguments are {}", order.join(" ")),
                            );
                        }
                    }
                    Some(_) => {}
                }
            }
        }
        for (arg, (_, param_ty)) in args.iter().zip(params) {
            let info = self.infer(&arg.value, Some(param_ty));
            self.require_handled(&info, arg.value.span);
            if let Some(function) = info.function {
                passed.push(function);
            }
            self.expect(&info.ty, param_ty, arg.value.span, "the argument");
        }
        passed
    }

    fn require_capability(
        &mut self,
        needed: &Capability,
        runtime_scoped: bool,
        source: &str,
        span: Span,
    ) {
        if covered(&self.context.needs, needed, runtime_scoped) {
            return;
        }
        let spelling = needed.spelling();
        let fix = if self.context.is_test {
            format!("add `needs {spelling}` after the test's name")
        } else {
            format!(
                "add `needs {spelling}` to the signature of `{}`",
                self.context.name
            )
        };
        self.error_fix(
            "capability-missing",
            format!(
                "`{source}` needs `{spelling}`, which `{}` does not declare",
                self.context.name
            ),
            span,
            fix,
        );
    }

    fn suggest_function(&self, name: &str) -> Option<String> {
        // a module function called without its namespace
        for (namespace, &target) in &self.world.modules[self.module].imports {
            if self.world.lookup_function(target, name).is_some() {
                return Some(format!("write `{namespace}.{name}(...)`"));
            }
        }
        let own: Vec<&String> = self.world.modules[self.module].functions.keys().collect();
        closest(name, own.into_iter().map(String::as_str)).map(|c| format!("did you mean `{c}`?"))
    }

    fn suggest_module_function(&self, module: ModuleId, name: &str) -> Option<String> {
        let names: Vec<&str> = self.world.modules[module]
            .functions
            .iter()
            .filter(|(_, ids)| ids.iter().any(|&id| !self.world.functions[id].is_method))
            .map(|(n, _)| n.as_str())
            .collect();
        closest(name, names.into_iter()).map(|c| format!("did you mean `{c}`?"))
    }

    fn suggest_method(&self, ty: &Ty, name: &str) -> Option<String> {
        let head = head_type(&self.base_of(ty))?;
        let names: Vec<&str> = self
            .world
            .method_index
            .keys()
            .filter(|(t, _)| *t == head)
            .map(|(_, n)| n.as_str())
            .collect();
        closest(name, names.into_iter()).map(|c| format!("did you mean `{c}`?"))
    }

    // ---------------------------------------------------------- constructions

    fn infer_construct(
        &mut self,
        name: &TypeName,
        args: &[Arg],
        span: Span,
        expected: Option<&Ty>,
    ) -> Info {
        let b = self.world.builtins.clone();
        // a variant with fields
        let variants = self.world.lookup_variant(self.module, &name.text);
        let type_id = if !variants.is_empty() {
            let chosen = if variants.len() == 1 {
                Some(variants[0])
            } else {
                match expected.map(|e| self.resolve(e)) {
                    Some(Ty::App(id, _)) => variants.iter().copied().find(|(t, _)| *t == id),
                    _ => {
                        // inside `fail with`, the declared failure types decide
                        let declared: Vec<(TypeId, usize)> = variants
                            .iter()
                            .copied()
                            .filter(|(t, _)| {
                                self.context
                                    .fails
                                    .iter()
                                    .any(|f| matches!(self.resolve(f), Ty::App(id, _) if id == *t))
                            })
                            .collect();
                        if declared.len() == 1 {
                            Some(declared[0])
                        } else {
                            None
                        }
                    }
                }
            };
            let Some((type_id, index)) = chosen else {
                self.error(
                    "ambiguous-variant",
                    format!(
                        "`{}` is a variant of several types; the context does not say which",
                        name.text
                    ),
                    name.span,
                );
                for arg in args {
                    self.infer(&arg.value, None);
                }
                return Info::plain(Ty::Error);
            };
            self.record(Target::Variant(type_id, index), name.span);
            let TypeKindInfo::Sum(variant_list) = &self.world.types[type_id].kind else {
                unreachable!()
            };
            let fields = variant_list[index].fields.clone();
            let args_tys: Vec<Ty> = self.world.types[type_id]
                .params
                .iter()
                .map(|_| self.fresh(VarKind::Any))
                .collect();
            let fails = self.construct_fields(&name.text, type_id, &args_tys, &fields, args, span);
            return Info {
                ty: Ty::App(type_id, args_tys),
                fails,
                function: None,
            };
        } else {
            self.world.lookup_type(self.module, &name.text)
        };
        let Some(type_id) = type_id else {
            let message = format!("unknown type `{}`", name.text);
            match self.world.suggest_import(self.module, &name.text) {
                Some(fix) => self.error_fix("unknown-type", message, name.span, fix),
                None => self.error("unknown-type", message, name.span),
            }
            for arg in args {
                self.infer(&arg.value, None);
            }
            return Info::plain(Ty::Error);
        };
        self.record(Target::Type(type_id), name.span);
        let params = self.world.types[type_id].params.clone();
        let args_tys: Vec<Ty> = params.iter().map(|_| self.fresh(VarKind::Any)).collect();
        if type_id == b.pair {
            let fields = vec![
                crate::world::FieldInfo {
                    name: "left".into(),
                    ty: Ty::Param(params[0]),
                    refinement: None,
                    span,
                },
                crate::world::FieldInfo {
                    name: "right".into(),
                    ty: Ty::Param(params[1]),
                    refinement: None,
                    span,
                },
            ];
            let fails = self.construct_fields("Pair", type_id, &args_tys, &fields, args, span);
            return Info {
                ty: Ty::App(type_id, args_tys),
                fails,
                function: None,
            };
        }
        match &self.world.types[type_id].kind {
            TypeKindInfo::Record(fields) => {
                let fields = fields.clone();
                let fails =
                    self.construct_fields(&name.text, type_id, &args_tys, &fields, args, span);
                Info {
                    ty: Ty::App(type_id, args_tys),
                    fails,
                    function: None,
                }
            }
            TypeKindInfo::Subtype { base, refinement } => {
                let base = base.clone();
                let refinement = refinement.clone();
                if args.len() != 1 {
                    self.error(
                        "argument-count",
                        format!("`{}` is built from one value", name.text),
                        span,
                    );
                    for arg in args {
                        self.infer(&arg.value, None);
                    }
                    return Info::plain(Ty::App(type_id, Vec::new()));
                }
                let arg = &args[0];
                if let Some(label) = &arg.name {
                    self.error_fix(
                        "argument-name",
                        "a construction from one value does not name it",
                        label.span,
                        "drop the name",
                    );
                }
                let info = self.infer(&arg.value, Some(&base));
                self.require_handled(&info, arg.value.span);
                self.expect(&info.ty, &base, arg.value.span, "the value");
                let mut fails = Vec::new();
                if let Some(condition) = &refinement {
                    match refine::literal_of(&arg.value) {
                        Some(literal) => match refine::evaluate(condition, "value", &literal) {
                            Verdict::Holds => {}
                            Verdict::Fails => {
                                self.error(
                                    "constraint-violation",
                                    format!(
                                        "this value does not satisfy the condition of `{}`",
                                        name.text
                                    ),
                                    arg.value.span,
                                );
                            }
                            Verdict::Unknown => fails.push(self.builtin(b.constraint_violation)),
                        },
                        None => fails.push(self.builtin(b.constraint_violation)),
                    }
                }
                Info {
                    ty: Ty::App(type_id, Vec::new()),
                    fails,
                    function: None,
                }
            }
            TypeKindInfo::Sum(_) => {
                self.error(
                    "construct-sum",
                    format!(
                        "`{}` is a sum type; construct one of its variants",
                        name.text
                    ),
                    name.span,
                );
                for arg in args {
                    self.infer(&arg.value, None);
                }
                Info::plain(Ty::Error)
            }
            TypeKindInfo::Opaque | TypeKindInfo::Unresolved => {
                self.error(
                    "construct-opaque",
                    format!("`{}` cannot be constructed from fields", name.text),
                    name.span,
                );
                for arg in args {
                    self.infer(&arg.value, None);
                }
                Info::plain(Ty::Error)
            }
        }
    }

    /// The fields of a record or variant construction; returns the errors the
    /// construction can raise (a refined field given a runtime value).
    fn construct_fields(
        &mut self,
        name: &str,
        type_id: TypeId,
        type_args: &[Ty],
        fields: &[crate::world::FieldInfo],
        args: &[Arg],
        span: Span,
    ) -> Vec<Ty> {
        let params = self.world.types[type_id].params.clone();
        let subst = |p: ParamId| {
            params
                .iter()
                .position(|q| *q == p)
                .map(|i| type_args[i].clone())
        };
        let mut fails = Vec::new();
        if args.len() != fields.len() {
            let wanted: Vec<String> = fields.iter().map(|f| format!("{}:", f.name)).collect();
            self.error_fix(
                "argument-count",
                format!(
                    "`{name}` has {} field{}, found {} argument{}",
                    fields.len(),
                    if fields.len() == 1 { "" } else { "s" },
                    args.len(),
                    if args.len() == 1 { "" } else { "s" }
                ),
                span,
                format!("the fields are {}", wanted.join(" ")),
            );
            for arg in args {
                self.infer(&arg.value, None);
            }
            return fails;
        }
        for (arg, field) in args.iter().zip(fields) {
            {
                match &arg.name {
                    None => self.error_fix(
                        "argument-name",
                        "a construction names every field",
                        arg.span,
                        format!("write `{}: ...`", field.name),
                    ),
                    Some(label) if label.text != field.name => {
                        let order: Vec<String> =
                            fields.iter().map(|f| format!("{}:", f.name)).collect();
                        if fields.iter().any(|f| f.name == label.text) {
                            self.error_fix(
                                "argument-order",
                                format!("`{}` is out of order", label.text),
                                label.span,
                                format!("the fields of `{name}` are {}", order.join(" ")),
                            );
                        } else {
                            self.error_fix(
                                "unknown-field",
                                format!("`{name}` has no field named `{}`", label.text),
                                label.span,
                                format!("the fields are {}", order.join(" ")),
                            );
                        }
                    }
                    Some(_) => {}
                }
            }
            let field_ty = field.ty.substitute(&subst);
            let info = self.infer(&arg.value, Some(&field_ty));
            self.require_handled(&info, arg.value.span);
            self.expect(&info.ty, &field_ty, arg.value.span, "the field value");
            if let Some(condition) = &field.refinement {
                let field_name = Name {
                    text: field.name.clone(),
                    span: field.span,
                };
                self.check_field_refinement(&field_name, condition, &arg.value, &mut fails);
            }
        }
        fails
    }

    /// A refined field: a literal is checked now, anything else at run time,
    /// which makes the construction fallible.
    fn check_field_refinement(
        &mut self,
        field: &Name,
        condition: &Expr,
        value: &Expr,
        fails: &mut Vec<Ty>,
    ) {
        let violation = self.builtin(self.world.builtins.constraint_violation);
        match refine::literal_of(value) {
            Some(literal) => match refine::evaluate(condition, &field.text, &literal) {
                Verdict::Holds => {}
                Verdict::Fails => {
                    self.error(
                        "constraint-violation",
                        format!(
                            "this value does not satisfy the condition on `{}`",
                            field.text
                        ),
                        value.span,
                    );
                }
                Verdict::Unknown => {
                    if !fails.contains(&violation) {
                        fails.push(violation);
                    }
                }
            },
            None => {
                if !fails.contains(&violation) {
                    fails.push(violation);
                }
            }
        }
    }

    // -------------------------------------------------------------- otherwise

    fn infer_otherwise(
        &mut self,
        value: &Expr,
        fallback: &Outcome,
        span: Span,
        expected: Option<&Ty>,
    ) -> Info {
        let info = self.infer(value, expected);
        let ty = self.resolve(&info.ty);
        let inner = match (&ty, info.fails.is_empty()) {
            (Ty::Maybe(inner), true) => (**inner).clone(),
            (Ty::Maybe(inner), false) => {
                // a fallible call that returns a maybe: `otherwise` handles the failure
                Ty::maybe((**inner).clone())
            }
            (_, false) => ty.clone(),
            (Ty::Error, true) => Ty::Error,
            (_, true) => {
                let shown = self.show(&ty);
                self.error_fix(
                    "superfluous-otherwise",
                    format!("`otherwise` after a value that cannot fail and is not optional (it is `{shown}`)"),
                    span,
                    "remove `otherwise` and what follows it",
                );
                ty.clone()
            }
        };
        match fallback {
            Outcome::Value(default) => {
                let default_info = self.infer(default, Some(&inner));
                self.require_handled(&default_info, default.span);
                self.expect(&default_info.ty, &inner, default.span, "the default");
            }
            Outcome::Fail(None, fail_span) => {
                if info.fails.is_empty() {
                    if matches!(ty, Ty::Maybe(_)) {
                        self.error_fix("otherwise-fail-maybe", "`otherwise fail` needs an error to pass on, but this value is optional, not fallible", *fail_span, "write `otherwise fail with Error(...)` or give a default");
                    }
                } else {
                    let union = Ty::Union(info.fails.clone());
                    self.check_error_declared(&union, *fail_span);
                }
            }
            Outcome::Fail(Some(error), fail_span) => {
                let error_info = self.infer(error, None);
                self.require_handled(&error_info, error.span);
                self.check_error_declared(&error_info.ty, *fail_span);
            }
            Outcome::Return(value, return_span) => self.check_return(value.as_ref(), *return_span),
            Outcome::Crash(message, _) => {
                let text = self.builtin(self.world.builtins.text);
                self.infer_value(message, &text, "the crash message");
            }
            Outcome::Break(break_span) | Outcome::Continue(break_span) => {
                if self.loop_depth == 0 {
                    self.error(
                        "outside-loop",
                        "`break` or `continue` outside a loop",
                        *break_span,
                    );
                }
            }
        }
        Info::plain(inner)
    }

    // ---------------------------------------------------------------- binary

    fn infer_binary(
        &mut self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        span: Span,
        expected: Option<&Ty>,
    ) -> Info {
        let b = self.world.builtins.clone();
        let boolean = self.builtin(b.boolean);
        match op {
            BinaryOp::And | BinaryOp::Or => {
                self.check_condition(left);
                self.check_condition(right);
                Info::plain(boolean)
            }
            BinaryOp::Is | BinaryOp::IsNot => {
                let left_info = self.infer(left, None);
                self.require_handled(&left_info, left.span);
                let right_info = self.infer(right, Some(&left_info.ty));
                self.require_handled(&right_info, right.span);
                // `maybe T` compares with `nothing` and with a `T`
                let left_ty = self.resolve(&left_info.ty);
                let ok = self.assign(&right_info.ty, &left_ty, right.span)
                    || self.assign(&left_info.ty, &right_info.ty, right.span);
                if !ok {
                    self.mismatch(
                        &right_info.ty,
                        &left_ty,
                        right.span,
                        "the right side of `is`",
                    );
                }
                self.require_ability(&left_info.ty, b.equal, span, "`is`");
                Info::plain(boolean)
            }
            BinaryOp::IsLessThan
            | BinaryOp::IsAtMost
            | BinaryOp::IsGreaterThan
            | BinaryOp::IsAtLeast => {
                let left_info = self.infer(left, None);
                self.require_handled(&left_info, left.span);
                let right_info = self.infer(right, Some(&left_info.ty));
                self.require_handled(&right_info, right.span);
                if !self.assign(&right_info.ty, &left_info.ty, right.span)
                    && !self.assign(&left_info.ty, &right_info.ty, left.span)
                {
                    self.mismatch(
                        &right_info.ty,
                        &left_info.ty,
                        right.span,
                        "the right side of the comparison",
                    );
                }
                self.require_ability(
                    &left_info.ty,
                    b.compare,
                    span,
                    format!("`{}`", op.spelling()).as_str(),
                );
                Info::plain(boolean)
            }
            BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Remainder
            | BinaryOp::Power => {
                let numeric_expected = expected.filter(|e| {
                    let resolved = self.resolve(e);
                    !matches!(resolved, Ty::Var(_)) && self.is_numeric(&resolved)
                });
                let left_info = self.infer(left, numeric_expected);
                self.require_handled(&left_info, left.span);
                let left_ty = self.resolve(&left_info.ty);
                if op == BinaryOp::Power {
                    // the exponent is an Integer for Integer and Decimal bases, a Float for Float
                    let base = self.base_of(&left_ty);
                    let exponent = if self.is_builtin(&base, b.float) {
                        self.builtin(b.float)
                    } else {
                        self.builtin(b.integer)
                    };
                    self.infer_value(right, &exponent, "the exponent");
                } else {
                    let right_info = self.infer(right, Some(&left_ty));
                    self.require_handled(&right_info, right.span);
                    if !self.unify(&self.base_of(&left_ty), &self.base_of(&right_info.ty), span) {
                        self.mismatch(&right_info.ty, &left_ty, right.span, "the right operand");
                    }
                }
                let result = self.base_of(&left_ty);
                let resolved = self.resolve(&result);
                let is_var = matches!(resolved, Ty::Var(_));
                if !is_var && !self.is_numeric(&resolved) && !resolved.is_error() {
                    let shown = self.show(&resolved);
                    let fix = if self.is_builtin(&resolved, b.text) {
                        "join texts with interpolation: \"{left}{right}\""
                    } else {
                        "arithmetic works on Integer, Decimal and Float"
                    };
                    self.error_fix(
                        "type-mismatch",
                        format!("`{}` needs numbers, but this is `{shown}`", op.spelling()),
                        left.span,
                        fix,
                    );
                    return Info::plain(Ty::Error);
                }
                if op == BinaryOp::Divide {
                    let integer_like = self.is_builtin(&resolved, b.integer)
                        || matches!(&resolved, Ty::Var(id) if matches!(self.vars[*id].kind, VarKind::IntegerLiteral(_)));
                    if integer_like {
                        self.error_fix(
                            "integer-division",
                            "`/` divides Decimal or Float values, not two Integers",
                            span,
                            "write `dividend.quotient(divisor)` for whole-number division, or `dividend.to_decimal() / divisor`",
                        );
                    }
                }
                Info::plain(result)
            }
        }
    }

    // --------------------------------------------------------------- patterns

    /// The subject of a `match`: a fallible call gives its success type and
    /// requires `success`/`failure` arms; anything else is matched directly.
    fn match_subject(&mut self, subject: &Expr) -> (Ty, Option<Vec<Ty>>) {
        let info = self.infer(subject, None);
        if info.fails.is_empty() {
            (info.ty, None)
        } else {
            (info.ty, Some(info.fails))
        }
    }

    fn check_pattern(&mut self, pattern: &Pattern, subject: &Ty, covers: &mut Vec<Cover>) {
        let subject_ty = self.resolve(subject);
        match pattern {
            Pattern::Binding(name) => {
                self.bind(name, subject_ty, false, BindingKind::Pattern);
                covers.push(Cover::Other);
            }
            Pattern::Nothing(span) => {
                match &subject_ty {
                    Ty::Maybe(_) | Ty::Error => {}
                    _ => {
                        let shown = self.show(&subject_ty);
                        self.error(
                            "pattern-mismatch",
                            format!(
                                "`nothing` matches a `maybe` value, but the subject is `{shown}`"
                            ),
                            *span,
                        );
                    }
                }
                covers.push(Cover::Nothing);
            }
            Pattern::Some(inner, span) => {
                match &subject_ty {
                    Ty::Maybe(item) => {
                        let item = (**item).clone();
                        self.check_pattern(inner, &item, &mut Vec::new());
                    }
                    Ty::Error => {
                        self.check_pattern(inner, &Ty::Error, &mut Vec::new());
                    }
                    _ => {
                        let shown = self.show(&subject_ty);
                        self.error(
                            "pattern-mismatch",
                            format!(
                                "`some(...)` matches a `maybe` value, but the subject is `{shown}`"
                            ),
                            *span,
                        );
                        self.check_pattern(inner, &Ty::Error, &mut Vec::new());
                    }
                }
                covers.push(Cover::Some);
            }
            Pattern::Success(inner, _) => {
                self.check_pattern(inner, &subject_ty, &mut Vec::new());
                covers.push(Cover::Success);
            }
            Pattern::Failure(inner, _) => {
                // the error union is attached to the match by `match_subject`; a
                // `failure(error: T)` arm removes T from the union the later arms see
                let errors = self.current_errors.clone().unwrap_or(Ty::Error);
                self.check_pattern(inner, &errors, &mut Vec::new());
                if let Pattern::Typed { ty, .. } = &**inner {
                    let handled = self.resolve_type(ty);
                    if let Some(Ty::Union(members)) = &self.current_errors {
                        let remaining: Vec<Ty> = members
                            .iter()
                            .filter(|m| !self.same_type(m, &handled))
                            .cloned()
                            .collect();
                        self.current_errors = Some(Ty::Union(remaining));
                    }
                }
                covers.push(Cover::Failure);
            }
            Pattern::Literal(literal) => {
                let info = self.infer(literal, Some(&subject_ty));
                self.expect(&info.ty, &subject_ty, literal.span, "the pattern");
                if let ExprKind::Boolean(value) = &literal.kind {
                    covers.push(if *value { Cover::True } else { Cover::False });
                } else {
                    covers.push(Cover::Other);
                }
            }
            Pattern::Typed { name, ty, span } => {
                let wanted = self.resolve_type(ty);
                match &subject_ty {
                    Ty::Union(members) => {
                        if !members.iter().any(|m| self.same_type(m, &wanted)) {
                            let shown = self.show(&wanted);
                            self.error(
                                "pattern-mismatch",
                                format!("the errors here are not `{shown}`"),
                                *span,
                            );
                        }
                    }
                    Ty::Error => {}
                    other => {
                        if !self.same_type(other, &wanted) {
                            let shown = self.show(&wanted);
                            let subject_shown = self.show(other);
                            self.error(
                                "pattern-mismatch",
                                format!("`{shown}` does not match a `{subject_shown}`"),
                                *span,
                            );
                        }
                    }
                }
                self.bind(name, wanted, false, BindingKind::Pattern);
                covers.push(Cover::Other);
            }
            Pattern::Variant { name, fields, span } => {
                // the type the variant belongs to: the subject's sum type, a member of
                // an error union, or a record matched like a single variant (K6)
                let target: Option<(Ty, Vec<crate::world::FieldInfo>, Vec<ParamId>)> =
                    match &subject_ty {
                        Ty::App(id, _) => self.variant_fields(*id, &name.text),
                        Ty::Union(members) => members.iter().find_map(|m| match self.resolve(m) {
                            Ty::App(id, _) => self.variant_fields(id, &name.text),
                            _ => None,
                        }),
                        Ty::Error => None,
                        _ => None,
                    };
                let subject_args: Vec<Ty> = match &subject_ty {
                    Ty::App(_, args) => args.clone(),
                    Ty::Union(members) => members
                        .iter()
                        .find_map(|m| match self.resolve(m) {
                            Ty::App(id, args) if self.variant_fields(id, &name.text).is_some() => {
                                Some(args)
                            }
                            _ => None,
                        })
                        .unwrap_or_default(),
                    _ => Vec::new(),
                };
                match target {
                    None => {
                        if !subject_ty.is_error() {
                            let shown = self.show(&subject_ty);
                            self.error(
                                "pattern-mismatch",
                                format!("`{shown}` has no variant named `{}`", name.text),
                                *span,
                            );
                        }
                        for field in fields {
                            if let Some(inner) = &field.pattern {
                                self.check_pattern(inner, &Ty::Error, &mut Vec::new());
                            } else {
                                self.bind(&field.field, Ty::Error, false, BindingKind::Pattern);
                            }
                        }
                    }
                    Some((_, variant_fields, params)) => {
                        let subst = |p: ParamId| {
                            params
                                .iter()
                                .position(|q| *q == p)
                                .and_then(|i| subject_args.get(i).cloned())
                        };
                        for field in fields {
                            match variant_fields.iter().find(|f| f.name == field.field.text) {
                                Some(info) => {
                                    let field_ty = info.ty.substitute(&subst);
                                    match &field.pattern {
                                        Some(inner) => {
                                            self.check_pattern(inner, &field_ty, &mut Vec::new())
                                        }
                                        None => self.bind(
                                            &field.field,
                                            field_ty,
                                            false,
                                            BindingKind::Pattern,
                                        ),
                                    }
                                }
                                None => {
                                    self.error(
                                        "unknown-field",
                                        format!(
                                            "`{}` has no field named `{}`",
                                            name.text, field.field.text
                                        ),
                                        field.field.span,
                                    );
                                    if let Some(inner) = &field.pattern {
                                        self.check_pattern(inner, &Ty::Error, &mut Vec::new());
                                    } else {
                                        self.bind(
                                            &field.field,
                                            Ty::Error,
                                            false,
                                            BindingKind::Pattern,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                covers.push(Cover::Variant(name.text.clone()));
            }
        }
    }

    /// The fields of a variant of a sum type, or of a record matched by its
    /// own name; with the type's parameters for substitution.
    fn variant_fields(
        &self,
        type_id: TypeId,
        name: &str,
    ) -> Option<(Ty, Vec<crate::world::FieldInfo>, Vec<ParamId>)> {
        let info = &self.world.types[type_id];
        match &info.kind {
            TypeKindInfo::Sum(variants) => variants.iter().find(|v| v.name == name).map(|v| {
                (
                    Ty::App(type_id, Vec::new()),
                    v.fields.clone(),
                    info.params.clone(),
                )
            }),
            TypeKindInfo::Record(fields) if info.name == name => Some((
                Ty::App(type_id, Vec::new()),
                fields.clone(),
                info.params.clone(),
            )),
            _ => None,
        }
    }

    /// Whether the arms cover the subject; reports what is missing.
    fn check_exhaustive(
        &mut self,
        subject: &Ty,
        covers: &[Cover],
        errors: Option<Vec<Ty>>,
        span: Span,
    ) -> bool {
        let b = self.world.builtins.clone();
        let _ = &b;
        let covered: HashSet<&Cover> = covers.iter().collect();
        if covered.contains(&Cover::Other) {
            return true;
        }
        if let Some(_errors) = errors {
            let mut missing = Vec::new();
            if !covered.contains(&Cover::Success) {
                missing.push("success(...)".to_string());
            }
            if !covered.contains(&Cover::Failure) {
                missing.push("failure(...)".to_string());
            }
            if !missing.is_empty() {
                self.error_fix(
                    "not-exhaustive",
                    format!("the match does not cover {}", missing.join(" and ")),
                    span,
                    "add the missing arm",
                );
                return false;
            }
            return true;
        }
        let ty = self.resolve(subject);
        match &ty {
            Ty::Maybe(_) => {
                let mut missing = Vec::new();
                if !covered.contains(&Cover::Some) {
                    missing.push("some(...)");
                }
                if !covered.contains(&Cover::Nothing) {
                    missing.push("nothing");
                }
                if !missing.is_empty() {
                    self.error_fix(
                        "not-exhaustive",
                        format!("the match does not cover {}", missing.join(" and ")),
                        span,
                        "add the missing arm or an `otherwise`",
                    );
                    return false;
                }
                true
            }
            Ty::App(id, _) if *id == b.boolean => {
                let both = covered.contains(&Cover::True) && covered.contains(&Cover::False);
                if !both {
                    self.error_fix(
                        "not-exhaustive",
                        "the match does not cover both `true` and `false`",
                        span,
                        "add the missing arm or an `otherwise`",
                    );
                }
                both
            }
            Ty::App(id, _) => match &self.world.types[*id].kind {
                TypeKindInfo::Sum(variants) => {
                    let missing: Vec<String> = variants
                        .iter()
                        .filter(|v| !covered.contains(&Cover::Variant(v.name.clone())))
                        .map(|v| format!("`{}`", v.name))
                        .collect();
                    if !missing.is_empty() {
                        self.error_fix(
                            "not-exhaustive",
                            format!("the match does not cover {}", missing.join(", ")),
                            span,
                            "add an arm for each, or an `otherwise`",
                        );
                        return false;
                    }
                    true
                }
                TypeKindInfo::Record(_) => covered.iter().any(|c| matches!(c, Cover::Variant(_))),
                _ => {
                    self.error_fix(
                        "not-exhaustive",
                        "a match on this value needs an `otherwise` arm",
                        span,
                        "add `otherwise ...`",
                    );
                    false
                }
            },
            Ty::Error => true,
            _ => {
                self.error_fix(
                    "not-exhaustive",
                    "a match on this value needs an `otherwise` arm",
                    span,
                    "add `otherwise ...`",
                );
                false
            }
        }
    }

    // ---------------------------------------------------------------- queries

    fn infer_query(&mut self, query: &Query, expected: Option<&Ty>) -> Info {
        let b = self.world.builtins.clone();
        self.push_scope();
        let mut last_item = Ty::Error;
        for source in &query.sources {
            let items = self.loop_items(&source.bindings, &source.source);
            last_item = if items.len() == 1 {
                items[0].clone()
            } else {
                Ty::App(b.pair, items.clone())
            };
            for (name, ty) in source.bindings.iter().zip(items) {
                self.bind(name, ty, false, BindingKind::Query);
            }
        }
        if let Some(within) = &query.within {
            self.check_within(within);
        }
        if let Some(filter) = &query.filter {
            self.check_condition(filter);
        }
        if let Some(order) = &query.order {
            self.check_ordering(order);
        }
        let key_ty = query.group_by.as_ref().map(|key| {
            let info = self.infer(key, None);
            self.require_handled(&info, key.span);
            self.require_ability(&info.ty, b.hash, key.span, "`group by`");
            info.ty
        });
        let expected_inner = match (expected.map(|e| self.resolve(e)), key_ty.is_some()) {
            (Some(Ty::App(id, args)), true) if id == b.map => Some(args[1].clone()),
            (Some(other), false) => Some(other),
            _ => None,
        };
        let result = match &query.terminal {
            QueryTerminal::Collect(value) => {
                let item_expected = match expected_inner.as_ref().map(|e| self.resolve(e)) {
                    Some(Ty::App(id, args)) if id == b.list => Some(args[0].clone()),
                    _ => None,
                };
                let info = self.infer(value, item_expected.as_ref());
                self.require_handled(&info, value.span);
                Ty::App(b.list, vec![info.ty])
            }
            QueryTerminal::Sum(value) => {
                let info = self.infer(value, None);
                self.require_handled(&info, value.span);
                let resolved = self.resolve(&info.ty);
                if !matches!(resolved, Ty::Var(_) | Ty::Error) && !self.is_numeric(&resolved) {
                    let shown = self.show(&resolved);
                    self.error(
                        "type-mismatch",
                        format!("`sum` adds numbers, but these are `{shown}`"),
                        value.span,
                    );
                }
                self.base_of(&info.ty)
            }
            QueryTerminal::Count => {
                // the loop variable is not read by `count` (decision M2)
                self.builtin(b.integer)
            }
            QueryTerminal::First => {
                // `first` returns the item, so the loop variables count as read
                if let Some(scope) = self.scopes.last_mut() {
                    for binding in &mut scope.bindings {
                        binding.used = true;
                    }
                }
                Ty::maybe(last_item)
            }
            QueryTerminal::Any(value) | QueryTerminal::All(value) => {
                self.check_condition(value);
                self.builtin(b.boolean)
            }
            QueryTerminal::None => Ty::App(b.list, vec![last_item]),
        };
        self.pop_scope();
        let ty = match key_ty {
            Some(key) => Ty::App(b.map, vec![key, result]),
            None => result,
        };
        if query.concurrently {
            // nothing to type: the body runs as parallel tasks
        }
        Info::plain(ty)
    }

    fn resolve_type(&mut self, ty: &Type) -> Ty {
        // body-level type annotations have no type parameters in scope; a
        // generic function's parameters are not visible here yet
        let mut world_diagnostics = Vec::new();
        let resolved = resolve_in_body(self.world, self.module, ty, &mut world_diagnostics);
        self.diagnostics.extend(world_diagnostics);
        self.record_type_names(ty);
        resolved
    }
}

enum Method {
    Declared(FunctionId),
    Ability(AbilityId, usize),
}

/// The callee of a call, looking through `otherwise`.
fn callee_of(expr: &Expr) -> Option<&Expr> {
    match &expr.kind {
        ExprKind::Call { callee, .. } => Some(callee),
        ExprKind::Otherwise { value, .. } => callee_of(value),
        _ => None,
    }
}

/// Resolve a type written inside a body without mutating the world.
fn resolve_in_body(
    world: &World,
    module: ModuleId,
    ty: &Type,
    diagnostics: &mut Vec<Diagnostic>,
) -> Ty {
    match ty {
        Type::Maybe(inner, _) => Ty::maybe(resolve_in_body(world, module, inner, diagnostics)),
        Type::Function {
            params,
            returns,
            fails,
            needs,
            ..
        } => Ty::Function(Box::new(FunctionTy {
            params: params
                .iter()
                .map(|p| resolve_in_body(world, module, p, diagnostics))
                .collect(),
            returns: returns
                .as_ref()
                .map(|r| resolve_in_body(world, module, r, diagnostics)),
            fails: fails
                .iter()
                .map(|f| resolve_in_body(world, module, f, diagnostics))
                .collect(),
            needs: needs.iter().map(Capability::from_ast).collect(),
        })),
        Type::Named { name, args, span } => {
            let Some(type_id) = world.lookup_type(module, &name.text) else {
                let message = format!("unknown type `{}`", name.text);
                let diagnostic = match world.suggest_import(module, &name.text) {
                    Some(fix) => {
                        Diagnostic::error("unknown-type", message, name.span).with_fix(fix)
                    }
                    None => Diagnostic::error("unknown-type", message, name.span),
                };
                diagnostics.push(diagnostic);
                return Ty::Error;
            };
            let expected = world.types[type_id].params.len();
            if expected != args.len() {
                diagnostics.push(Diagnostic::error(
                    "type-arity",
                    format!(
                        "`{}` takes {expected} type argument{}, found {}",
                        name.text,
                        if expected == 1 { "" } else { "s" },
                        args.len()
                    ),
                    *span,
                ));
                return Ty::Error;
            }
            Ty::App(
                type_id,
                args.iter()
                    .map(|a| resolve_in_body(world, module, a, diagnostics))
                    .collect(),
            )
        }
    }
}

/// The closest name by edit distance, when it is close enough to be a slip.
fn closest<'a>(name: &str, candidates: impl Iterator<Item = &'a str>) -> Option<String> {
    let mut best: Option<(usize, &str)> = None;
    for candidate in candidates {
        let distance = edit_distance(name, candidate);
        if distance <= 2 && best.is_none_or(|(d, _)| distance < d) {
            best = Some((distance, candidate));
        }
    }
    best.map(|(_, c)| c.to_string())
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut current = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            current.push(
                (previous[j] + cost)
                    .min(previous[j + 1] + 1)
                    .min(current[j] + 1),
            );
        }
        previous = current;
    }
    previous[b.len()]
}

/// The per-module driver: check every body of a user module.
pub fn check_module(world: &World, module: ModuleId) -> Vec<Diagnostic> {
    check_module_with_references(world, module).0
}

/// Check a module and also hand back every reference its bodies make, by
/// body (the project map is built from them).
pub fn check_module_with_references(
    world: &World,
    module: ModuleId,
) -> (Vec<Diagnostic>, Vec<(BodyLocation, Reference)>) {
    let mut checker = Checker::new(world, module);
    let info = &world.modules[module];
    for (index, item) in info.ast.items.iter().enumerate() {
        match item {
            Item::Function(function) => {
                if let Some(id) = find_function(world, module, BodyLocation::Item(index)) {
                    checker.check_function(id, function);
                }
            }
            Item::Implementation(implementation) => {
                for (function_index, function) in implementation.functions.iter().enumerate() {
                    if let Some(id) = find_function(
                        world,
                        module,
                        BodyLocation::Implementation(index, function_index),
                    ) {
                        checker.check_function(id, function);
                    }
                }
            }
            Item::Test(test) => checker.check_test(index, test),
            Item::Constant(constant) => {
                if let Some(constant_info) = info.constants.get(&constant.name.text) {
                    let ty = constant_info.ty.clone();
                    checker.check_constant(index, constant, &ty);
                }
            }
            _ => {}
        }
    }
    if info.ast.docs.purpose.is_none() {
        checker.diagnostics.push(
            Diagnostic::error(
                "purpose-missing",
                "the module has no purpose",
                Span::new(0, info.ast.name.last().map_or(0, |n| n.span.end)),
            )
            .with_fix("add a `purpose:` clause under the `module` line"),
        );
    }
    (checker.diagnostics, checker.references)
}

fn find_function(world: &World, module: ModuleId, body: BodyLocation) -> Option<FunctionId> {
    world.functions.iter().position(|f| {
        f.module == module
            && match (f.body, body) {
                (BodyLocation::Item(a), BodyLocation::Item(b)) => a == b,
                (BodyLocation::Implementation(a, c), BodyLocation::Implementation(b, d)) => {
                    a == b && c == d
                }
                _ => false,
            }
    })
}
