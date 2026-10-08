//! The compiler from the checked syntax tree to bytecode: one `Code` per
//! function body, test, constant, `example:` line and refinement. Names are
//! resolved through the references the checker recorded (`Target`), so the
//! compiler never resolves a name itself; a construct it cannot compile
//! crashes at run time with a message rather than failing the whole program.
//! Every span the program keeps counts characters, as the bytecode file
//! does (decision Z1), so that a program loaded from a file is the program
//! compiled in process.

mod expr;
mod pattern;
mod query;
mod stmt;

use std::collections::HashMap;

use renyi_check::effects::Capability;
use renyi_check::types::Ty;
use renyi_check::world::{Builtins, TypeKindInfo, World};
use renyi_check::{BodyLocation, CheckedProject, FunctionId, ModuleId, NumberKind, Target, TypeId};
use renyi_syntax::ast::{self, Block, Expr, Item, TypeKind};
use renyi_syntax::{SourceFile, Span};

use crate::bytecode::{Code, CodeKind, Op};
use crate::types::{TypeShape, Types};
use crate::value::Value;

pub type CodeId = usize;

/// An `example:` line of a function.
#[derive(Clone, Debug)]
pub struct ExampleMeta {
    pub function: FunctionId,
    pub module: ModuleId,
    /// The example's source text on one line, for reports.
    pub text: String,
    pub span: Span,
    /// Evaluates the call; its value may be a `Failure`.
    pub call: CodeId,
    pub expected: Expected,
}

#[derive(Clone, Debug)]
pub enum Expected {
    /// Evaluates the expected value.
    Is(CodeId),
    /// Takes the error as its one parameter and returns whether the pattern
    /// matches.
    FailsWith(CodeId),
}

#[derive(Clone, Debug)]
pub struct TestMeta {
    pub module: ModuleId,
    pub name: String,
    pub needs: Vec<Capability>,
    pub replays: Option<String>,
    pub span: Span,
    pub code: CodeId,
}

#[derive(Clone, Debug)]
pub struct ConstantMeta {
    pub module: ModuleId,
    pub name: String,
    pub code: CodeId,
}

/// What the VM needs to know about a declared function at run time.
#[derive(Clone, Debug)]
pub struct FunctionMeta {
    pub module: String,
    pub name: String,
    pub is_library: bool,
    pub is_method: bool,
    pub params: Vec<String>,
    /// The type of the first parameter as the checker spells it: `Text`,
    /// `List of Item`, `Map of Key to Value`.
    pub receiver: Option<String>,
    /// Every parameter's type, spelled the same way; the effect of a call
    /// finds its `Path` or `Url` argument by it.
    pub param_types: Vec<String>,
    /// The declared result type; a recorded result is decoded by it.
    pub returns: Option<Ty>,
    /// The declared error types; a recorded failure is decoded by them.
    pub fails: Vec<Ty>,
    pub needs: Vec<Capability>,
    /// The `purpose:` clause, for narrated runs.
    pub purpose: Option<String>,
    /// A foreign function's binding (decision AF1).
    pub foreign: Option<Foreign>,
    /// A Python function's binding (decision AL1).
    pub python: Option<Python>,
}

/// A foreign function's binding (decision AF1): the libraries its symbol
/// is looked up in, tried in order, the symbol, and the C signature as
/// `renyi_check::foreign` spells it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Foreign {
    pub libraries: Vec<String>,
    pub symbol: String,
    pub parameters: Vec<String>,
    pub result: String,
}

/// A Python function's binding (decision AL1): the name the worker imports
/// the module by, the function's name on the Python side, the interpreter
/// the manifest names, if any, and the project root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Python {
    pub package: String,
    pub symbol: String,
    pub interpreter: Option<String>,
    pub root: String,
}

/// The binding of a function of a Python module (decision AL1): the
/// symbol the manifest renames or the function's own name.
fn python_binding(
    info: &renyi_check::world::FunctionInfo,
    module: &renyi_check::world::ModuleInfo,
) -> Option<Python> {
    let binding = module.python.as_ref()?;
    let symbol = binding
        .module
        .symbols
        .iter()
        .find(|(renyi, _)| *renyi == info.name)
        .map(|(_, symbol)| symbol.clone())
        .unwrap_or_else(|| info.name.clone());
    Some(Python {
        package: binding.module.package.clone(),
        symbol,
        interpreter: binding.interpreter.clone(),
        root: binding.root.clone(),
    })
}

