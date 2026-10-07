//! The bytecode file (decisions Z1 to Z4): a compiled program as the derived
//! JSON of the Renyi types of `compiler/bytecode.ry`, written by `renyi
//! compile` and by the emitter written in Renyi, loaded by `renyi run`,
//! `record`, `test` and `reproduce` when the path ends in `.ryc`. The
//! encoder here and those types are one format: a record is an object whose
//! keys are its fields in declaration order, a variant an object with
//! `kind` first and its fields after it, a `maybe` without a value `null`,
//! a list an array, and every span counts characters; the document is
//! rendered as `json.render_indented` renders it, so that the two writers
//! can be compared byte for byte (decision Z3). The loader reads the
//! document with the VM's own JSON reader (decision Z2) and refuses
//! anything that does not fit, naming the place.

use std::collections::HashMap;

use renyi_check::effects::Capability;
use renyi_check::types::{FunctionTy, Ty};
use renyi_check::world::Builtins;
use renyi_syntax::ast::BinaryOp;
use renyi_syntax::json::Json as Out;
use renyi_syntax::Span;

use crate::bytecode::{Code, CodeKind, GroupFold, Op};
use crate::compile::{
    CodeId, ConstantMeta, ExampleMeta, Expected, Foreign, FunctionMeta, Program, SourceLines,
    Specials, TestMeta,
};
use crate::decimal::Decimal;
use crate::integer::Int;
use crate::natives::json::{read_json, Json as In};
use crate::render::float_text;
use crate::types::{FieldMeta, TypeMeta, TypeShape, Types, VariantMeta};
use crate::value::Value;

/// The version of the format, the first field of the file.
pub const FORMAT: usize = 2;

/// The extension of a bytecode file.
pub const EXTENSION: &str = "ryc";

/// Whether a path names a bytecode file.
pub fn is_bytecode(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .is_some_and(|extension| extension == EXTENSION)
}

// ----------------------------------------------------------------- writing

fn string(value: &str) -> Out {
    Out::String(value.to_string())
}

fn number(value: usize) -> Out {
    Out::Number(value)
}

fn optional<T>(value: Option<&T>, encode: impl FnOnce(&T) -> Out) -> Out {
    value.map(encode).unwrap_or(Out::Null)
}

fn array<T>(items: &[T], encode: impl Fn(&T) -> Out) -> Out {
    Out::Array(items.iter().map(encode).collect())
}

fn numbers(items: &[usize]) -> Out {
    array(items, |item| number(*item))
}

fn strings(items: &[String]) -> Out {
    array(items, |item| string(item))
}

fn record(fields: Vec<(&'static str, Out)>) -> Out {
    Out::Object(fields)
}

/// A variant of a sum type: `kind` first, then its fields.
fn variant(kind: &'static str, fields: Vec<(&'static str, Out)>) -> Out {
    let mut all = vec![("kind", string(kind))];
    all.extend(fields);
    Out::Object(all)
}

fn span_json(span: Span) -> Out {
    record(vec![
        ("start", number(span.start)),
        ("stop", number(span.end)),
    ])
}

/// The program as the text of its file.
pub fn render(program: &Program) -> String {
    encode(program).render()
}

fn encode(program: &Program) -> Out {
    let modules: Vec<Out> = (0..program.module_names.len())
        .map(|id| {
            let source = program.sources.get(id).and_then(|source| source.as_ref());
            record(vec![
                ("name", string(&program.module_names[id])),
                (
                    "path",
                    optional(source.map(|source| &source.name), |name| string(name)),
                ),
                (
                    "line_starts",
                    numbers(source.map_or(&[][..], |source| &source.line_starts)),
                ),
            ])
        })
        .collect();
    let mut impls: Vec<_> = program.types.impls.iter().collect();
    impls.sort_by_key(|(key, _)| **key);
    let impls: Vec<Out> = impls
        .into_iter()
        .map(|((ability, ty), methods)| {
            let mut methods: Vec<_> = methods.iter().collect();
            methods.sort_by_key(|(name, _)| *name);
            record(vec![
                ("ability_id", number(*ability)),
                ("type_id", number(*ty)),
                (
                    "methods",
                    Out::Array(
                        methods
                            .into_iter()
                            .map(|(name, function)| {
                                record(vec![
                                    ("name", string(name)),
                                    ("function_id", number(*function)),
                                ])
                            })
                            .collect(),
                    ),
                ),
            ])
        })
        .collect();
    let mut method_index: Vec<_> = program.method_index.iter().collect();
    method_index.sort_by_key(|(key, _)| *key);
    let method_index: Vec<Out> = method_index
        .into_iter()
        .map(|((ty, name), functions)| {
            record(vec![
                ("type_id", number(*ty)),
                ("name", string(name)),
                ("functions", numbers(functions)),
            ])
        })
        .collect();
    let functions: Vec<Out> = program
        .function_metas
        .iter()
        .enumerate()
        .map(|(id, meta)| function_json(meta, program.functions.get(&id).copied()))
        .collect();
    record(vec![
        ("format", number(FORMAT)),
        ("modules", Out::Array(modules)),
        ("builtins", builtins_json(&program.builtins)),
        ("types", array(&program.types.metas, type_meta_json)),
        ("impls", Out::Array(impls)),
        (
            "abilities",
            array(&program.abilities, |(name, methods)| {
                record(vec![("name", string(name)), ("methods", strings(methods))])
            }),
        ),
        ("method_index", Out::Array(method_index)),
        ("functions", Out::Array(functions)),
        ("constants", array(&program.constants, constant_meta_json)),
        ("tests", array(&program.tests, test_json)),
        ("examples", array(&program.examples, example_json)),
        ("codes", array(&program.codes, code_json)),
        ("result_types", array(&program.result_types, type_json)),
        ("specials", array(&program.specials, specials_json)),
        ("field_sites", number(program.field_sites)),
        ("main", optional(program.main.as_ref(), |id| number(*id))),
        ("date", optional(program.date.as_ref(), |id| number(*id))),
    ])
}

