//! The tool manifest of decision D6 (`renyi tools`): every function of the
//! project with `expose as tool`, with a JSON Schema for its parameters and
//! its result built from the checked types, its `purpose:` as the
//! description and its `needs` as the permissions. The manifest is what an
//! agent host reads to call the function as a tool; `renyi serve --mcp`
//! (milestone M5) will serve the same entries.
//!
//! The schemas follow the `ToJson` and `FromJson` rules of the library
//! sketch (section 7): a record is an object keyed by its field names or
//! their `as` names, a sum type is an object with a `kind` key next to the
//! variant's fields, `maybe` admits `null`, a list is an array, a map keyed
//! by `Text` is an object, the numbers are numbers, `Bytes` is base64 and an
//! `Instant` or a `Date` is its text. A type that refers to itself is
//! written once under `$defs` and referred to from there.

use renyi_check::effects::Capability;
use renyi_check::types::{ParamId, Ty};
use renyi_check::world::{FieldInfo, TypeKindInfo, World};
use renyi_check::{check_project_in, Library, TypeId};
use renyi_syntax::json::Json;
use renyi_syntax::SourceFile;

/// One tool of the manifest.
#[derive(Clone, Debug)]
pub struct Tool {
    /// `module.function`.
    pub name: String,
    pub module: String,
    pub function: String,
    /// The `purpose:` clause.
    pub description: Option<String>,
    /// A JSON Schema object with one property per parameter.
    pub input_schema: Json,
    /// The schema of the result; `None` for a function without `returns`.
    pub output_schema: Option<Json>,
    /// The declared failure types, as the checker spells them.
    pub fails: Vec<String>,
    /// The `needs` clause, one capability per entry.
    pub permissions: Vec<String>,
    pub file: String,
    pub line: usize,
}

/// The tools of a project checked against the standard library.
pub fn tools_of(files: &[SourceFile]) -> Vec<Tool> {
    tools_of_in(&Library::standard(), files)
}

/// The tools of a project: its files are checked together, against the
/// declaration files of the toolchain's extensions (decision AJ1); a file
/// that does not check still contributes nothing, so the caller reports
/// errors first.
pub fn tools_of_in(library: &Library, files: &[SourceFile]) -> Vec<Tool> {
    let checked = check_project_in(library, files, &[]);
    let world = &checked.world;
    let mut tools = Vec::new();
    for (id, info) in world.functions.iter().enumerate() {
        if info.is_library || !info.expose_as_tool {
            continue;
        }
        let Some(file) = checked
            .modules
            .iter()
            .find(|module| module.id == Some(info.module))
            .and_then(|module| files.get(module.file))
        else {
            continue;
        };
        let module_name = world.modules[info.module].name.clone();
        let purpose = world.function_purpose(id);
        let mut builder = SchemaBuilder::new(world);
        let mut properties = Vec::new();
        let mut required = Vec::new();
        for (name, ty) in &info.params {
            properties.push((leak(name), builder.schema(ty)));
            if !matches!(ty, Ty::Maybe(_)) {
                required.push(Json::String(name.clone()));
            }
        }
        let mut input = vec![
            ("type", Json::String("object".to_string())),
            ("properties", Json::Object(properties)),
            ("required", Json::Array(required)),
            ("additionalProperties", Json::Bool(false)),
        ];
        let output_schema = info.returns.as_ref().map(|ty| builder.schema(ty));
        let defs = builder.definitions();
        let output_schema = output_schema.map(|schema| with_defs(schema, &defs));
        if !defs.is_empty() {
            input.push(("$defs", Json::Object(defs)));
        }
        tools.push(Tool {
            name: format!("{module_name}.{}", info.name),
            module: module_name,
            function: info.name.clone(),
            description: purpose,
            input_schema: Json::Object(input),
            output_schema,
            fails: info.fails.iter().map(|ty| world.show(ty)).collect(),
            permissions: info.needs.iter().map(Capability::spelling).collect(),
            file: file.name.replace('\\', "/"),
            line: file.position(info.span.start).line,
        });
    }
    tools
}