/// The binding of a function of a foreign module (decision AF1): the
/// symbol the manifest renames or the function's own name, and the C
/// signature the checker accepted.
fn foreign_binding(
    world: &World,
    info: &renyi_check::world::FunctionInfo,
    module: &renyi_check::world::ModuleInfo,
) -> Option<Foreign> {
    let binding = module.foreign.as_ref()?;
    let symbol = binding
        .symbols
        .iter()
        .find(|(renyi, _)| *renyi == info.name)
        .map(|(_, symbol)| symbol.clone())
        .unwrap_or_else(|| info.name.clone());
    Some(Foreign {
        libraries: binding.libraries.clone(),
        symbol,
        parameters: info
            .params
            .iter()
            .map(|(_, ty)| {
                world
                    .c_type_of(ty)
                    .map(|c_type| c_type.spelling().to_string())
                    .unwrap_or_default()
            })
            .collect(),
        result: world
            .c_result_of(info.returns.as_ref())
            .map(|result| result.spelling().to_string())
            .unwrap_or_default(),
    })
}

/// The compiled project.
pub struct Program {
    pub codes: Vec<Code>,
    pub functions: HashMap<FunctionId, CodeId>,
    /// `functions` as a table over every function, `None` for a library
    /// primitive: the lookup of every call (decision X3).
    pub function_codes: Vec<Option<CodeId>>,
    pub function_metas: Vec<FunctionMeta>,
    /// In evaluation order; `Op::Global(i)` reads the i-th.
    pub constants: Vec<ConstantMeta>,
    pub tests: Vec<TestMeta>,
    pub examples: Vec<ExampleMeta>,
    pub types: Types,
    pub builtins: Builtins,
    /// `Op::ResultType(i)` names the i-th.
    pub result_types: Vec<Ty>,
    /// Per module, the path of its source file and where its lines start,
    /// `None` for a library module; the location of a crash or a test
    /// comes from it.
    pub sources: Vec<Option<SourceLines>>,
    pub module_names: Vec<String>,
    /// Methods by the head type of their receiver and their name.
    pub method_index: HashMap<(TypeId, String), Vec<FunctionId>>,
    /// Every ability's name and the names of its methods, by `AbilityId`.
    pub abilities: Vec<(String, Vec<String>)>,
    /// The `main` function of the first file given, when it has one.
    pub main: Option<FunctionId>,
    /// How many `Op::Field` sites the program has: the size of the VM's
    /// cache of field indices (decision X3).
    pub field_sites: usize,
    /// Per type, the methods the VM looks up on every comparison, rendering
    /// and loop, so that no name is looked up at run time (decision X3).
    pub specials: Vec<Specials>,
    /// `std.time.Date`, whose construction also checks the day against the
    /// month (library sketch, section 4).
    pub date: Option<TypeId>,
}

/// The declared `equals`, `compare`, `to_text` and `to_list` of a type,
/// each when exactly one is declared for it (decision K1: a declared
/// method replaces the derived behaviour).
#[derive(Clone, Copy, Debug, Default)]
pub struct Specials {
    pub equals: Option<FunctionId>,
    pub compare: Option<FunctionId>,
    pub to_text: Option<FunctionId>,
    pub to_list: Option<FunctionId>,
}

/// A source file as the program remembers it: its path and the character
/// offset at which each line starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceLines {
    pub name: String,
    pub line_starts: Vec<usize>,
}

impl SourceLines {
    pub fn new(name: &str, text: &str) -> SourceLines {
        let mut line_starts = vec![0];
        for (offset, c) in text.chars().enumerate() {
            if c == '\n' {
                line_starts.push(offset + 1);
            }
        }
        SourceLines {
            name: name.to_string(),
            line_starts,
        }
    }

    /// The line (from 1) holding a character offset.
    pub fn line_of(&self, offset: usize) -> usize {
        self.line_starts.partition_point(|start| *start <= offset)
    }
}

