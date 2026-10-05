//! The checker's type representation: nominal types applied to arguments,
//! `maybe`, function types, rigid type parameters, inference variables and
//! the error unions of fallible calls.

use std::fmt;

use crate::effects::Capability;

pub type TypeId = usize;
pub type FunctionId = usize;
pub type AbilityId = usize;
pub type ModuleId = usize;
pub type ParamId = usize;
pub type VarId = usize;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Ty {
    /// A named type with its arguments: `Integer`, `List of Text`, `User`.
    App(TypeId, Vec<Ty>),
    /// `maybe T`.
    Maybe(Box<Ty>),
    Function(Box<FunctionTy>),
    /// A `for any` parameter inside the definition that introduces it.
    Param(ParamId),
    /// An inference variable of the body being checked.
    Var(VarId),
    /// `Self` inside an ability declaration.
    SelfType,
    /// The error value of a fallible call: one of these error types.
    Union(Vec<Ty>),
    /// No value: the result of a call to a function without `returns`.
    Unit,
    /// A way out (`fail`, `return`, `crash`, `break`, `continue`): fits anywhere.
    Never,
    /// Unknown after a reported error; fits anywhere and reports nothing more.
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FunctionTy {
    pub params: Vec<Ty>,
    pub returns: Option<Ty>,
    pub fails: Vec<Ty>,
    pub needs: Vec<Capability>,
}

impl Ty {
    pub fn maybe(inner: Ty) -> Ty {
        Ty::Maybe(Box::new(inner))
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Ty::Error)
    }

    /// Apply a substitution of type parameters.
    pub fn substitute(&self, map: &dyn Fn(ParamId) -> Option<Ty>) -> Ty {
        match self {
            Ty::App(id, args) => Ty::App(*id, args.iter().map(|a| a.substitute(map)).collect()),
            Ty::Maybe(inner) => Ty::maybe(inner.substitute(map)),
            Ty::Function(function) => Ty::Function(Box::new(FunctionTy {
                params: function.params.iter().map(|p| p.substitute(map)).collect(),
                returns: function.returns.as_ref().map(|r| r.substitute(map)),
                fails: function.fails.iter().map(|f| f.substitute(map)).collect(),
                needs: function.needs.clone(),
            })),
            Ty::Param(id) => map(*id).unwrap_or(Ty::Param(*id)),
            Ty::Union(members) => Ty::Union(members.iter().map(|m| m.substitute(map)).collect()),
            other => other.clone(),
        }
    }

    /// Replace `Self` with a type.
    pub fn with_self(&self, self_type: &Ty) -> Ty {
        match self {
            Ty::SelfType => self_type.clone(),
            Ty::App(id, args) => {
                Ty::App(*id, args.iter().map(|a| a.with_self(self_type)).collect())
            }
            Ty::Maybe(inner) => Ty::maybe(inner.with_self(self_type)),
            Ty::Function(function) => Ty::Function(Box::new(FunctionTy {
                params: function
                    .params
                    .iter()
                    .map(|p| p.with_self(self_type))
                    .collect(),
                returns: function.returns.as_ref().map(|r| r.with_self(self_type)),
                fails: function
                    .fails
                    .iter()
                    .map(|f| f.with_self(self_type))
                    .collect(),
                needs: function.needs.clone(),
            })),
            Ty::Union(members) => {
                Ty::Union(members.iter().map(|m| m.with_self(self_type)).collect())
            }
            other => other.clone(),
        }
    }

    /// Whether an inference variable occurs in the type.
    pub fn mentions_var(&self, var: VarId) -> bool {
        match self {
            Ty::Var(id) => *id == var,
            Ty::App(_, args) | Ty::Union(args) => args.iter().any(|a| a.mentions_var(var)),
            Ty::Maybe(inner) => inner.mentions_var(var),
            Ty::Function(function) => {
                function.params.iter().any(|p| p.mentions_var(var))
                    || function
                        .returns
                        .as_ref()
                        .is_some_and(|r| r.mentions_var(var))
                    || function.fails.iter().any(|f| f.mentions_var(var))
            }
            _ => false,
        }
    }

    pub fn has_vars(&self) -> bool {
        match self {
            Ty::Var(_) => true,
            Ty::App(_, args) | Ty::Union(args) => args.iter().any(Ty::has_vars),
            Ty::Maybe(inner) => inner.has_vars(),
            Ty::Function(function) => {
                function.params.iter().any(Ty::has_vars)
                    || function.returns.as_ref().is_some_and(Ty::has_vars)
                    || function.fails.iter().any(Ty::has_vars)
            }
            _ => false,
        }
    }
}

/// How a type is spelled in diagnostics: the caller supplies type and
/// parameter names, since the checker's tables are not in scope here.
pub struct TyDisplay<'a> {
    pub ty: &'a Ty,
    pub type_name: &'a dyn Fn(TypeId) -> String,
    pub param_name: &'a dyn Fn(ParamId) -> String,
}

impl fmt::Display for TyDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let show = |ty: &Ty| {
            TyDisplay {
                ty,
                type_name: self.type_name,
                param_name: self.param_name,
            }
            .to_string()
        };
        match self.ty {
            Ty::App(id, args) => {
                let name = (self.type_name)(*id);
                if args.is_empty() {
                    write!(f, "{name}")
                } else if name == "Map" && args.len() == 2 {
                    write!(f, "Map of {} to {}", show(&args[0]), show(&args[1]))
                } else {
                    let shown: Vec<String> = args.iter().map(show).collect();
                    write!(f, "{name} of {}", shown.join(", "))
                }
            }
            Ty::Maybe(inner) => write!(f, "maybe {}", show(inner)),
            Ty::Function(function) => {
                let params: Vec<String> = function.params.iter().map(show).collect();
                write!(f, "function({})", params.join(", "))?;
                if let Some(returns) = &function.returns {
                    write!(f, " returns {}", show(returns))?;
                }
                if !function.fails.is_empty() {
                    let fails: Vec<String> = function.fails.iter().map(show).collect();
                    write!(f, " or fails with {}", fails.join(" or "))?;
                }
                Ok(())
            }
            Ty::Param(id) => write!(f, "{}", (self.param_name)(*id)),
            Ty::Var(_) => write!(f, "_"),
            Ty::SelfType => write!(f, "Self"),
            Ty::Union(members) => {
                let shown: Vec<String> = members.iter().map(show).collect();
                write!(f, "{}", shown.join(" or "))
            }
            Ty::Unit => write!(f, "nothing"),
            Ty::Never => write!(f, "a way out"),
            Ty::Error => write!(f, "?"),
        }
    }
}