fn builtins_json(builtins: &Builtins) -> Out {
    record(vec![
        ("integer", number(builtins.integer)),
        ("decimal", number(builtins.decimal)),
        ("float", number(builtins.float)),
        ("boolean", number(builtins.boolean)),
        ("text", number(builtins.text)),
        ("bytes", number(builtins.bytes)),
        ("list", number(builtins.list)),
        ("map", number(builtins.map)),
        ("set_type", number(builtins.set)),
        ("range", number(builtins.range)),
        ("pair", number(builtins.pair)),
        ("duration", number(builtins.duration)),
        ("ordering", number(builtins.ordering)),
        ("timed_out", number(builtins.timed_out)),
        (
            "constraint_violation",
            number(builtins.constraint_violation),
        ),
        ("guarded", number(builtins.guarded)),
        ("equal", number(builtins.equal)),
        ("compare", number(builtins.compare)),
        ("hash", number(builtins.hash)),
        ("to_text", number(builtins.to_text)),
        ("iterable", number(builtins.iterable)),
    ])
}

fn type_meta_json(meta: &TypeMeta) -> Out {
    record(vec![
        ("id", number(meta.id)),
        ("name", string(&meta.name)),
        ("module_name", string(&meta.module)),
        ("is_library", Out::Bool(meta.is_library)),
        ("params", numbers(&meta.params)),
        ("shape", shape_json(&meta.shape)),
        ("derives", numbers(&meta.derives)),
        (
            "compare_by",
            optional(meta.compare_by.as_ref(), |fields| numbers(fields)),
        ),
        ("refinements", numbers(&meta.refinements)),
    ])
}

fn shape_json(shape: &TypeShape) -> Out {
    match shape {
        TypeShape::Opaque => variant("OpaqueShape", vec![]),
        TypeShape::Record(fields) => {
            variant("RecordShape", vec![("fields", array(fields, field_json))])
        }
        TypeShape::Sum(variants) => variant(
            "SumShape",
            vec![(
                "variants",
                array(variants, |variant: &VariantMeta| {
                    record(vec![
                        ("name", string(&variant.name)),
                        ("fields", array(&variant.fields, field_json)),
                    ])
                }),
            )],
        ),
        TypeShape::Subtype { base } => variant("SubtypeShape", vec![("base", type_json(base))]),
    }
}

fn field_json(field: &FieldMeta) -> Out {
    record(vec![
        ("name", string(&field.name)),
        ("ty", type_json(&field.ty)),
        (
            "external_name",
            optional(field.external_name.as_ref(), |name| string(name)),
        ),
        ("optional", Out::Bool(field.optional)),
        (
            "refinement",
            optional(field.refinement.as_ref(), |index| number(*index)),
        ),
    ])
}

fn function_json(meta: &FunctionMeta, code: Option<CodeId>) -> Out {
    record(vec![
        ("module_name", string(&meta.module)),
        ("name", string(&meta.name)),
        ("is_library", Out::Bool(meta.is_library)),
        ("is_method", Out::Bool(meta.is_method)),
        ("params", strings(&meta.params)),
        (
            "receiver",
            optional(meta.receiver.as_ref(), |receiver| string(receiver)),
        ),
        ("param_types", strings(&meta.param_types)),
        ("result", optional(meta.returns.as_ref(), type_json)),
        ("failures", array(&meta.fails, type_json)),
        ("capabilities", array(&meta.needs, grant_json)),
        (
            "summary",
            optional(meta.purpose.as_ref(), |purpose| string(purpose)),
        ),
        ("code", optional(code.as_ref(), |code| number(*code))),
        ("foreign", optional(meta.foreign.as_ref(), foreign_json)),
    ])
}

fn foreign_json(foreign: &Foreign) -> Out {
    record(vec![
        ("libraries", strings(&foreign.libraries)),
        ("symbol", string(&foreign.symbol)),
        ("parameters", strings(&foreign.parameters)),
        ("result", string(&foreign.result)),
    ])
}

fn constant_meta_json(meta: &ConstantMeta) -> Out {
    record(vec![
        ("owner", number(meta.module)),
        ("name", string(&meta.name)),
        ("code", number(meta.code)),
    ])
}

fn test_json(meta: &TestMeta) -> Out {
    record(vec![
        ("owner", number(meta.module)),
        ("name", string(&meta.name)),
        ("capabilities", array(&meta.needs, grant_json)),
        (
            "fixture",
            optional(meta.replays.as_ref(), |fixture| string(fixture)),
        ),
        ("span", span_json(meta.span)),
        ("code", number(meta.code)),
    ])
}

fn example_json(meta: &ExampleMeta) -> Out {
    let expected = match &meta.expected {
        Expected::Is(code) => variant("IsExpected", vec![("code", number(*code))]),
        Expected::FailsWith(code) => variant("FailsWithExpected", vec![("code", number(*code))]),
    };
    record(vec![
        ("function_id", number(meta.function)),
        ("owner", number(meta.module)),
        ("text", string(&meta.text)),
        ("span", span_json(meta.span)),
        ("call", number(meta.call)),
        ("expected", expected),
    ])
}

fn specials_json(specials: &Specials) -> Out {
    let id = |id: Option<usize>| optional(id.as_ref(), |id| number(*id));
    record(vec![
        ("equals", id(specials.equals)),
        ("compare", id(specials.compare)),
        ("to_text", id(specials.to_text)),
        ("to_list", id(specials.to_list)),
    ])
}

fn code_json(code: &Code) -> Out {
    let kind = match code.kind {
        CodeKind::Function => "FunctionCode",
        CodeKind::Test => "TestCode",
        CodeKind::Constant => "ConstantCode",
        CodeKind::Example => "ExampleCode",
        CodeKind::Refinement => "RefinementCode",
    };
    record(vec![
        ("name", string(&code.name)),
        ("owner", number(code.module)),
        ("kind", variant(kind, vec![])),
        (
            "function_id",
            optional(code.function.as_ref(), |id| number(*id)),
        ),
        ("params", number(code.params as usize)),
        ("locals", number(code.locals as usize)),
        ("ops", array(&code.ops, op_json)),
        ("spans", array(&code.spans, |span| span_json(*span))),
        ("constants", array(&code.constants, constant_json)),
    ])
}

/// A literal of a code object; the compiler puts nothing else there.
fn constant_json(value: &Value) -> Out {
    match value {
        Value::Nothing => variant("NothingConstant", vec![]),
        Value::Boolean(value) => variant("BooleanConstant", vec![("value", Out::Bool(*value))]),
        Value::Integer(value) => variant(
            "IntegerConstant",
            vec![("digits", string(&value.to_string()))],
        ),
        Value::Decimal(value) => variant(
            "DecimalConstant",
            vec![("digits", string(&value.to_string()))],
        ),
        Value::Float(value) => variant(
            "FloatConstant",
            vec![("digits", string(&float_text(*value)))],
        ),
        Value::Text(text) => variant("TextConstant", vec![("text", string(text))]),
        Value::Function(id) => variant("FunctionConstant", vec![("function_id", number(*id))]),
        other => panic!(
            "a constant of kind {}, which the file cannot hold",
            other.kind_name()
        ),
    }
}