impl Program {
    pub fn code(&self, id: CodeId) -> &Code {
        &self.codes[id]
    }

    /// The source of a module, when it has one.
    pub fn source(&self, module: ModuleId) -> Option<&SourceLines> {
        self.sources.get(module).and_then(|source| source.as_ref())
    }

    /// Where a span of a module lies, as `file:line`.
    pub fn location(&self, module: ModuleId, span: Span) -> String {
        match self.source(module) {
            Some(source) => format!("{}:{}", source.name, source.line_of(span.start)),
            None => self.module_names[module].clone(),
        }
    }
}

/// The references of one body, by span; two references may share a span (a
/// call and its context-decided result type).
type Refs = HashMap<Span, Vec<Target>>;

/// What every body's compiler shares.
struct Context<'w> {
    world: &'w World,
    globals: HashMap<(ModuleId, String), u32>,
    sources: HashMap<ModuleId, String>,
    /// Per module with a source, the character offset of every byte offset
    /// of its text (one more entry for the end), so that the spans the
    /// program keeps count characters.
    characters: HashMap<ModuleId, Vec<usize>>,
    result_types: Vec<Ty>,
    refs: HashMap<(ModuleId, BodyLocation), Refs>,
    /// The `Op::Field` sites numbered so far.
    field_sites: u32,
}

impl Context<'_> {
    fn take_refs(&mut self, module: ModuleId, body: BodyLocation) -> Refs {
        self.refs.remove(&(module, body)).unwrap_or_default()
    }

    fn clone_refs(&self, module: ModuleId, body: BodyLocation) -> Refs {
        self.refs.get(&(module, body)).cloned().unwrap_or_default()
    }

    fn text_of(&self, module: ModuleId, span: Span) -> Option<String> {
        self.sources
            .get(&module)
            .and_then(|text| text.get(span.start..span.end))
            .map(|text| text.split_whitespace().collect::<Vec<_>>().join(" "))
    }

    /// A span of the syntax tree, in bytes, as the program keeps it, in
    /// characters; a module without a source (the library) is ASCII.
    fn characters(&self, module: ModuleId, span: Span) -> Span {
        match self.characters.get(&module) {
            Some(table) => {
                let last = table.len() - 1;
                Span::new(table[span.start.min(last)], table[span.end.min(last)])
            }
            None => span,
        }
    }
}

/// The character offset of every byte offset of a text, and of its end.
fn character_table(text: &str) -> Vec<usize> {
    let mut characters = vec![0; text.len() + 1];
    let mut count = 0;
    for (offset, c) in text.char_indices() {
        characters[offset..offset + c.len_utf8()].fill(count);
        count += 1;
    }
    characters[text.len()] = count;
    characters
}

