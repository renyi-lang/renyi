//! The compiler from the checked syntax tree to bytecode: one `Code` per
//! function body, test, constant, `example:` line and refinement. Names are
//! resolved through the references the checker recorded (`Target`), so the
//! compiler never resolves a name itself; a construct it cannot compile
//! crashes at run time with a message rather than failing the whole program.

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
}

/// The compiled project.
pub struct Program {
    pub codes: Vec<Code>,
    pub functions: HashMap<FunctionId, CodeId>,
    pub function_metas: Vec<FunctionMeta>,
    /// In evaluation order; `Op::Global(i)` reads the i-th.
    pub constants: Vec<ConstantMeta>,
    pub tests: Vec<TestMeta>,
    pub examples: Vec<ExampleMeta>,
    pub types: Types,
    pub builtins: Builtins,
    /// `Op::ResultType(i)` names the i-th.
    pub result_types: Vec<Ty>,
    pub sources: HashMap<ModuleId, SourceFile>,
    pub module_names: Vec<String>,
    /// Methods by the head type of their receiver and their name.
    pub method_index: HashMap<(TypeId, String), Vec<FunctionId>>,
    /// Every ability's name and the names of its methods, by `AbilityId`.
    pub abilities: Vec<(String, Vec<String>)>,
    /// The `main` function of the first file given, when it has one.
    pub main: Option<FunctionId>,
}

impl Program {
    pub fn code(&self, id: CodeId) -> &Code {
        &self.codes[id]
    }

    /// A method of a type by name, when exactly one is declared for it.
    pub fn method(&self, ty: TypeId, name: &str) -> Option<FunctionId> {
        let ids = self.method_index.get(&(ty, name.to_string()))?;
        match ids.as_slice() {
            [id] => Some(*id),
            _ => None,
        }
    }

    /// Where a span of a module lies, as `file:line`.
    pub fn location(&self, module: ModuleId, span: Span) -> String {
        match self.sources.get(&module) {
            Some(file) => {
                let position = file.position(span.start);
                format!("{}:{}", file.name, position.line)
            }
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
    result_types: Vec<Ty>,
    refs: HashMap<(ModuleId, BodyLocation), Refs>,
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
    let mut sources = HashMap::new();
    let mut source_texts = HashMap::new();
    for module in &checked.modules {
        if let (Some(id), Some(file)) = (module.id, files.get(module.file)) {
            sources.insert(id, file.clone());
            source_texts.insert(id, file.text.clone());
        }
    }
    let main_module = checked.modules.first().and_then(|m| m.id);
    let mut program = Program {
        codes: Vec::new(),
        functions: HashMap::new(),
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
    };
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
        result_types: Vec::new(),
        refs,
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
                        span: test.span,
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
            span: example.span,
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

/// Where `break` and `continue` go inside a loop, and how many handled
/// regions were open when the loop began.
struct LoopContext {
    breaks: Vec<usize>,
    continues: Vec<usize>,
    handler_depth: usize,
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
    /// `set x to x.method(...)`: the receiver's name token is loaded with
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

    pub fn emit(&mut self, op: Op, span: Span) -> usize {
        self.code.emit(op, span)
    }

    pub fn constant(&mut self, value: Value, span: Span) {
        let index = self.code.constant(value);
        self.code.emit(Op::Const(index), span);
    }

    pub fn name_constant(&mut self, name: &str) -> u32 {
        self.code.constant(Value::text(name))
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

    pub fn enter_loop(&mut self) {
        self.loops.push(LoopContext {
            breaks: Vec::new(),
            continues: Vec::new(),
            handler_depth: self.handler_depth,
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

    /// Leave every handled region opened inside the innermost loop, then jump.
    fn leave_regions_of_loop(&mut self, span: Span) {
        let open = self
            .loops
            .last()
            .map(|l| self.handler_depth - l.handler_depth)
            .unwrap_or(0);
        for _ in 0..open {
            self.emit(Op::PopHandler, span);
        }
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