fn operator_name(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "Add",
        BinaryOp::Subtract => "Subtract",
        BinaryOp::Multiply => "Multiply",
        BinaryOp::Divide => "Divide",
        BinaryOp::Remainder => "Remainder",
        BinaryOp::Power => "Power",
        BinaryOp::Is => "Is",
        BinaryOp::IsNot => "IsNot",
        BinaryOp::IsLessThan => "IsLessThan",
        BinaryOp::IsAtMost => "IsAtMost",
        BinaryOp::IsGreaterThan => "IsGreaterThan",
        BinaryOp::IsAtLeast => "IsAtLeast",
        BinaryOp::And => "And",
        BinaryOp::Or => "Or",
    }
}

fn operator_of(name: &str) -> Option<BinaryOp> {
    Some(match name {
        "Add" => BinaryOp::Add,
        "Subtract" => BinaryOp::Subtract,
        "Multiply" => BinaryOp::Multiply,
        "Divide" => BinaryOp::Divide,
        "Remainder" => BinaryOp::Remainder,
        "Power" => BinaryOp::Power,
        "Is" => BinaryOp::Is,
        "IsNot" => BinaryOp::IsNot,
        "IsLessThan" => BinaryOp::IsLessThan,
        "IsAtMost" => BinaryOp::IsAtMost,
        "IsGreaterThan" => BinaryOp::IsGreaterThan,
        "IsAtLeast" => BinaryOp::IsAtLeast,
        "And" => BinaryOp::And,
        "Or" => BinaryOp::Or,
        _ => return None,
    })
}

fn op_json(op: &Op) -> Out {
    let small = |value: u16| number(value as usize);
    let wide = |value: u32| number(value as usize);
    match op {
        Op::Const(index) => variant("OpConst", vec![("index", wide(*index))]),
        Op::Nothing => variant("OpNothing", vec![]),
        Op::Global(index) => variant("OpGlobal", vec![("index", wide(*index))]),
        Op::Load(slot) => variant("OpLoad", vec![("slot", small(*slot))]),
        Op::LoadMove(slot) => variant("OpLoadMove", vec![("slot", small(*slot))]),
        Op::Store(slot) => variant("OpStore", vec![("slot", small(*slot))]),
        Op::Pop => variant("OpPop", vec![]),
        Op::Dup => variant("OpDup", vec![]),
        Op::MakeList(size) => variant("OpMakeList", vec![("size", small(*size))]),
        Op::MakeMap(entries) => variant("OpMakeMap", vec![("entries", small(*entries))]),
        Op::MakePair => variant("OpMakePair", vec![]),
        Op::MakeRange { stepped } => variant("OpMakeRange", vec![("stepped", Out::Bool(*stepped))]),
        Op::Construct { ty, fields } => variant(
            "OpConstruct",
            vec![("ty", number(*ty)), ("fields", small(*fields))],
        ),
        Op::ConstructVariant { ty, tag, fields } => variant(
            "OpConstructVariant",
            vec![
                ("ty", number(*ty)),
                ("tag", small(*tag)),
                ("fields", small(*fields)),
            ],
        ),
        Op::Field { name, site } => variant(
            "OpField",
            vec![("name", wide(*name)), ("site", wide(*site))],
        ),
        Op::With(fields) => variant("OpWith", vec![("fields", small(*fields))]),
        Op::Call { function, args } => variant(
            "OpCall",
            vec![("function_id", number(*function)), ("args", small(*args))],
        ),
        Op::CallAbility {
            ability,
            method,
            args,
        } => variant(
            "OpCallAbility",
            vec![
                ("ability_id", number(*ability)),
                ("method", small(*method)),
                ("args", small(*args)),
            ],
        ),
        Op::CallValue(args) => variant("OpCallValue", vec![("args", small(*args))]),
        Op::ResultType(index) => variant("OpResultType", vec![("index", wide(*index))]),
        Op::Not => variant("OpNot", vec![]),
        Op::Binary(operator) => variant(
            "OpBinary",
            vec![("op", variant(operator_name(*operator), vec![]))],
        ),
        Op::ToText => variant("OpToText", vec![]),
        Op::Concat(pieces) => variant("OpConcat", vec![("pieces", small(*pieces))]),
        Op::Jump(target) => variant("OpJump", vec![("target", wide(*target))]),
        Op::JumpIfFalse(target) => variant("OpJumpIfFalse", vec![("target", wide(*target))]),
        Op::JumpIfTrue(target) => variant("OpJumpIfTrue", vec![("target", wide(*target))]),
        Op::JumpIfAbsent(target) => variant("OpJumpIfAbsent", vec![("target", wide(*target))]),
        Op::JumpIfFailure(target) => variant("OpJumpIfFailure", vec![("target", wide(*target))]),
        Op::PushHandler(target) => variant("OpPushHandler", vec![("target", wide(*target))]),
        Op::PopHandler => variant("OpPopHandler", vec![]),
        Op::Return => variant("OpReturn", vec![]),
        Op::ReturnNothing => variant("OpReturnNothing", vec![]),
        Op::Fail => variant("OpFail", vec![]),
        Op::Crash => variant("OpCrash", vec![]),
        Op::IsVariant(tag) => variant("OpIsVariant", vec![("tag", small(*tag))]),
        Op::IsNothing => variant("OpIsNothing", vec![]),
        Op::IsFailure => variant("OpIsFailure", vec![]),
        Op::IsType(ty) => variant("OpIsType", vec![("ty", number(*ty))]),
        Op::Unpack(parts) => variant("OpUnpack", vec![("parts", small(*parts))]),
        Op::UnwrapFailure => variant("OpUnwrapFailure", vec![]),
        Op::IterInit(slot) => variant("OpIterInit", vec![("slot", small(*slot))]),
        Op::IterNext { slot, exit } => variant(
            "OpIterNext",
            vec![("slot", small(*slot)), ("exit", wide(*exit))],
        ),
        Op::ListPush => variant("OpListPush", vec![]),
        Op::GroupInsert => variant("OpGroupInsert", vec![]),
        Op::GroupFold(fold) => {
            let fold = match fold {
                GroupFold::Sum => "SumFold",
                GroupFold::First => "FirstFold",
                GroupFold::Any => "AnyFold",
                GroupFold::All => "AllFold",
            };
            variant("OpGroupFold", vec![("fold", variant(fold, vec![]))])
        }
        Op::SortByKey { descending } => variant(
            "OpSortByKey",
            vec![("is_descending", Out::Bool(*descending))],
        ),
        Op::Deadline(slot) => variant("OpDeadline", vec![("slot", small(*slot))]),
        Op::CheckDeadline(slot) => variant("OpCheckDeadline", vec![("slot", small(*slot))]),
        Op::MarkStack(slot) => variant("OpMarkStack", vec![("slot", small(*slot))]),
        Op::UnwindStack(slot) => variant("OpUnwindStack", vec![("slot", small(*slot))]),
        Op::Check(index) => variant("OpCheck", vec![("index", wide(*index))]),
    }
}