/// Compile every module of a checked project; `files` are the sources the
/// project was checked from, in the same order.
pub fn compile_project(checked: &CheckedProject, files: &[SourceFile]) -> Program {
    let world = &checked.world;
    let mut refs: HashMap<(ModuleId, BodyLocation), Refs> = HashMap::new();
    for (module, body, reference) in &checked.references {
        refs.entry((*module, *body))
            .or_default()
            .entry(reference.span)
            .or_default()
            .push(reference.target.clone());
    }
    let mut sources = vec![None; world.modules.len()];
    let mut source_texts = HashMap::new();
    let mut characters = HashMap::new();
    for module in &checked.modules {
        if let (Some(id), Some(file)) = (module.id, files.get(module.file)) {
            sources[id] = Some(SourceLines::new(&file.name, &file.text));
            source_texts.insert(id, file.text.clone());
            characters.insert(id, character_table(&file.text));
        }
    }
    let main_module = checked.modules.first().and_then(|m| m.id);
    let mut program = Program {
        codes: Vec::new(),
        functions: HashMap::new(),
        function_codes: Vec::new(),
        function_metas: Vec::new(),
        constants: Vec::new(),
        tests: Vec::new(),
        examples: Vec::new(),
        types: Types::from_world(world),
        builtins: world.builtins.clone(),
        result_types: Vec::new(),
        sources,
        module_names: world.modules.iter().map(|m| m.name.clone()).collect(),
        method_index: world.method_index.clone(),
        abilities: world
            .abilities
            .iter()
            .map(|ability| {
                (
                    ability.name.clone(),
                    ability.methods.iter().map(|m| m.name.clone()).collect(),
                )
            })
            .collect(),
        main: main_module.and_then(|m| world.lookup_function(m, "main")),
        field_sites: 0,
        specials: Vec::new(),
        date: None,
    };
    let single = |ty: TypeId, name: &str| -> Option<FunctionId> {
        match world.method_index.get(&(ty, name.to_string()))?.as_slice() {
            [id] => Some(*id),
            _ => None,
        }
    };
    program.specials = (0..program.types.metas.len())
        .map(|ty| Specials {
            equals: single(ty, "equals"),
            compare: single(ty, "compare"),
            to_text: single(ty, "to_text"),
            to_list: single(ty, "to_list"),
        })
        .collect();
    program.date = program.types.find("std.time", "Date");
    for info in &world.functions {
        let module = &world.modules[info.module];
        let purpose = match info.body {
            BodyLocation::Item(item) => match module.ast.items.get(item) {
                Some(Item::Function(function)) => function.docs.purpose.clone(),
                _ => None,
            },
            BodyLocation::Implementation(item, index) => match module.ast.items.get(item) {
                Some(Item::Implementation(implementation)) => implementation
                    .functions
                    .get(index)
                    .and_then(|function| function.docs.purpose.clone()),
                _ => None,
            },
            _ => None,
        };
        program.function_metas.push(FunctionMeta {
            module: module.name.clone(),
            name: info.name.clone(),
            is_library: info.is_library,
            is_method: info.is_method,
            params: info.params.iter().map(|(n, _)| n.clone()).collect(),
            receiver: info.params.first().map(|(_, ty)| world.show(ty)),
            param_types: info.params.iter().map(|(_, ty)| world.show(ty)).collect(),
            returns: info.returns.clone(),
            fails: info.fails.clone(),
            needs: info.needs.clone(),
            purpose,
            foreign: foreign_binding(world, info, module),
            python: python_binding(info, module),
        });
    }
    // constants by name per module, so that bodies can refer to them
    let mut globals: HashMap<(ModuleId, String), u32> = HashMap::new();
    for (module_id, module) in world.modules.iter().enumerate() {
        for item in &module.ast.items {
            if let Item::Constant(constant) = item {
                let index = program.constants.len() as u32;
                globals.insert((module_id, constant.name.text.clone()), index);
                program.constants.push(ConstantMeta {
                    module: module_id,
                    name: constant.name.text.clone(),
                    code: usize::MAX,
                });
            }
        }
    }
    let mut ctx = Context {
        world,
        globals,
        sources: source_texts,
        characters,
        result_types: Vec::new(),
        refs,
        field_sites: 0,
    };
    for (module_id, module) in world.modules.iter().enumerate() {
        if module.is_library {
            continue;
        }
        for (item_index, item) in module.ast.items.iter().enumerate() {
            match item {
                Item::Function(function) => {
                    let body = BodyLocation::Item(item_index);
                    if let Some(id) = function_at(world, module_id, body) {
                        compile_function(&mut program, &mut ctx, module_id, id, function, body);
                    }
                }
                Item::Implementation(implementation) => {
                    for (function_index, function) in implementation.functions.iter().enumerate() {
                        let body = BodyLocation::Implementation(item_index, function_index);
                        if let Some(id) = function_at(world, module_id, body) {
                            compile_function(&mut program, &mut ctx, module_id, id, function, body);
                        }
                    }
                }
                Item::Constant(constant) => {
                    let refs = ctx.take_refs(module_id, BodyLocation::Item(item_index));
                    let code = Code::new(constant.name.text.clone(), module_id, CodeKind::Constant);
                    let mut compiler = Compiler::new(&mut ctx, module_id, refs, code);
                    compiler.expr(&constant.value);
                    compiler.emit(Op::Return, constant.span);
                    let code = compiler.finish();
                    let index = ctx.globals[&(module_id, constant.name.text.clone())] as usize;
                    program.constants[index].code = program.codes.len();
                    program.codes.push(code);
                }
                Item::Test(test) => {
                    let refs = ctx.take_refs(module_id, BodyLocation::Item(item_index));
                    let code =
                        Code::new(format!("test {:?}", test.name), module_id, CodeKind::Test);
                    let mut compiler = Compiler::new(&mut ctx, module_id, refs, code);
                    compiler.in_test = true;
                    compiler.block(&test.body);
                    compiler.emit(Op::ReturnNothing, test.span);
                    let code = compiler.finish();
                    program.tests.push(TestMeta {
                        module: module_id,
                        name: test.name.clone(),
                        needs: test.needs.iter().map(Capability::from_ast).collect(),
                        replays: test.replays.clone(),
                        span: ctx.characters(module_id, test.span),
                        code: program.codes.len(),
                    });
                    program.codes.push(code);
                }
                Item::Type(def) => {
                    compile_refinements(&mut program, &mut ctx, module_id, item_index, def)
                }
                Item::Ability(_) => {}
            }
        }
    }
    // the library's own refined types (`Port`, `Date`) guard construction too
    for (module_id, module) in world.modules.iter().enumerate() {
        if !module.is_library {
            continue;
        }
        for (item_index, item) in module.ast.items.iter().enumerate() {
            if let Item::Type(def) = item {
                compile_refinements(&mut program, &mut ctx, module_id, item_index, def);
            }
        }
    }
    program.result_types = ctx.result_types;
    program.field_sites = ctx.field_sites as usize;
    program.function_codes = (0..program.function_metas.len())
        .map(|id| program.functions.get(&id).copied())
        .collect();
    program
}

