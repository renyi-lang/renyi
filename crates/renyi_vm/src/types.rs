//! What the VM needs to know about every declared type at run time: names
//! for rendering, field order, types and external names for construction and
//! JSON, the fields `can Compare by` lists, and the refinements to check at
//! construction (as code objects, compiled by the compiler).

use std::collections::HashMap;

use renyi_check::types::{ParamId, Ty};
use renyi_check::world::{TypeKindInfo, World};
use renyi_check::{AbilityId, FunctionId, TypeId};
use renyi_syntax::ast::{Item, TypeKind};

#[derive(Clone, Debug)]
pub struct FieldMeta {
    pub name: String,
    pub ty: Ty,
    pub external_name: Option<String>,
    /// Whether the field's type is `maybe`, for JSON decoding of a missing key.
    pub optional: bool,
    /// The index of the compiled refinement in `TypeMeta::refinements`.
    pub refinement: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct VariantMeta {
    pub name: String,
    pub fields: Vec<FieldMeta>,
}

#[derive(Clone, Debug)]
pub enum TypeShape {
    Opaque,
    Record(Vec<FieldMeta>),
    Sum(Vec<VariantMeta>),
    /// `type Email is Text where ...`: the base type; a refinement, when
    /// there is one, is index 0 of `refinements`.
    Subtype {
        base: Ty,
    },
}

#[derive(Clone, Debug)]
pub struct TypeMeta {
    pub id: TypeId,
    pub name: String,
    pub module: String,
    pub is_library: bool,
    pub params: Vec<ParamId>,
    pub shape: TypeShape,
    /// The abilities derived with `can`.
    pub derives: Vec<AbilityId>,
    /// `can Compare by a, b`: field indices; `None` means every field.
    pub compare_by: Option<Vec<usize>>,
    /// Compiled refinement predicates, filled by the compiler: a subtype has
    /// one for `value`; a record or variant one per refined field, each over
    /// all the fields.
    pub refinements: Vec<usize>,
}

#[derive(Clone)]
pub struct Types {
    pub metas: Vec<TypeMeta>,
    /// An explicit implementation of an ability for a type: the ability's
    /// method name to the function.
    pub impls: HashMap<(AbilityId, TypeId), HashMap<String, FunctionId>>,
}

impl Types {
    pub fn from_world(world: &World) -> Types {
        let mut metas = Vec::with_capacity(world.types.len());
        for (id, info) in world.types.iter().enumerate() {
            let module = &world.modules[info.module];
            let ast = module.ast.items.iter().find_map(|item| match item {
                Item::Type(def) if def.name.text == info.name => Some(def),
                _ => None,
            });
            let field_meta =
                |field: &renyi_check::world::FieldInfo, external: Option<String>| FieldMeta {
                    name: field.name.clone(),
                    ty: field.ty.clone(),
                    external_name: external,
                    optional: matches!(field.ty, Ty::Maybe(_)),
                    refinement: None,
                };
            let shape = match &info.kind {
                TypeKindInfo::Opaque | TypeKindInfo::Unresolved => TypeShape::Opaque,
                TypeKindInfo::Record(fields) => TypeShape::Record(
                    fields
                        .iter()
                        .map(|field| field_meta(field, external_name(ast, &field.name)))
                        .collect(),
                ),
                TypeKindInfo::Sum(variants) => TypeShape::Sum(
                    variants
                        .iter()
                        .map(|variant| VariantMeta {
                            name: variant.name.clone(),
                            fields: variant
                                .fields
                                .iter()
                                .map(|field| field_meta(field, None))
                                .collect(),
                        })
                        .collect(),
                ),
                TypeKindInfo::Subtype { base, .. } => TypeShape::Subtype { base: base.clone() },
            };
            let compare_by = ast.and_then(|def| {
                let derives = match &def.kind {
                    TypeKind::Record { derives, .. } | TypeKind::Sum { derives, .. } => derives,
                    TypeKind::Subtype { .. } => return None,
                };
                let derive = derives.iter().find(|d| d.ability.text == "Compare")?;
                if derive.by.is_empty() {
                    return None;
                }
                let names: Vec<String> = match &info.kind {
                    TypeKindInfo::Record(fields) => fields.iter().map(|f| f.name.clone()).collect(),
                    _ => return None,
                };
                Some(
                    derive
                        .by
                        .iter()
                        .filter_map(|name| names.iter().position(|n| *n == name.text))
                        .collect(),
                )
            });
            metas.push(TypeMeta {
                id,
                name: info.name.clone(),
                module: module.name.clone(),
                is_library: module.is_library,
                params: info.params.clone(),
                shape,
                derives: info.derives.clone(),
                compare_by,
                refinements: Vec::new(),
            });
        }
        let mut impls: HashMap<(AbilityId, TypeId), HashMap<String, FunctionId>> = HashMap::new();
        for implementation in &world.impls {
            let Some(target) = renyi_check::world::head_type(&implementation.target) else {
                continue;
            };
            let methods = impls.entry((implementation.ability, target)).or_default();
            for &function in &implementation.functions {
                methods.insert(world.functions[function].name.clone(), function);
            }
        }
        Types { metas, impls }
    }

    pub fn meta(&self, id: TypeId) -> &TypeMeta {
        &self.metas[id]
    }

    /// A type by module and name, for the library's error values.
    pub fn find(&self, module: &str, name: &str) -> Option<TypeId> {
        self.metas
            .iter()
            .position(|meta| meta.module == module && meta.name == name)
    }

    pub fn variant_tag(&self, ty: TypeId, name: &str) -> Option<usize> {
        match &self.metas[ty].shape {
            TypeShape::Sum(variants) => variants.iter().position(|v| v.name == name),
            _ => None,
        }
    }

    /// The function implementing an ability method for a type, when an
    /// explicit implementation exists.
    pub fn implementation(
        &self,
        ability: AbilityId,
        ty: TypeId,
        method: &str,
    ) -> Option<FunctionId> {
        self.impls.get(&(ability, ty))?.get(method).copied()
    }

    pub fn field_index(&self, ty: TypeId, name: &str) -> Option<usize> {
        match &self.metas[ty].shape {
            TypeShape::Record(fields) => fields.iter().position(|f| f.name == name),
            _ => None,
        }
    }

    pub fn variant_field_index(&self, ty: TypeId, tag: usize, name: &str) -> Option<usize> {
        match &self.metas[ty].shape {
            TypeShape::Sum(variants) => variants
                .get(tag)?
                .fields
                .iter()
                .position(|f| f.name == name),
            _ => None,
        }
    }
}

fn external_name(ast: Option<&renyi_syntax::ast::TypeDef>, field: &str) -> Option<String> {
    let def = ast?;
    match &def.kind {
        TypeKind::Record { fields, .. } => fields
            .iter()
            .find(|f| f.name.text == field)
            .and_then(|f| f.external_name.clone()),
        _ => None,
    }
}