/// A type as `compiler/types.ry` declares it.
fn type_json(ty: &Ty) -> Out {
    match ty {
        Ty::App(id, args) => variant(
            "App",
            vec![("id", number(*id)), ("args", array(args, type_json))],
        ),
        Ty::Maybe(inner) => variant("MaybeTy", vec![("inner", type_json(inner))]),
        Ty::Function(function) => variant(
            "FunctionTy",
            vec![(
                "signature",
                record(vec![
                    ("params", array(&function.params, type_json)),
                    ("result", optional(function.returns.as_ref(), type_json)),
                    ("failures", array(&function.fails, type_json)),
                    ("capabilities", array(&function.needs, grant_json)),
                ]),
            )],
        ),
        Ty::Param(id) => variant("Param", vec![("id", number(*id))]),
        Ty::Var(id) => variant("Var", vec![("id", number(*id))]),
        Ty::SelfType => variant("SelfTy", vec![]),
        Ty::Union(members) => variant("Union", vec![("members", array(members, type_json))]),
        Ty::Unit => variant("Unit", vec![]),
        Ty::Never => variant("Never", vec![]),
        Ty::Error => variant("ErrorTy", vec![]),
    }
}

/// A capability as `compiler/effects.ry` declares a `Grant`.
fn grant_json(capability: &Capability) -> Out {
    record(vec![
        ("path", strings(&capability.path)),
        (
            "scope",
            optional(capability.scope.as_ref(), |scope| string(scope)),
        ),
        (
            "budget",
            optional(capability.budget.as_ref(), |(limit, unit)| {
                record(vec![("limit", string(limit)), ("unit", string(unit))])
            }),
        ),
        (
            "only_to",
            array(&capability.only_to, |(path, scope)| {
                record(vec![
                    ("path", strings(path)),
                    ("scope", optional(scope.as_ref(), |scope| string(scope))),
                ])
            }),
        ),
    ])
}

// ----------------------------------------------------------------- reading

type Fields = [(String, In)];
type Read<T> = Result<T, String>;

/// The program a file holds; the error names the place that does not fit.
pub fn load(text: &str) -> Read<Program> {
    let json = read_json(text).map_err(|(detail, line)| format!("line {line}: {detail}"))?;
    let top = object(&json, "the file")?;
    let format = usize_at(top, "format", "the file")?;
    if format != FORMAT {
        return Err(format!(
            "the file is format {format}; this VM reads format {FORMAT}"
        ));
    }
    let program = read_program(top)?;
    check(&program)?;
    Ok(program)
}

fn kind_name(json: &In) -> &'static str {
    match json {
        In::Null => "null",
        In::Boolean(_) => "a boolean",
        In::Number(_) => "a number",
        In::Text(_) => "a string",
        In::Array(_) => "an array",
        In::Object(_) => "an object",
    }
}

fn object<'a>(json: &'a In, at: &str) -> Read<&'a Fields> {
    match json {
        In::Object(fields) => Ok(fields),
        other => Err(format!(
            "{at}: expected an object, found {}",
            kind_name(other)
        )),
    }
}

fn array_of<'a>(json: &'a In, at: &str) -> Read<&'a [In]> {
    match json {
        In::Array(items) => Ok(items),
        other => Err(format!(
            "{at}: expected an array, found {}",
            kind_name(other)
        )),
    }
}

fn usize_of(json: &In, at: &str) -> Read<usize> {
    match json {
        In::Number(text) => text
            .parse()
            .map_err(|_| format!("{at}: `{text}` is not a whole number")),
        other => Err(format!(
            "{at}: expected a number, found {}",
            kind_name(other)
        )),
    }
}

fn text_of<'a>(json: &'a In, at: &str) -> Read<&'a str> {
    match json {
        In::Text(text) => Ok(text),
        other => Err(format!(
            "{at}: expected a string, found {}",
            kind_name(other)
        )),
    }
}

fn bool_of(json: &In, at: &str) -> Read<bool> {
    match json {
        In::Boolean(value) => Ok(*value),
        other => Err(format!(
            "{at}: expected a boolean, found {}",
            kind_name(other)
        )),
    }
}

/// A field of an object with the place it names, for the messages.
fn field<'a>(fields: &'a Fields, name: &str, at: &str) -> Read<(&'a In, String)> {
    fields
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| (value, format!("{at}.{name}")))
        .ok_or_else(|| format!("{at}: no `{name}`"))
}

fn usize_at(fields: &Fields, name: &str, at: &str) -> Read<usize> {
    let (json, at) = field(fields, name, at)?;
    usize_of(json, &at)
}

fn text_at(fields: &Fields, name: &str, at: &str) -> Read<String> {
    let (json, at) = field(fields, name, at)?;
    text_of(json, &at).map(str::to_string)
}

fn bool_at(fields: &Fields, name: &str, at: &str) -> Read<bool> {
    let (json, at) = field(fields, name, at)?;
    bool_of(json, &at)
}

fn optional_at<T>(
    fields: &Fields,
    name: &str,
    at: &str,
    read: impl FnOnce(&In, &str) -> Read<T>,
) -> Read<Option<T>> {
    let (json, at) = field(fields, name, at)?;
    match json {
        In::Null => Ok(None),
        other => read(other, &at).map(Some),
    }
}

fn list_at<T>(
    fields: &Fields,
    name: &str,
    at: &str,
    read: impl Fn(&In, &str) -> Read<T>,
) -> Read<Vec<T>> {
    let (json, at) = field(fields, name, at)?;
    list_of(json, &at, read)
}

fn list_of<T>(json: &In, at: &str, read: impl Fn(&In, &str) -> Read<T>) -> Read<Vec<T>> {
    array_of(json, at)?
        .iter()
        .enumerate()
        .map(|(index, item)| read(item, &format!("{at}[{index}]")))
        .collect()
}