fn function_at(world: &World, module: ModuleId, body: BodyLocation) -> Option<FunctionId> {
    world
        .functions
        .iter()
        .position(|f| f.module == module && f.body == body)
}

fn compile_function(
    program: &mut Program,
    ctx: &mut Context<'_>,
    module: ModuleId,
    id: FunctionId,
    function: &ast::Function,
    body: BodyLocation,
) {
    let Some(block) = &function.body else {
        return;
    };
    let refs = ctx.take_refs(module, body);
    let mut code = Code::new(function.name.text.clone(), module, CodeKind::Function);
    code.function = Some(id);
    let mut compiler = Compiler::new(ctx, module, refs, code);
    for param in &function.params {
        compiler.declare(&param.name.text);
    }
    compiler.code.params = function.params.len() as u16;
    compiler.block(block);
    compiler.emit(Op::ReturnNothing, function.span);
    let code = compiler.finish();
    program.functions.insert(id, program.codes.len());
    program.codes.push(code);
    let (item, method) = match body {
        BodyLocation::Item(item) => (item, None),
        BodyLocation::Implementation(item, method) => (item, Some(method)),
        _ => return,
    };
    for (index, example) in function.docs.examples.iter().enumerate() {
        let body = BodyLocation::Example {
            item,
            method,
            example: index,
        };
        let name = format!("example of {}", function.name.text);
        let refs = ctx.clone_refs(module, body);
        let code = Code::new(name.clone(), module, CodeKind::Example);
        let mut compiler = Compiler::new(ctx, module, refs, code);
        compiler.expr(&example.expression);
        compiler.emit(Op::Return, example.span);
        let call = program.codes.len();
        program.codes.push(compiler.finish());
        let expected = match &example.outcome {
            ast::ExampleOutcome::Is(value) => {
                let refs = ctx.clone_refs(module, body);
                let code = Code::new(name.clone(), module, CodeKind::Example);
                let mut compiler = Compiler::new(ctx, module, refs, code);
                compiler.expr(value);
                compiler.emit(Op::Return, value.span);
                let id = program.codes.len();
                program.codes.push(compiler.finish());
                Expected::Is(id)
            }
            ast::ExampleOutcome::FailsWith(pattern) => {
                let refs = ctx.clone_refs(module, body);
                let code = Code::new(name.clone(), module, CodeKind::Refinement);
                let mut compiler = Compiler::new(ctx, module, refs, code);
                let error = compiler.declare("error");
                compiler.code.params = 1;
                let span = pattern.span();
                let next = compiler.pattern(pattern, error);
                compiler.constant(Value::Boolean(true), span);
                compiler.emit(Op::Return, span);
                for at in next {
                    compiler.patch(at);
                }
                compiler.constant(Value::Boolean(false), span);
                compiler.emit(Op::Return, span);
                let id = program.codes.len();
                program.codes.push(compiler.finish());
                Expected::FailsWith(id)
            }
        };
        let text = ctx
            .text_of(module, example.span)
            .unwrap_or_else(|| name.clone());
        program.examples.push(ExampleMeta {
            function: id,
            module,
            text,
            span: ctx.characters(module, example.span),
            call,
            expected,
        });
    }
}