/// The manifest as one JSON document: an array with one object per tool.
pub fn manifest_json(tools: &[Tool]) -> Json {
    Json::Array(
        tools
            .iter()
            .map(|tool| {
                Json::Object(vec![
                    ("name", Json::String(tool.name.clone())),
                    (
                        "description",
                        tool.description.clone().map_or(Json::Null, Json::String),
                    ),
                    ("input_schema", tool.input_schema.clone()),
                    (
                        "output_schema",
                        tool.output_schema.clone().unwrap_or(Json::Null),
                    ),
                    (
                        "fails",
                        Json::Array(tool.fails.iter().cloned().map(Json::String).collect()),
                    ),
                    (
                        "permissions",
                        Json::Array(tool.permissions.iter().cloned().map(Json::String).collect()),
                    ),
                    ("module", Json::String(tool.module.clone())),
                    ("function", Json::String(tool.function.clone())),
                    ("file", Json::String(tool.file.clone())),
                    ("line", Json::Number(tool.line)),
                ])
            })
            .collect(),
    )
}

/// A schema with the shared definitions attached, when there are any.
fn with_defs(schema: Json, defs: &[(&'static str, Json)]) -> Json {
    if defs.is_empty() {
        return schema;
    }
    match schema {
        Json::Object(mut fields) => {
            fields.push(("$defs", Json::Object(defs.to_vec())));
            Json::Object(fields)
        }
        other => other,
    }
}

/// JSON keys are static strings in the syntax crate's JSON type; a schema's
/// keys come from the program, so they are leaked once per manifest.
fn leak(name: &str) -> &'static str {
    Box::leak(name.to_string().into_boxed_str())
}

fn typed(kind: &'static str) -> Json {
    Json::Object(vec![("type", Json::String(kind.to_string()))])
}

struct SchemaBuilder<'w> {
    world: &'w World,
    /// The types being expanded, innermost last: a reference to one of them
    /// is recursion.
    stack: Vec<TypeId>,
    /// The types that referred to themselves, each written once under `$defs`.
    recursive: Vec<TypeId>,
}