fn usizes_at(fields: &Fields, name: &str, at: &str) -> Read<Vec<usize>> {
    list_at(fields, name, at, usize_of)
}

fn texts_at(fields: &Fields, name: &str, at: &str) -> Read<Vec<String>> {
    list_at(fields, name, at, |json, at| {
        text_of(json, at).map(str::to_string)
    })
}

fn object_at<'a>(fields: &'a Fields, name: &str, at: &str) -> Read<(&'a Fields, String)> {
    let (json, at) = field(fields, name, at)?;
    Ok((object(json, &at)?, at))
}

/// A variant: the object with its `kind`.
fn variant_of<'a>(json: &'a In, at: &str) -> Read<(&'a Fields, &'a str)> {
    let fields = object(json, at)?;
    let (kind, kind_at) = field(fields, "kind", at)?;
    Ok((fields, text_of(kind, &kind_at)?))
}

fn small(value: usize, at: &str) -> Read<u16> {
    u16::try_from(value).map_err(|_| format!("{at}: {value} does not fit a slot or a count"))
}

fn wide(value: usize, at: &str) -> Read<u32> {
    u32::try_from(value).map_err(|_| format!("{at}: {value} does not fit an index"))
}

fn read_program(top: &Fields) -> Read<Program> {
    let at = "the file";
    let modules = list_at(top, "modules", at, read_module)?;
    let (builtins, builtins_at) = object_at(top, "builtins", at)?;
    let builtins = read_builtins(builtins, &builtins_at)?;
    let metas = list_at(top, "types", at, read_type_meta)?;
    let impls = list_at(top, "impls", at, read_impl)?;
    let abilities = list_at(top, "abilities", at, read_ability)?;
    let method_index = list_at(top, "method_index", at, read_method_entry)?;
    let functions = list_at(top, "functions", at, read_function)?;
    let constants = list_at(top, "constants", at, read_constant_meta)?;
    let tests = list_at(top, "tests", at, read_test)?;
    let examples = list_at(top, "examples", at, read_example)?;
    let codes = list_at(top, "codes", at, read_code)?;
    let result_types = list_at(top, "result_types", at, read_type)?;
    let specials = list_at(top, "specials", at, read_specials)?;
    let field_sites = usize_at(top, "field_sites", at)?;
    let main = optional_at(top, "main", at, usize_of)?;
    let date = optional_at(top, "date", at, usize_of)?;
    let (module_names, sources): (Vec<String>, Vec<Option<SourceLines>>) =
        modules.into_iter().unzip();
    let function_codes: Vec<Option<CodeId>> = functions.iter().map(|(_, code)| *code).collect();
    let function_table: HashMap<usize, CodeId> = function_codes
        .iter()
        .enumerate()
        .filter_map(|(id, code)| code.map(|code| (id, code)))
        .collect();
    Ok(Program {
        codes,
        functions: function_table,
        function_codes,
        function_metas: functions.into_iter().map(|(meta, _)| meta).collect(),
        constants,
        tests,
        examples,
        types: Types {
            metas,
            impls: impls.into_iter().collect(),
        },
        builtins,
        result_types,
        sources,
        module_names,
        method_index: method_index.into_iter().collect(),
        abilities,
        main,
        field_sites,
        specials,
        date,
    })
}

fn read_module(json: &In, at: &str) -> Read<(String, Option<SourceLines>)> {
    let fields = object(json, at)?;
    let name = text_at(fields, "name", at)?;
    let path = optional_at(fields, "path", at, |json, at| {
        text_of(json, at).map(str::to_string)
    })?;
    let line_starts = usizes_at(fields, "line_starts", at)?;
    Ok((name, path.map(|name| SourceLines { name, line_starts })))
}

fn read_builtins(fields: &Fields, at: &str) -> Read<Builtins> {
    Ok(Builtins {
        integer: usize_at(fields, "integer", at)?,
        decimal: usize_at(fields, "decimal", at)?,
        float: usize_at(fields, "float", at)?,
        boolean: usize_at(fields, "boolean", at)?,
        text: usize_at(fields, "text", at)?,
        bytes: usize_at(fields, "bytes", at)?,
        list: usize_at(fields, "list", at)?,
        map: usize_at(fields, "map", at)?,
        set: usize_at(fields, "set_type", at)?,
        range: usize_at(fields, "range", at)?,
        pair: usize_at(fields, "pair", at)?,
        duration: usize_at(fields, "duration", at)?,
        ordering: usize_at(fields, "ordering", at)?,
        timed_out: usize_at(fields, "timed_out", at)?,
        constraint_violation: usize_at(fields, "constraint_violation", at)?,
        guarded: usize_at(fields, "guarded", at)?,
        equal: usize_at(fields, "equal", at)?,
        compare: usize_at(fields, "compare", at)?,
        hash: usize_at(fields, "hash", at)?,
        to_text: usize_at(fields, "to_text", at)?,
        iterable: usize_at(fields, "iterable", at)?,
    })
}

fn read_type_meta(json: &In, at: &str) -> Read<TypeMeta> {
    let fields = object(json, at)?;
    let (shape, shape_at) = field(fields, "shape", at)?;
    Ok(TypeMeta {
        id: usize_at(fields, "id", at)?,
        name: text_at(fields, "name", at)?,
        module: text_at(fields, "module_name", at)?,
        is_library: bool_at(fields, "is_library", at)?,
        params: usizes_at(fields, "params", at)?,
        shape: read_shape(shape, &shape_at)?,
        derives: usizes_at(fields, "derives", at)?,
        compare_by: optional_at(fields, "compare_by", at, |json, at| {
            list_of(json, at, usize_of)
        })?,
        refinements: usizes_at(fields, "refinements", at)?,
    })
}

fn read_shape(json: &In, at: &str) -> Read<TypeShape> {
    let (fields, kind) = variant_of(json, at)?;
    Ok(match kind {
        "OpaqueShape" => TypeShape::Opaque,
        "RecordShape" => TypeShape::Record(list_at(fields, "fields", at, read_field)?),
        "SumShape" => TypeShape::Sum(list_at(fields, "variants", at, |json, at| {
            let fields = object(json, at)?;
            Ok(VariantMeta {
                name: text_at(fields, "name", at)?,
                fields: list_at(fields, "fields", at, read_field)?,
            })
        })?),
        "SubtypeShape" => {
            let (base, base_at) = field(fields, "base", at)?;
            TypeShape::Subtype {
                base: read_type(base, &base_at)?,
            }
        }
        other => return Err(format!("{at}: unknown shape `{other}`")),
    })
}