/// The refinement predicates of a type: a subtype's `where value ...` and the
/// field conditions of a record or of a variant, each as a code over the
/// fields by name.
fn compile_refinements(
    program: &mut Program,
    ctx: &mut Context<'_>,
    module: ModuleId,
    item_index: usize,
    def: &ast::TypeDef,
) {
    let Some(ty) = ctx.world.modules[module].types.get(&def.name.text).copied() else {
        return;
    };
    match &def.kind {
        TypeKind::Subtype {
            refinement: Some(condition),
            ..
        } => {
            let owner = BodyLocation::Condition {
                item: item_index,
                variant: None,
                field: None,
            };
            let code = compile_predicate(ctx, module, owner, &["value"], condition, ty);
            let id = program.codes.len();
            program.codes.push(code);
            program.types.metas[ty].refinements.push(id);
        }
        TypeKind::Record { fields, .. } => {
            let names: Vec<&str> = fields.iter().map(|f| f.name.text.as_str()).collect();
            for (index, field) in fields.iter().enumerate() {
                let Some(condition) = &field.refinement else {
                    continue;
                };
                let owner = BodyLocation::Condition {
                    item: item_index,
                    variant: None,
                    field: Some(index),
                };
                let code = compile_predicate(ctx, module, owner, &names, condition, ty);
                let id = program.codes.len();
                program.codes.push(code);
                let meta = &mut program.types.metas[ty];
                meta.refinements.push(id);
                if let TypeShape::Record(metas) = &mut meta.shape {
                    metas[index].refinement = Some(meta.refinements.len() - 1);
                }
            }
        }
        TypeKind::Sum { variants, .. } => {
            for (tag, variant) in variants.iter().enumerate() {
                let names: Vec<&str> = variant
                    .fields
                    .iter()
                    .map(|f| f.name.text.as_str())
                    .collect();
                for (index, field) in variant.fields.iter().enumerate() {
                    let Some(condition) = &field.refinement else {
                        continue;
                    };
                    let owner = BodyLocation::Condition {
                        item: item_index,
                        variant: Some(tag),
                        field: Some(index),
                    };
                    let code = compile_predicate(ctx, module, owner, &names, condition, ty);
                    let id = program.codes.len();
                    program.codes.push(code);
                    let meta = &mut program.types.metas[ty];
                    meta.refinements.push(id);
                    if let TypeShape::Sum(metas) = &mut meta.shape {
                        metas[tag].fields[index].refinement = Some(meta.refinements.len() - 1);
                    }
                }
            }
        }
        _ => {}
    }
}

fn compile_predicate(
    ctx: &mut Context<'_>,
    module: ModuleId,
    owner: BodyLocation,
    params: &[&str],
    condition: &Expr,
    ty: TypeId,
) -> Code {
    let name = format!("refinement of {}", ctx.world.types[ty].name);
    let text = ctx
        .text_of(module, condition.span)
        .unwrap_or_else(|| "its condition".to_string());
    let code = Code::new(name, module, CodeKind::Refinement);
    let refs = ctx.take_refs(module, owner);
    let mut compiler = Compiler::new(ctx, module, refs, code);
    for param in params {
        compiler.declare(param);
    }
    compiler.code.params = params.len() as u16;
    // the condition's text is constant 0: the detail of a violation
    compiler.code.constant(Value::text(text));
    compiler.expr(condition);
    compiler.emit(Op::Return, condition.span);
    compiler.finish()
}

/// Where `break` and `continue` go inside a loop, how many handled regions
/// were open when the loop began, and the slot holding the operand stack's
/// height at its entry (decision Y4).
struct LoopContext {
    breaks: Vec<usize>,
    continues: Vec<usize>,
    handler_depth: usize,
    mark: u16,
}