impl<'w> SchemaBuilder<'w> {
    fn new(world: &'w World) -> SchemaBuilder<'w> {
        SchemaBuilder {
            world,
            stack: Vec::new(),
            recursive: Vec::new(),
        }
    }

    fn schema(&mut self, ty: &Ty) -> Json {
        match ty {
            Ty::Maybe(inner) => Json::Object(vec![(
                "anyOf",
                Json::Array(vec![self.schema(inner), typed("null")]),
            )]),
            Ty::App(id, args) => self.named(*id, args),
            // a type parameter, a function or an error union is not JSON; the
            // checker has rejected it (`tool-type`), so anything is admitted
            _ => Json::Object(Vec::new()),
        }
    }

    fn named(&mut self, id: TypeId, args: &[Ty]) -> Json {
        let b = &self.world.builtins;
        if id == b.integer {
            return typed("integer");
        }
        if id == b.decimal {
            return Json::Object(vec![
                ("type", Json::String("number".to_string())),
                ("format", Json::String("decimal".to_string())),
            ]);
        }
        if id == b.float {
            return typed("number");
        }
        if id == b.boolean {
            return typed("boolean");
        }
        if id == b.text {
            return typed("string");
        }
        if id == b.bytes {
            return Json::Object(vec![
                ("type", Json::String("string".to_string())),
                ("contentEncoding", Json::String("base64".to_string())),
            ]);
        }
        if id == b.duration {
            return Json::Object(vec![
                ("type", Json::String("integer".to_string())),
                ("description", Json::String("milliseconds".to_string())),
            ]);
        }
        if id == b.list || id == b.set {
            let items = args
                .first()
                .map_or(Json::Object(Vec::new()), |item| self.schema(item));
            let mut fields = vec![
                ("type", Json::String("array".to_string())),
                ("items", items),
            ];
            if id == b.set {
                fields.push(("uniqueItems", Json::Bool(true)));
            }
            return Json::Object(fields);
        }
        if id == b.map {
            let values = args
                .get(1)
                .map_or(Json::Object(Vec::new()), |value| self.schema(value));
            return Json::Object(vec![
                ("type", Json::String("object".to_string())),
                ("additionalProperties", values),
            ]);
        }
        if id == b.pair {
            let items: Vec<Json> = args.iter().map(|arg| self.schema(arg)).collect();
            return Json::Object(vec![
                ("type", Json::String("array".to_string())),
                ("prefixItems", Json::Array(items)),
                ("minItems", Json::Number(2)),
                ("maxItems", Json::Number(2)),
            ]);
        }
        let info = &self.world.types[id];
        match &info.kind {
            TypeKindInfo::Subtype { base, .. } => {
                let base = base.clone();
                self.schema(&base)
            }
            TypeKindInfo::Opaque | TypeKindInfo::Unresolved => {
                if info.name == "Instant" || info.name == "Date" {
                    typed("string")
                } else {
                    Json::Object(Vec::new())
                }
            }
            TypeKindInfo::Record(_) | TypeKindInfo::Sum(_) => {
                if self.stack.contains(&id) {
                    if !self.recursive.contains(&id) {
                        self.recursive.push(id);
                    }
                    return Json::Object(vec![(
                        "$ref",
                        Json::String(format!("#/$defs/{}", self.definition_name(id))),
                    )]);
                }
                self.stack.push(id);
                let schema = self.structure(id, args);
                self.stack.pop();
                schema
            }
        }
    }

    /// The object schema of a record or the `oneOf` of a sum type, with the
    /// type's parameters replaced by the arguments.
    fn structure(&mut self, id: TypeId, args: &[Ty]) -> Json {
        let info = &self.world.types[id];
        let params = info.params.clone();
        let substitute = |ty: &Ty| {
            ty.substitute(&|param: ParamId| {
                params
                    .iter()
                    .position(|p| *p == param)
                    .and_then(|index| args.get(index).cloned())
            })
        };
        match &info.kind {
            TypeKindInfo::Record(fields) => {
                let fields = fields.clone();
                self.object(&fields, &substitute, None)
            }
            TypeKindInfo::Sum(variants) => {
                let variants = variants.clone();
                let options: Vec<Json> = variants
                    .iter()
                    .map(|variant| self.object(&variant.fields, &substitute, Some(&variant.name)))
                    .collect();
                Json::Object(vec![("oneOf", Json::Array(options))])
            }
            _ => Json::Object(Vec::new()),
        }
    }

    /// An object with one property per field; a variant's object carries
    /// its name under `kind` first.
    fn object(
        &mut self,
        fields: &[FieldInfo],
        substitute: &dyn Fn(&Ty) -> Ty,
        variant: Option<&str>,
    ) -> Json {
        let mut properties = Vec::new();
        let mut required = Vec::new();
        if let Some(name) = variant {
            properties.push((
                "kind",
                Json::Object(vec![("const", Json::String(name.to_string()))]),
            ));
            required.push(Json::String("kind".to_string()));
        }
        for field in fields {
            let key = field
                .external_name
                .clone()
                .unwrap_or_else(|| field.name.clone());
            let ty = substitute(&field.ty);
            properties.push((leak(&key), self.schema(&ty)));
            if !matches!(ty, Ty::Maybe(_)) {
                required.push(Json::String(key));
            }
        }
        Json::Object(vec![
            ("type", Json::String("object".to_string())),
            ("properties", Json::Object(properties)),
            ("required", Json::Array(required)),
            ("additionalProperties", Json::Bool(false)),
        ])
    }

    /// The key of a type under `$defs`: its name, qualified by its module
    /// when another type of the project has the same name.
    fn definition_name(&self, id: TypeId) -> String {
        let info = &self.world.types[id];
        let clash = self
            .world
            .types
            .iter()
            .enumerate()
            .any(|(other, candidate)| other != id && candidate.name == info.name);
        if clash {
            format!("{}.{}", self.world.modules[info.module].name, info.name)
        } else {
            info.name.clone()
        }
    }

    /// The definitions of the recursive types met so far, each expanded once
    /// with its own parameters left open.
    fn definitions(&mut self) -> Vec<(&'static str, Json)> {
        let mut defs = Vec::new();
        let mut done = 0;
        while done < self.recursive.len() {
            let id = self.recursive[done];
            done += 1;
            self.stack.push(id);
            let schema = self.structure(id, &[]);
            self.stack.pop();
            defs.push((leak(&self.definition_name(id)), schema));
        }
        defs
    }
}