fn read_field(json: &In, at: &str) -> Read<FieldMeta> {
    let fields = object(json, at)?;
    let (ty, ty_at) = field(fields, "ty", at)?;
    Ok(FieldMeta {
        name: text_at(fields, "name", at)?,
        ty: read_type(ty, &ty_at)?,
        external_name: optional_at(fields, "external_name", at, |json, at| {
            text_of(json, at).map(str::to_string)
        })?,
        optional: bool_at(fields, "optional", at)?,
        refinement: optional_at(fields, "refinement", at, usize_of)?,
    })
}

type ImplEntry = ((usize, usize), HashMap<String, usize>);

fn read_impl(json: &In, at: &str) -> Read<ImplEntry> {
    let fields = object(json, at)?;
    let methods = list_at(fields, "methods", at, |json, at| {
        let fields = object(json, at)?;
        Ok((
            text_at(fields, "name", at)?,
            usize_at(fields, "function_id", at)?,
        ))
    })?;
    Ok((
        (
            usize_at(fields, "ability_id", at)?,
            usize_at(fields, "type_id", at)?,
        ),
        methods.into_iter().collect(),
    ))
}

fn read_ability(json: &In, at: &str) -> Read<(String, Vec<String>)> {
    let fields = object(json, at)?;
    Ok((
        text_at(fields, "name", at)?,
        texts_at(fields, "methods", at)?,
    ))
}

fn read_method_entry(json: &In, at: &str) -> Read<((usize, String), Vec<usize>)> {
    let fields = object(json, at)?;
    Ok((
        (
            usize_at(fields, "type_id", at)?,
            text_at(fields, "name", at)?,
        ),
        usizes_at(fields, "functions", at)?,
    ))
}

fn read_function(json: &In, at: &str) -> Read<(FunctionMeta, Option<CodeId>)> {
    let fields = object(json, at)?;
    let meta = FunctionMeta {
        module: text_at(fields, "module_name", at)?,
        name: text_at(fields, "name", at)?,
        is_library: bool_at(fields, "is_library", at)?,
        is_method: bool_at(fields, "is_method", at)?,
        params: texts_at(fields, "params", at)?,
        receiver: optional_at(fields, "receiver", at, |json, at| {
            text_of(json, at).map(str::to_string)
        })?,
        param_types: texts_at(fields, "param_types", at)?,
        returns: optional_at(fields, "result", at, read_type)?,
        fails: list_at(fields, "failures", at, read_type)?,
        needs: list_at(fields, "capabilities", at, read_grant)?,
        purpose: optional_at(fields, "summary", at, |json, at| {
            text_of(json, at).map(str::to_string)
        })?,
        foreign: optional_at(fields, "foreign", at, read_foreign)?,
    };
    Ok((meta, optional_at(fields, "code", at, usize_of)?))
}

fn read_foreign(json: &In, at: &str) -> Read<Foreign> {
    let fields = object(json, at)?;
    Ok(Foreign {
        libraries: texts_at(fields, "libraries", at)?,
        symbol: text_at(fields, "symbol", at)?,
        parameters: texts_at(fields, "parameters", at)?,
        result: text_at(fields, "result", at)?,
    })
}

fn read_constant_meta(json: &In, at: &str) -> Read<ConstantMeta> {
    let fields = object(json, at)?;
    Ok(ConstantMeta {
        module: usize_at(fields, "owner", at)?,
        name: text_at(fields, "name", at)?,
        code: usize_at(fields, "code", at)?,
    })
}

fn read_test(json: &In, at: &str) -> Read<TestMeta> {
    let fields = object(json, at)?;
    let (span, span_at) = field(fields, "span", at)?;
    Ok(TestMeta {
        module: usize_at(fields, "owner", at)?,
        name: text_at(fields, "name", at)?,
        needs: list_at(fields, "capabilities", at, read_grant)?,
        replays: optional_at(fields, "fixture", at, |json, at| {
            text_of(json, at).map(str::to_string)
        })?,
        span: read_span(span, &span_at)?,
        code: usize_at(fields, "code", at)?,
    })
}

fn read_example(json: &In, at: &str) -> Read<ExampleMeta> {
    let fields = object(json, at)?;
    let (span, span_at) = field(fields, "span", at)?;
    let (expected, expected_at) = field(fields, "expected", at)?;
    let (expected_fields, kind) = variant_of(expected, &expected_at)?;
    let code = usize_at(expected_fields, "code", &expected_at)?;
    let expected = match kind {
        "IsExpected" => Expected::Is(code),
        "FailsWithExpected" => Expected::FailsWith(code),
        other => return Err(format!("{expected_at}: unknown expectation `{other}`")),
    };
    Ok(ExampleMeta {
        function: usize_at(fields, "function_id", at)?,
        module: usize_at(fields, "owner", at)?,
        text: text_at(fields, "text", at)?,
        span: read_span(span, &span_at)?,
        call: usize_at(fields, "call", at)?,
        expected,
    })
}

fn read_specials(json: &In, at: &str) -> Read<Specials> {
    let fields = object(json, at)?;
    Ok(Specials {
        equals: optional_at(fields, "equals", at, usize_of)?,
        compare: optional_at(fields, "compare", at, usize_of)?,
        to_text: optional_at(fields, "to_text", at, usize_of)?,
        to_list: optional_at(fields, "to_list", at, usize_of)?,
    })
}

fn read_span(json: &In, at: &str) -> Read<Span> {
    let fields = object(json, at)?;
    Ok(Span::new(
        usize_at(fields, "start", at)?,
        usize_at(fields, "stop", at)?,
    ))
}

fn read_code(json: &In, at: &str) -> Read<Code> {
    let fields = object(json, at)?;
    let (kind, kind_at) = field(fields, "kind", at)?;
    let (_, kind) = variant_of(kind, &kind_at)?;
    let kind = match kind {
        "FunctionCode" => CodeKind::Function,
        "TestCode" => CodeKind::Test,
        "ConstantCode" => CodeKind::Constant,
        "ExampleCode" => CodeKind::Example,
        "RefinementCode" => CodeKind::Refinement,
        other => return Err(format!("{kind_at}: unknown code kind `{other}`")),
    };
    let params = usize_at(fields, "params", at)?;
    let locals = usize_at(fields, "locals", at)?;
    Ok(Code {
        name: text_at(fields, "name", at)?,
        module: usize_at(fields, "owner", at)?,
        kind,
        function: optional_at(fields, "function_id", at, usize_of)?,
        params: small(params, &format!("{at}.params"))?,
        locals: small(locals, &format!("{at}.locals"))?,
        ops: list_at(fields, "ops", at, read_op)?,
        spans: list_at(fields, "spans", at, read_span)?,
        constants: list_at(fields, "constants", at, read_constant)?,
    })
}