pub(crate) struct Compiler<'c, 'w> {
    pub world: &'w World,
    ctx: &'c mut Context<'w>,
    pub module: ModuleId,
    refs: Refs,
    pub code: Code,
    scopes: Vec<Vec<(String, u16)>>,
    next_slot: u16,
    loops: Vec<LoopContext>,
    handler_depth: usize,
    pub in_test: bool,
    /// `change x to x.method(...)`: the receiver's name token is loaded with
    /// `LoadMove`, so the collection is updated in place.
    move_receiver: Option<Span>,
}

impl<'c, 'w> Compiler<'c, 'w> {
    fn new(ctx: &'c mut Context<'w>, module: ModuleId, refs: Refs, code: Code) -> Compiler<'c, 'w> {
        Compiler {
            world: ctx.world,
            ctx,
            module,
            refs,
            code,
            scopes: vec![Vec::new()],
            next_slot: 0,
            loops: Vec::new(),
            handler_depth: 0,
            in_test: false,
            move_receiver: None,
        }
    }

    fn finish(mut self) -> Code {
        self.code.locals = self.next_slot;
        self.code
    }

    // ------------------------------------------------------------ scopes

    /// A new local in the current scope.
    pub fn declare(&mut self, name: &str) -> u16 {
        let slot = self.temp();
        self.scopes
            .last_mut()
            .expect("a scope")
            .push((name.to_string(), slot));
        slot
    }

    /// A fresh unnamed local.
    pub fn temp(&mut self) -> u16 {
        let slot = self.next_slot;
        self.next_slot += 1;
        slot
    }

    /// Give an existing slot a name in the current scope (a pattern binding).
    pub fn alias(&mut self, name: &str, slot: u16) {
        self.scopes
            .last_mut()
            .expect("a scope")
            .push((name.to_string(), slot));
    }

