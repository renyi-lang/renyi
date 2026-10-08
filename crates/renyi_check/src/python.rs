//! The Python bridge (decisions AJ2, AJ3 and AL1 to AL4): the rules a
//! declaration file bound to a Python module follows. Each of its functions
//! needs `python("<package>")` and nothing else, fails with `PythonError`
//! and nothing else and takes no type parameters; its parameters are types
//! that can `ToJson` and its result a type that can `FromJson`, or nothing,
//! since a call crosses as JSON both ways. The VM's `natives/python.rs`
//! runs what passes here.

use renyi_syntax::ast;

use crate::effects::Capability;
use crate::types::{AbilityId, ModuleId, Ty};
use crate::world::{head_type, TypeKindInfo, World};

/// The module of the bridge's failure type.
pub const MODULE: &str = "std.python";

/// The bridge's failure type (decision AL4).
pub const ERROR_TYPE: &str = "PythonError";

/// The types a parameter may have, for the diagnostic.
pub const PARAMETER_TYPES: &str = "a type `std.json` renders, one that can ToJson";

/// The types a result may have, for the diagnostic.
pub const RESULT_TYPES: &str = "a type `std.json` parses, one that can FromJson, or nothing";

impl World {
    /// The prelude's `ToJson` or `FromJson`.
    pub fn json_ability(&self, name: &str) -> Option<AbilityId> {
        let prelude = self.prelude?;
        self.modules[prelude].abilities.get(name).copied()
    }

    /// Whether a declared type crosses the bridge, as a parameter
    /// (`ToJson`) or as a result (`FromJson`): the structural rule of the
    /// body checker's `has_ability` on a type without inference variables.
    /// A derived or implemented ability, a subtype's base, a list or a set
    /// of carried items, a map with `Text` keys and carried values, `maybe`
    /// a carried type, and the scalars of the prelude.
    pub fn carried(&self, ty: &Ty, ability: AbilityId) -> bool {
        let b = &self.builtins;
        match ty {
            Ty::Maybe(inner) => self.carried(inner, ability),
            Ty::App(id, args) => {
                let info = &self.types[*id];
                if info.derives.contains(&ability)
                    || self
                        .impls
                        .iter()
                        .any(|i| i.ability == ability && head_type(&i.target) == Some(*id))
                {
                    return true;
                }
                if let TypeKindInfo::Subtype { base, .. } = &info.kind {
                    return self.carried(base, ability);
                }
                if *id == b.list || *id == b.set {
                    return args.first().is_some_and(|item| self.carried(item, ability));
                }
                if *id == b.map {
                    return args.len() == 2
                        && matches!(&args[0], Ty::App(key, _) if *key == b.text)
                        && self.carried(&args[1], ability);
                }
                *id == b.integer
                    || *id == b.decimal
                    || *id == b.float
                    || *id == b.text
                    || *id == b.boolean
                    || *id == b.bytes
            }
            _ => false,
        }
    }

    /// Whether a type is `PythonError` of `std.python`.
    fn is_python_error(&self, ty: &Ty) -> bool {
        let Ty::App(id, args) = ty else {
            return false;
        };
        let info = &self.types[*id];
        args.is_empty() && info.name == ERROR_TYPE && self.modules[info.module].name == MODULE
    }

    /// Decision AL1: a function of a Python module is called through the
    /// bridge. It needs `python("<package>")` and nothing else, fails with
    /// `PythonError` and nothing else, takes no type parameters, and its
    /// parameters and result are types JSON carries.
    pub(crate) fn check_python_signature(
        &mut self,
        module: ModuleId,
        function: &ast::Function,
        params: &[(String, Ty)],
        returns: Option<&Ty>,
        fails: &[Ty],
        needs: &[Capability],
    ) {
        let name = function.name.text.clone();
        let package = self.modules[module]
            .python
            .as_ref()
            .map(|binding| binding.module.package.clone())
            .unwrap_or_default();
        let only_python = needs.len() == 1
            && needs[0].path == ["python"]
            && needs[0].scope.as_deref() == Some(package.as_str())
            && !needs[0].has_grant_clauses();
        if !only_python {
            self.error_with_fix(
                module,
                "python-signature",
                format!(
                    "`{name}` is declared in a Python module; it needs `python(\"{package}\")` and nothing else"
                ),
                function.name.span,
                format!("write `needs python(\"{package}\")`"),
            );
        }
        // an unknown failure type is reported as such already
        let python_error =
            fails.len() == 1 && (self.is_python_error(&fails[0]) || fails[0] == Ty::Error);
        if !python_error {
            self.error_with_fix(
                module,
                "python-signature",
                format!(
                    "`{name}` is declared in a Python module; it fails with `PythonError` and nothing else"
                ),
                function.name.span,
                "write `or fails with PythonError`, imported from `std.python`".into(),
            );
        }
        if function.type_params.is_some() {
            self.error_with_fix(
                module,
                "python-signature",
                format!(
                    "`{name}` is declared in a Python module; a Python function takes no type parameters"
                ),
                function.name.span,
                "drop `for any`".into(),
            );
        }
        let to_json = self.json_ability("ToJson");
        let from_json = self.json_ability("FromJson");
        for (index, (param_name, ty)) in params.iter().enumerate() {
            if to_json.is_some_and(|ability| self.carried(ty, ability)) {
                continue;
            }
            let span = function
                .params
                .get(index)
                .map(|param| param.name.span)
                .unwrap_or(function.name.span);
            self.error_with_fix(
                module,
                "python-type",
                format!(
                    "the parameter `{param_name}` of `{name}` has type `{}`; across the bridge a parameter is {PARAMETER_TYPES}",
                    self.show(ty)
                ),
                span,
                "declare it with a type that can ToJson".into(),
            );
        }
        if let Some(ty) = returns {
            if !from_json.is_some_and(|ability| self.carried(ty, ability)) {
                let span = function
                    .returns
                    .as_ref()
                    .map(|ty| ty.span())
                    .unwrap_or(function.name.span);
                self.error_with_fix(
                    module,
                    "python-type",
                    format!(
                        "`{name}` returns `{}`; across the bridge a result is {RESULT_TYPES}",
                        self.show(ty)
                    ),
                    span,
                    "declare it with a type that can FromJson, or drop `returns`".into(),
                );
            }
        }
    }
}