fn read_constant(json: &In, at: &str) -> Read<Value> {
    let (fields, kind) = variant_of(json, at)?;
    Ok(match kind {
        "NothingConstant" => Value::Nothing,
        "BooleanConstant" => Value::Boolean(bool_at(fields, "value", at)?),
        "IntegerConstant" => {
            let digits = text_at(fields, "digits", at)?;
            Value::Integer(
                Int::parse(&digits).ok_or_else(|| format!("{at}: `{digits}` is not an Integer"))?,
            )
        }
        "DecimalConstant" => {
            let digits = text_at(fields, "digits", at)?;
            Value::decimal(
                Decimal::parse(&digits)
                    .ok_or_else(|| format!("{at}: `{digits}` is not a Decimal"))?,
            )
        }
        "FloatConstant" => {
            let digits = text_at(fields, "digits", at)?;
            Value::Float(
                digits
                    .parse()
                    .map_err(|_| format!("{at}: `{digits}` is not a Float"))?,
            )
        }
        "TextConstant" => Value::text(text_at(fields, "text", at)?),
        "FunctionConstant" => Value::Function(usize_at(fields, "function_id", at)?),
        other => return Err(format!("{at}: unknown constant `{other}`")),
    })
}

fn read_op(json: &In, at: &str) -> Read<Op> {
    let (fields, kind) = variant_of(json, at)?;
    let slot = |name: &str| -> Read<u16> {
        let value = usize_at(fields, name, at)?;
        small(value, &format!("{at}.{name}"))
    };
    let index = |name: &str| -> Read<u32> {
        let value = usize_at(fields, name, at)?;
        wide(value, &format!("{at}.{name}"))
    };
    Ok(match kind {
        "OpConst" => Op::Const(index("index")?),
        "OpNothing" => Op::Nothing,
        "OpGlobal" => Op::Global(index("index")?),
        "OpLoad" => Op::Load(slot("slot")?),
        "OpLoadMove" => Op::LoadMove(slot("slot")?),
        "OpStore" => Op::Store(slot("slot")?),
        "OpPop" => Op::Pop,
        "OpDup" => Op::Dup,
        "OpMakeList" => Op::MakeList(slot("size")?),
        "OpMakeMap" => Op::MakeMap(slot("entries")?),
        "OpMakePair" => Op::MakePair,
        "OpMakeRange" => Op::MakeRange {
            stepped: bool_at(fields, "stepped", at)?,
        },
        "OpConstruct" => Op::Construct {
            ty: usize_at(fields, "ty", at)?,
            fields: slot("fields")?,
        },
        "OpConstructVariant" => Op::ConstructVariant {
            ty: usize_at(fields, "ty", at)?,
            tag: slot("tag")?,
            fields: slot("fields")?,
        },
        "OpField" => Op::Field {
            name: index("name")?,
            site: index("site")?,
        },
        "OpWith" => Op::With(slot("fields")?),
        "OpCall" => Op::Call {
            function: usize_at(fields, "function_id", at)?,
            args: slot("args")?,
        },
        "OpCallAbility" => Op::CallAbility {
            ability: usize_at(fields, "ability_id", at)?,
            method: slot("method")?,
            args: slot("args")?,
        },
        "OpCallValue" => Op::CallValue(slot("args")?),
        "OpResultType" => Op::ResultType(index("index")?),
        "OpNot" => Op::Not,
        "OpBinary" => {
            let (op, op_at) = field(fields, "op", at)?;
            let (_, name) = variant_of(op, &op_at)?;
            Op::Binary(
                operator_of(name).ok_or_else(|| format!("{op_at}: unknown operator `{name}`"))?,
            )
        }
        "OpToText" => Op::ToText,
        "OpConcat" => Op::Concat(slot("pieces")?),
        "OpJump" => Op::Jump(index("target")?),
        "OpJumpIfFalse" => Op::JumpIfFalse(index("target")?),
        "OpJumpIfTrue" => Op::JumpIfTrue(index("target")?),
        "OpJumpIfAbsent" => Op::JumpIfAbsent(index("target")?),
        "OpJumpIfFailure" => Op::JumpIfFailure(index("target")?),
        "OpPushHandler" => Op::PushHandler(index("target")?),
        "OpPopHandler" => Op::PopHandler,
        "OpReturn" => Op::Return,
        "OpReturnNothing" => Op::ReturnNothing,
        "OpFail" => Op::Fail,
        "OpCrash" => Op::Crash,
        "OpIsVariant" => Op::IsVariant(slot("tag")?),
        "OpIsNothing" => Op::IsNothing,
        "OpIsFailure" => Op::IsFailure,
        "OpIsType" => Op::IsType(usize_at(fields, "ty", at)?),
        "OpUnpack" => Op::Unpack(slot("parts")?),
        "OpUnwrapFailure" => Op::UnwrapFailure,
        "OpIterInit" => Op::IterInit(slot("slot")?),
        "OpIterNext" => Op::IterNext {
            slot: slot("slot")?,
            exit: index("exit")?,
        },
        "OpListPush" => Op::ListPush,
        "OpGroupInsert" => Op::GroupInsert,
        "OpGroupFold" => {
            let (fold, fold_at) = field(fields, "fold", at)?;
            let (_, name) = variant_of(fold, &fold_at)?;
            Op::GroupFold(match name {
                "SumFold" => GroupFold::Sum,
                "FirstFold" => GroupFold::First,
                "AnyFold" => GroupFold::Any,
                "AllFold" => GroupFold::All,
                other => return Err(format!("{fold_at}: unknown fold `{other}`")),
            })
        }
        "OpSortByKey" => Op::SortByKey {
            descending: bool_at(fields, "is_descending", at)?,
        },
        "OpDeadline" => Op::Deadline(slot("slot")?),
        "OpCheckDeadline" => Op::CheckDeadline(slot("slot")?),
        "OpMarkStack" => Op::MarkStack(slot("slot")?),
        "OpUnwindStack" => Op::UnwindStack(slot("slot")?),
        "OpCheck" => Op::Check(index("index")?),
        other => return Err(format!("{at}: unknown operation `{other}`")),
    })
}