    pub fn lookup(&self, name: &str) -> Option<u16> {
        for scope in self.scopes.iter().rev() {
            if let Some((_, slot)) = scope.iter().rev().find(|(n, _)| n == name) {
                return Some(*slot);
            }
        }
        None
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(Vec::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    // ------------------------------------------------------------ references

    pub fn targets(&self, span: Span) -> &[Target] {
        self.refs.get(&span).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn global(&self, module: ModuleId, name: &str) -> Option<u32> {
        self.ctx.globals.get(&(module, name.to_string())).copied()
    }

    /// The number type the checker gave a literal or a `sum`.
    pub fn number_at(&self, span: Span) -> Option<NumberKind> {
        self.targets(span).iter().find_map(|t| match t {
            Target::Number(kind) => Some(*kind),
            _ => None,
        })
    }

    /// The index in the program's `result_types` of the context type the
    /// checker recorded for a call, when it did.
    pub fn result_type_at(&mut self, span: Span) -> Option<u32> {
        let ty = self.targets(span).iter().find_map(|t| match t {
            Target::Result(ty) => Some(ty.clone()),
            _ => None,
        })?;
        if let Some(index) = self.ctx.result_types.iter().position(|t| *t == ty) {
            return Some(index as u32);
        }
        self.ctx.result_types.push(ty);
        Some((self.ctx.result_types.len() - 1) as u32)
    }

    pub fn source_text(&self, span: Span) -> String {
        self.ctx.text_of(self.module, span).unwrap_or_default()
    }

    // ------------------------------------------------------------ emission

    /// Emit an operation under a span of the tree (in bytes); the code
    /// keeps the span in characters.
    pub fn emit(&mut self, op: Op, span: Span) -> usize {
        let span = self.ctx.characters(self.module, span);
        self.code.emit(op, span)
    }

    pub fn constant(&mut self, value: Value, span: Span) {
        let index = self.code.constant(value);
        self.emit(Op::Const(index), span);
    }

    pub fn name_constant(&mut self, name: &str) -> u32 {
        self.code.constant(Value::text(name))
    }

    /// The number of the next `Op::Field` in the program (decision X3).
    pub fn field_site(&mut self) -> u32 {
        let site = self.ctx.field_sites;
        self.ctx.field_sites += 1;
        site
    }

    pub fn here(&self) -> u32 {
        self.code.here()
    }

    pub fn patch(&mut self, at: usize) {
        self.code.patch(at)
    }

    /// A crash for a construct the compiler does not know how to run.
    pub fn unsupported(&mut self, what: &str, span: Span) {
        self.constant(
            Value::text(format!("{what} is not supported by this build of the VM")),
            span,
        );
        self.emit(Op::Crash, span);
    }

    // ------------------------------------------------------------ handlers

    /// Open a handled region: returns the index of the `PushHandler` to patch.
    pub fn push_handler(&mut self, span: Span) -> usize {
        self.handler_depth += 1;
        self.emit(Op::PushHandler(0), span)
    }

    pub fn pop_handler(&mut self, span: Span) {
        self.handler_depth -= 1;
        self.emit(Op::PopHandler, span);
    }

    // ------------------------------------------------------------ loops

    /// Open a loop: its entry marks the operand stack's height, which
    /// `break` and `continue` restore (decision Y4). Called before the
    /// position a `continue` returns to.
    pub fn enter_loop(&mut self, span: Span) {
        let mark = self.temp();
        self.emit(Op::MarkStack(mark), span);
        self.loops.push(LoopContext {
            breaks: Vec::new(),
            continues: Vec::new(),
            handler_depth: self.handler_depth,
            mark,
        });
    }

    /// Close the loop: `break`s jump here, `continue`s to `continue_target`.
    pub fn leave_loop(&mut self, continue_target: u32) {
        let context = self.loops.pop().expect("a loop");
        for at in context.breaks {
            self.code.patch(at);
        }
        for at in context.continues {
            self.code.patch_to(at, continue_target);
        }
    }

    /// Leave every handled region opened inside the innermost loop and drop
    /// the operands of the expression being interrupted, then jump.
    fn leave_regions_of_loop(&mut self, span: Span) {
        let Some(context) = self.loops.last() else {
            return;
        };
        let open = self.handler_depth - context.handler_depth;
        let mark = context.mark;
        for _ in 0..open {
            self.emit(Op::PopHandler, span);
        }
        self.emit(Op::UnwindStack(mark), span);
    }

    pub fn emit_break(&mut self, span: Span) {
        self.leave_regions_of_loop(span);
        let at = self.emit(Op::Jump(0), span);
        match self.loops.last_mut() {
            Some(context) => context.breaks.push(at),
            None => self.unsupported("`break` outside a loop", span),
        }
    }

    pub fn emit_continue(&mut self, span: Span) {
        self.leave_regions_of_loop(span);
        let at = self.emit(Op::Jump(0), span);
        match self.loops.last_mut() {
            Some(context) => context.continues.push(at),
            None => self.unsupported("`continue` outside a loop", span),
        }
    }

    // ------------------------------------------------------------ types

    /// The fields of a record type in declaration order.
    pub fn record_fields(&self, ty: TypeId) -> Vec<String> {
        match &self.world.types[ty].kind {
            TypeKindInfo::Record(fields) => fields.iter().map(|f| f.name.clone()).collect(),
            _ => Vec::new(),
        }
    }

    pub fn variant_fields(&self, ty: TypeId, tag: usize) -> Vec<String> {
        match &self.world.types[ty].kind {
            TypeKindInfo::Sum(variants) => variants
                .get(tag)
                .map(|v| v.fields.iter().map(|f| f.name.clone()).collect())
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    /// The parameter names of a function, `self` included.
    pub fn param_names(&self, id: FunctionId) -> Vec<String> {
        self.world.functions[id]
            .params
            .iter()
            .map(|(name, _)| name.clone())
            .collect()
    }

    pub fn is_method(&self, id: FunctionId) -> bool {
        self.world.functions[id].is_method
    }

    pub fn lookup_type(&self, name: &str) -> Option<TypeId> {
        self.world.lookup_type(self.module, name)
    }

    // ------------------------------------------------------------ blocks

    pub fn block(&mut self, block: &Block) {
        self.push_scope();
        for statement in &block.statements {
            self.statement(statement);
        }
        self.pop_scope();
    }

    /// The statements of a block in the current scope (`run concurrently`
    /// leaves its bindings visible after `end`).
    pub fn statements_inline(&mut self, block: &Block) {
        for statement in &block.statements {
            self.statement(statement);
        }
    }
}