fn read_type(json: &In, at: &str) -> Read<Ty> {
    let (fields, kind) = variant_of(json, at)?;
    Ok(match kind {
        "App" => Ty::App(
            usize_at(fields, "id", at)?,
            list_at(fields, "args", at, read_type)?,
        ),
        "MaybeTy" => {
            let (inner, inner_at) = field(fields, "inner", at)?;
            Ty::Maybe(Box::new(read_type(inner, &inner_at)?))
        }
        "FunctionTy" => {
            let (signature, signature_at) = object_at(fields, "signature", at)?;
            let at = &signature_at;
            Ty::Function(Box::new(FunctionTy {
                params: list_at(signature, "params", at, read_type)?,
                returns: optional_at(signature, "result", at, read_type)?,
                fails: list_at(signature, "failures", at, read_type)?,
                needs: list_at(signature, "capabilities", at, read_grant)?,
            }))
        }
        "Param" => Ty::Param(usize_at(fields, "id", at)?),
        "Var" => Ty::Var(usize_at(fields, "id", at)?),
        "SelfTy" => Ty::SelfType,
        "Union" => Ty::Union(list_at(fields, "members", at, read_type)?),
        "Unit" => Ty::Unit,
        "Never" => Ty::Never,
        "ErrorTy" => Ty::Error,
        other => return Err(format!("{at}: unknown type `{other}`")),
    })
}

fn read_grant(json: &In, at: &str) -> Read<Capability> {
    let fields = object(json, at)?;
    let scope = |fields: &Fields, at: &str| {
        optional_at(fields, "scope", at, |json, at| {
            text_of(json, at).map(str::to_string)
        })
    };
    Ok(Capability {
        path: texts_at(fields, "path", at)?,
        scope: scope(fields, at)?,
        budget: optional_at(fields, "budget", at, |json, at| {
            let fields = object(json, at)?;
            Ok((text_at(fields, "limit", at)?, text_at(fields, "unit", at)?))
        })?,
        only_to: list_at(fields, "only_to", at, |json, at| {
            let fields = object(json, at)?;
            Ok((texts_at(fields, "path", at)?, scope(fields, at)?))
        })?,
    })
}

// ----------------------------------------------------------------- checking

/// Every index of the program points inside it, so that the VM never
/// reaches past a table on a file written by hand.
fn check(program: &Program) -> Read<()> {
    let codes = program.codes.len();
    let functions = program.function_metas.len();
    let types = program.types.metas.len();
    let within = |what: &str, index: usize, limit: usize| -> Read<()> {
        if index < limit {
            Ok(())
        } else {
            Err(format!(
                "the file is not consistent: {what} {index} is out of range (the limit is {limit})"
            ))
        }
    };
    for (id, code) in program.functions.iter() {
        within("the function", *id, functions)?;
        within("the code", *code, codes)?;
    }
    for constant in &program.constants {
        within("the constant's code", constant.code, codes)?;
    }
    for test in &program.tests {
        within("the test's code", test.code, codes)?;
    }
    for example in &program.examples {
        within("the example's function", example.function, functions)?;
        within("the example's code", example.call, codes)?;
        let expected = match example.expected {
            Expected::Is(code) | Expected::FailsWith(code) => code,
        };
        within("the example's expected code", expected, codes)?;
    }
    for meta in &program.types.metas {
        for refinement in &meta.refinements {
            within("the refinement's code", *refinement, codes)?;
        }
    }
    if let Some(main) = program.main {
        within("main", main, functions)?;
    }
    if let Some(date) = program.date {
        within("the Date type", date, types)?;
    }
    if program.specials.len() != types {
        return Err(format!(
            "the file is not consistent: {} specials for {types} types",
            program.specials.len()
        ));
    }
    for (index, code) in program.codes.iter().enumerate() {
        let name = format!("code {index}");
        if code.spans.len() != code.ops.len() {
            return Err(format!(
                "the file is not consistent: {name} has {} operations and {} spans",
                code.ops.len(),
                code.spans.len()
            ));
        }
        if let Some(function) = code.function {
            within(&format!("{name}'s function"), function, functions)?;
        }
        let ops = code.ops.len();
        let locals = code.locals as usize;
        let constants = code.constants.len();
        for op in &code.ops {
            match op {
                Op::Const(i) | Op::Check(i) => {
                    within(&format!("{name}'s constant"), *i as usize, constants)?
                }
                Op::Global(i) => within(
                    &format!("{name}'s global"),
                    *i as usize,
                    program.constants.len(),
                )?,
                Op::Load(slot)
                | Op::LoadMove(slot)
                | Op::Store(slot)
                | Op::IterInit(slot)
                | Op::Deadline(slot)
                | Op::CheckDeadline(slot)
                | Op::MarkStack(slot)
                | Op::UnwindStack(slot) => {
                    within(&format!("{name}'s slot"), *slot as usize, locals)?
                }
                Op::IterNext { slot, exit } => {
                    within(&format!("{name}'s slot"), *slot as usize, locals)?;
                    within(&format!("{name}'s jump"), *exit as usize, ops)?;
                }
                Op::Jump(target)
                | Op::JumpIfFalse(target)
                | Op::JumpIfTrue(target)
                | Op::JumpIfAbsent(target)
                | Op::JumpIfFailure(target)
                | Op::PushHandler(target) => {
                    within(&format!("{name}'s jump"), *target as usize, ops)?
                }
                Op::Construct { ty, .. } | Op::ConstructVariant { ty, .. } | Op::IsType(ty) => {
                    within(&format!("{name}'s type"), *ty, types)?
                }
                Op::Field {
                    name: constant,
                    site,
                } => {
                    within(&format!("{name}'s constant"), *constant as usize, constants)?;
                    within(
                        &format!("{name}'s field site"),
                        *site as usize,
                        program.field_sites,
                    )?;
                }
                Op::Call { function, .. } => {
                    within(&format!("{name}'s function"), *function, functions)?
                }
                Op::CallAbility { ability, .. } => within(
                    &format!("{name}'s ability"),
                    *ability,
                    program.abilities.len(),
                )?,
                Op::ResultType(i) => within(
                    &format!("{name}'s result type"),
                    *i as usize,
                    program.result_types.len(),
                )?,
                _ => {}
            }
        }
        for constant in &code.constants {
            if let Value::Function(function) = constant {
                within(&format!("{name}'s function constant"), *function, functions)?;
            }
        }
    }
    Ok(())
}
