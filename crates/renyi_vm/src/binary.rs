//! The binary encoding of a program (decision AT3): what the bytecode
//! file of `file.rs` holds, written as bytes that an image carries and
//! its loader reads without parsing text. The `.ryc` stays the JSON of
//! decisions Z1 to Z3, which people read and the compiler written in
//! Renyi writes too; this encoding is the image's alone, written by
//! `renyi build` and read by the VM that loads the image, which the
//! image's header already holds to one `renyi`. An integer is LEB128, a
//! text its length then its bytes, an option one byte then the value, a
//! list its count then its items, a variant one byte then its fields, in
//! the order `file.rs` writes them; a map is written in key order, so
//! that one program has one encoding.

use std::collections::HashMap;

use renyi_check::effects::Capability;
use renyi_check::types::{FunctionTy, Ty};
use renyi_check::world::Builtins;
use renyi_syntax::ast::BinaryOp;
use renyi_syntax::Span;

use crate::bytecode::{Code, CodeKind, GroupFold, Op};
use crate::compile::{
    CodeId, ConstantMeta, ExampleMeta, Expected, Foreign, FunctionMeta, Program, Python,
    SourceLines, Specials, TestMeta,
};
use crate::decimal::Decimal;
use crate::integer::Int;
use crate::types::{FieldMeta, TypeMeta, TypeShape, Types, VariantMeta};
use crate::value::Value;

/// The version of the encoding, its first integer.
pub const FORMAT: u64 = 3;

type Read<T> = Result<T, String>;

/// The operators in the order of their codes.
const OPERATORS: [BinaryOp; 14] = [
    BinaryOp::Add,
    BinaryOp::Subtract,
    BinaryOp::Multiply,
    BinaryOp::Divide,
    BinaryOp::Remainder,
    BinaryOp::Power,
    BinaryOp::Is,
    BinaryOp::IsNot,
    BinaryOp::IsLessThan,
    BinaryOp::IsAtMost,
    BinaryOp::IsGreaterThan,
    BinaryOp::IsAtLeast,
    BinaryOp::And,
    BinaryOp::Or,
];

fn operator_code(op: BinaryOp) -> u8 {
    match op {
        BinaryOp::Add => 0,
        BinaryOp::Subtract => 1,
        BinaryOp::Multiply => 2,
        BinaryOp::Divide => 3,
        BinaryOp::Remainder => 4,
        BinaryOp::Power => 5,
        BinaryOp::Is => 6,
        BinaryOp::IsNot => 7,
        BinaryOp::IsLessThan => 8,
        BinaryOp::IsAtMost => 9,
        BinaryOp::IsGreaterThan => 10,
        BinaryOp::IsAtLeast => 11,
        BinaryOp::And => 12,
        BinaryOp::Or => 13,
    }
}

// ----------------------------------------------------------------- writing

#[derive(Default)]
struct Writer {
    out: Vec<u8>,
}

impl Writer {
    fn u8(&mut self, value: u8) {
        self.out.push(value);
    }

    fn uint(&mut self, mut value: u64) {
        loop {
            let byte = (value & 0x7f) as u8;
            value >>= 7;
            if value == 0 {
                self.out.push(byte);
                return;
            }
            self.out.push(byte | 0x80);
        }
    }

    fn usize(&mut self, value: usize) {
        self.uint(value as u64);
    }

    fn bool(&mut self, value: bool) {
        self.u8(value as u8);
    }

    fn text(&mut self, text: &str) {
        self.usize(text.len());
        self.out.extend_from_slice(text.as_bytes());
    }

    fn texts(&mut self, texts: &[String]) {
        self.list(texts, |out, text| out.text(text));
    }

    fn usizes(&mut self, values: &[usize]) {
        self.list(values, |out, value| out.usize(*value));
    }

    fn opt<T>(&mut self, value: Option<&T>, write: impl FnOnce(&mut Writer, &T)) {
        match value {
            None => self.u8(0),
            Some(value) => {
                self.u8(1);
                write(self, value);
            }
        }
    }

    fn list<T>(&mut self, items: &[T], write: impl Fn(&mut Writer, &T)) {
        self.usize(items.len());
        for item in items {
            write(self, item);
        }
    }

    fn span(&mut self, span: Span) {
        self.usize(span.start);
        self.usize(span.end);
    }
}

fn write_type(out: &mut Writer, ty: &Ty) {
    match ty {
        Ty::App(id, args) => {
            out.u8(0);
            out.usize(*id);
            out.list(args, write_type);
        }
        Ty::Maybe(inner) => {
            out.u8(1);
            write_type(out, inner);
        }
        Ty::Function(function) => {
            out.u8(2);
            out.list(&function.params, write_type);
            out.opt(function.returns.as_ref(), write_type);
            out.list(&function.fails, write_type);
            out.list(&function.needs, write_grant);
        }
        Ty::Param(id) => {
            out.u8(3);
            out.usize(*id);
        }
        Ty::Var(id) => {
            out.u8(4);
            out.usize(*id);
        }
        Ty::SelfType => out.u8(5),
        Ty::Union(members) => {
            out.u8(6);
            out.list(members, write_type);
        }
        Ty::Unit => out.u8(7),
        Ty::Never => out.u8(8),
        Ty::Error => out.u8(9),
    }
}

fn write_grant(out: &mut Writer, capability: &Capability) {
    out.texts(&capability.path);
    out.opt(capability.scope.as_ref(), |out, scope| out.text(scope));
    out.opt(capability.budget.as_ref(), |out, (limit, unit)| {
        out.text(limit);
        out.text(unit);
    });
    out.list(&capability.only_to, |out, (path, scope)| {
        out.texts(path);
        out.opt(scope.as_ref(), |out, scope| out.text(scope));
    });
}

fn write_builtins(out: &mut Writer, builtins: &Builtins) {
    for id in [
        builtins.integer,
        builtins.decimal,
        builtins.float,
        builtins.boolean,
        builtins.text,
        builtins.bytes,
        builtins.list,
        builtins.map,
        builtins.set,
        builtins.range,
        builtins.pair,
        builtins.duration,
        builtins.ordering,
        builtins.timed_out,
        builtins.constraint_violation,
        builtins.guarded,
        builtins.equal,
        builtins.compare,
        builtins.hash,
        builtins.to_text,
        builtins.iterable,
    ] {
        out.usize(id);
    }
}

fn write_field(out: &mut Writer, field: &FieldMeta) {
    out.text(&field.name);
    write_type(out, &field.ty);
    out.opt(field.external_name.as_ref(), |out, name| out.text(name));
    out.bool(field.optional);
    out.opt(field.refinement.as_ref(), |out, index| out.usize(*index));
}

fn write_type_meta(out: &mut Writer, meta: &TypeMeta) {
    out.usize(meta.id);
    out.text(&meta.name);
    out.text(&meta.module);
    out.bool(meta.is_library);
    out.usizes(&meta.params);
    match &meta.shape {
        TypeShape::Opaque => out.u8(0),
        TypeShape::Record(fields) => {
            out.u8(1);
            out.list(fields, write_field);
        }
        TypeShape::Sum(variants) => {
            out.u8(2);
            out.list(variants, |out, variant| {
                out.text(&variant.name);
                out.list(&variant.fields, write_field);
            });
        }
        TypeShape::Subtype { base } => {
            out.u8(3);
            write_type(out, base);
        }
    }
    out.usizes(&meta.derives);
    out.opt(meta.compare_by.as_ref(), |out, fields| out.usizes(fields));
    out.usizes(&meta.refinements);
}

fn write_function(out: &mut Writer, meta: &FunctionMeta) {
    out.text(&meta.module);
    out.text(&meta.name);
    out.bool(meta.is_library);
    out.bool(meta.is_method);
    out.texts(&meta.params);
    out.opt(meta.receiver.as_ref(), |out, receiver| out.text(receiver));
    out.texts(&meta.param_types);
    out.list(&meta.param_type_indices, |out, ty| {
        out.opt(ty.as_ref(), |out, index| out.usize(*index as usize))
    });
    out.opt(meta.returns.as_ref(), write_type);
    out.list(&meta.fails, write_type);
    out.list(&meta.needs, write_grant);
    out.opt(meta.purpose.as_ref(), |out, purpose| out.text(purpose));
    out.opt(meta.foreign.as_ref(), |out, foreign| {
        out.texts(&foreign.libraries);
        out.text(&foreign.symbol);
        out.texts(&foreign.parameters);
        out.text(&foreign.result);
    });
    out.opt(meta.python.as_ref(), |out, python| {
        out.text(&python.package);
        out.text(&python.symbol);
        out.opt(python.interpreter.as_ref(), |out, interpreter| {
            out.text(interpreter)
        });
        out.text(&python.root);
    });
}

fn write_constant(out: &mut Writer, value: &Value) {
    match value {
        Value::Nothing => out.u8(0),
        Value::Boolean(value) => {
            out.u8(1);
            out.bool(*value);
        }
        Value::Integer(value) => {
            out.u8(2);
            out.text(&value.to_string());
        }
        Value::Decimal(value) => {
            out.u8(3);
            out.text(&value.to_string());
        }
        Value::Float(value) => {
            out.u8(4);
            out.out.extend_from_slice(&value.to_bits().to_le_bytes());
        }
        Value::Text(text) => {
            out.u8(5);
            out.text(text);
        }
        Value::Function(id) => {
            out.u8(6);
            out.usize(*id);
        }
        other => panic!(
            "a constant of kind {}, which the encoding cannot hold",
            other.kind_name()
        ),
    }
}

fn write_op(out: &mut Writer, op: &Op) {
    out.u8(op.kind().0 as u8);
    match op {
        Op::Nothing
        | Op::Pop
        | Op::Dup
        | Op::MakePair
        | Op::Not
        | Op::ToText
        | Op::PopHandler
        | Op::Return
        | Op::ReturnNothing
        | Op::Fail
        | Op::Crash
        | Op::IsNothing
        | Op::IsFailure
        | Op::UnwrapFailure
        | Op::ListPush
        | Op::GroupInsert => {}
        Op::Const(index)
        | Op::Global(index)
        | Op::ResultType(index)
        | Op::Jump(index)
        | Op::JumpIfFalse(index)
        | Op::JumpIfTrue(index)
        | Op::JumpIfAbsent(index)
        | Op::JumpIfFailure(index)
        | Op::PushHandler(index)
        | Op::Check(index) => out.usize(*index as usize),
        Op::Load(slot)
        | Op::LoadMove(slot)
        | Op::Store(slot)
        | Op::MakeList(slot)
        | Op::MakeMap(slot)
        | Op::With(slot)
        | Op::CallValue(slot)
        | Op::Concat(slot)
        | Op::IsVariant(slot)
        | Op::Unpack(slot)
        | Op::IterInit(slot)
        | Op::Deadline(slot)
        | Op::CheckDeadline(slot)
        | Op::MarkStack(slot)
        | Op::UnwindStack(slot) => out.usize(*slot as usize),
        Op::MakeRange { stepped } => out.bool(*stepped),
        Op::Construct { ty, fields } => {
            out.usize(*ty);
            out.usize(*fields as usize);
        }
        Op::ConstructVariant { ty, tag, fields } => {
            out.usize(*ty);
            out.usize(*tag as usize);
            out.usize(*fields as usize);
        }
        Op::Field { name, site } => {
            out.usize(*name as usize);
            out.usize(*site as usize);
        }
        Op::LoadField { slot, name, site } => {
            out.usize(*slot as usize);
            out.usize(*name as usize);
            out.usize(*site as usize);
        }
        Op::Call { function, args } => {
            out.usize(*function);
            out.usize(*args as usize);
        }
        Op::CallAbility {
            ability,
            method,
            args,
        } => {
            out.usize(*ability);
            out.usize(*method as usize);
            out.usize(*args as usize);
        }
        Op::Binary(operator) => out.u8(operator_code(*operator)),
        Op::IsType(ty) => out.usize(*ty),
        Op::IterNext { slot, exit } => {
            out.usize(*slot as usize);
            out.usize(*exit as usize);
        }
        Op::GroupFold(fold) => out.u8(match fold {
            GroupFold::Sum => 0,
            GroupFold::First => 1,
            GroupFold::Any => 2,
            GroupFold::All => 3,
        }),
        Op::SortByKey { descending } => out.bool(*descending),
    }
}

fn write_code(out: &mut Writer, code: &Code) {
    out.text(&code.name);
    out.usize(code.module);
    out.u8(match code.kind {
        CodeKind::Function => 0,
        CodeKind::Test => 1,
        CodeKind::Constant => 2,
        CodeKind::Example => 3,
        CodeKind::Refinement => 4,
    });
    out.opt(code.function.as_ref(), |out, function| out.usize(*function));
    out.usize(code.params as usize);
    out.usize(code.locals as usize);
    out.list(&code.ops, write_op);
    out.list(&code.spans, |out, span| out.span(*span));
    out.list(&code.constants, write_constant);
    out.list(&code.types, |out, ty| {
        out.opt(ty.as_ref(), |out, index| out.usize(*index as usize))
    });
}

/// The program as bytes.
pub fn encode(program: &Program) -> Vec<u8> {
    let mut out = Writer::default();
    out.uint(FORMAT);
    out.usize(program.module_names.len());
    for (name, source) in program.module_names.iter().zip(&program.sources) {
        out.text(name);
        out.opt(source.as_ref(), |out, source| {
            out.text(&source.name);
            out.usizes(&source.line_starts);
        });
    }
    write_builtins(&mut out, &program.builtins);
    out.list(&program.types.metas, write_type_meta);
    let mut impls: Vec<_> = program.types.impls.iter().collect();
    impls.sort_by_key(|(key, _)| **key);
    out.usize(impls.len());
    for ((ability, ty), methods) in impls {
        out.usize(*ability);
        out.usize(*ty);
        let mut methods: Vec<_> = methods.iter().collect();
        methods.sort();
        out.usize(methods.len());
        for (name, function) in methods {
            out.text(name);
            out.usize(*function);
        }
    }
    out.list(&program.abilities, |out, (name, methods)| {
        out.text(name);
        out.texts(methods);
    });
    let mut index: Vec<_> = program.method_index.iter().collect();
    index.sort_by(|(a, _), (b, _)| a.cmp(b));
    out.usize(index.len());
    for ((ty, name), functions) in index {
        out.usize(*ty);
        out.text(name);
        out.usizes(functions);
    }
    out.usize(program.function_metas.len());
    for (meta, code) in program.function_metas.iter().zip(&program.function_codes) {
        write_function(&mut out, meta);
        out.opt(code.as_ref(), |out, code| out.usize(*code));
    }
    out.list(&program.constants, |out, constant| {
        out.usize(constant.module);
        out.text(&constant.name);
        out.usize(constant.code);
    });
    out.list(&program.tests, |out, test| {
        out.usize(test.module);
        out.text(&test.name);
        out.list(&test.needs, write_grant);
        out.opt(test.replays.as_ref(), |out, fixture| out.text(fixture));
        out.span(test.span);
        out.usize(test.code);
    });
    out.list(&program.examples, |out, example| {
        out.usize(example.function);
        out.usize(example.module);
        out.text(&example.text);
        out.span(example.span);
        out.usize(example.call);
        match example.expected {
            Expected::Is(code) => {
                out.u8(0);
                out.usize(code);
            }
            Expected::FailsWith(code) => {
                out.u8(1);
                out.usize(code);
            }
        }
    });
    out.list(&program.codes, write_code);
    out.list(&program.result_types, write_type);
    out.list(&program.specials, |out, specials| {
        for function in [
            specials.equals,
            specials.compare,
            specials.to_text,
            specials.to_list,
        ] {
            out.opt(function.as_ref(), |out, function| out.usize(*function));
        }
    });
    out.usize(program.field_sites);
    out.opt(program.main.as_ref(), |out, main| out.usize(*main));
    out.opt(program.date.as_ref(), |out, date| out.usize(*date));
    out.out
}

// ----------------------------------------------------------------- reading

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

fn truncated<T>() -> Read<T> {
    Err("the program is truncated".to_string())
}

impl<'a> Reader<'a> {
    fn remaining(&self) -> usize {
        self.bytes.len() - self.at
    }

    fn take(&mut self, count: usize) -> Read<&'a [u8]> {
        if count > self.remaining() {
            return truncated();
        }
        let taken = &self.bytes[self.at..self.at + count];
        self.at += count;
        Ok(taken)
    }

    fn u8(&mut self) -> Read<u8> {
        Ok(self.take(1)?[0])
    }

    fn uint(&mut self) -> Read<u64> {
        let mut value = 0u64;
        for shift in (0..64).step_by(7) {
            let byte = self.u8()?;
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err("the program holds an integer too wide".to_string())
    }

    fn usize(&mut self) -> Read<usize> {
        usize::try_from(self.uint()?)
            .map_err(|_| "the program holds an integer too wide".to_string())
    }

    fn u16(&mut self) -> Read<u16> {
        u16::try_from(self.uint()?).map_err(|_| "the program holds a slot too wide".to_string())
    }

    fn u32(&mut self) -> Read<u32> {
        u32::try_from(self.uint()?).map_err(|_| "the program holds an index too wide".to_string())
    }

    fn bool(&mut self) -> Read<bool> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            other => Err(format!("the program holds {other} for a boolean")),
        }
    }

    fn text(&mut self) -> Read<String> {
        let length = self.usize()?;
        let bytes = self.take(length)?;
        String::from_utf8(bytes.to_vec())
            .map_err(|_| "the program holds a text that is not UTF-8".to_string())
    }

    fn texts(&mut self) -> Read<Vec<String>> {
        self.list(Reader::text)
    }

    fn usizes(&mut self) -> Read<Vec<usize>> {
        self.list(Reader::usize)
    }

    fn opt<T>(&mut self, read: impl FnOnce(&mut Reader<'a>) -> Read<T>) -> Read<Option<T>> {
        match self.u8()? {
            0 => Ok(None),
            1 => read(self).map(Some),
            other => Err(format!("the program holds {other} for an option")),
        }
    }

    fn list<T>(&mut self, read: impl Fn(&mut Reader<'a>) -> Read<T>) -> Read<Vec<T>> {
        let count = self.usize()?;
        // every item takes a byte at least: a count past the end is not
        // a count
        if count > self.remaining() {
            return truncated();
        }
        (0..count).map(|_| read(self)).collect()
    }

    fn span(&mut self) -> Read<Span> {
        Ok(Span::new(self.usize()?, self.usize()?))
    }

    fn float(&mut self) -> Read<f64> {
        let bytes = self.take(8)?;
        let mut word = [0u8; 8];
        word.copy_from_slice(bytes);
        Ok(f64::from_bits(u64::from_le_bytes(word)))
    }
}

fn read_type(input: &mut Reader) -> Read<Ty> {
    Ok(match input.u8()? {
        0 => {
            let id = input.usize()?;
            Ty::App(id, input.list(read_type)?)
        }
        1 => Ty::Maybe(Box::new(read_type(input)?)),
        2 => Ty::Function(Box::new(FunctionTy {
            params: input.list(read_type)?,
            returns: input.opt(read_type)?,
            fails: input.list(read_type)?,
            needs: input.list(read_grant)?,
        })),
        3 => Ty::Param(input.usize()?),
        4 => Ty::Var(input.usize()?),
        5 => Ty::SelfType,
        6 => Ty::Union(input.list(read_type)?),
        7 => Ty::Unit,
        8 => Ty::Never,
        9 => Ty::Error,
        other => return Err(format!("the program holds {other} for a type")),
    })
}

fn read_grant(input: &mut Reader) -> Read<Capability> {
    Ok(Capability {
        path: input.texts()?,
        scope: input.opt(Reader::text)?,
        budget: input.opt(|input| Ok((input.text()?, input.text()?)))?,
        only_to: input.list(|input| Ok((input.texts()?, input.opt(Reader::text)?)))?,
    })
}

fn read_builtins(input: &mut Reader) -> Read<Builtins> {
    Ok(Builtins {
        integer: input.usize()?,
        decimal: input.usize()?,
        float: input.usize()?,
        boolean: input.usize()?,
        text: input.usize()?,
        bytes: input.usize()?,
        list: input.usize()?,
        map: input.usize()?,
        set: input.usize()?,
        range: input.usize()?,
        pair: input.usize()?,
        duration: input.usize()?,
        ordering: input.usize()?,
        timed_out: input.usize()?,
        constraint_violation: input.usize()?,
        guarded: input.usize()?,
        equal: input.usize()?,
        compare: input.usize()?,
        hash: input.usize()?,
        to_text: input.usize()?,
        iterable: input.usize()?,
    })
}

fn read_field(input: &mut Reader) -> Read<FieldMeta> {
    Ok(FieldMeta {
        name: input.text()?,
        ty: read_type(input)?,
        external_name: input.opt(Reader::text)?,
        optional: input.bool()?,
        refinement: input.opt(Reader::usize)?,
    })
}

fn read_type_meta(input: &mut Reader) -> Read<TypeMeta> {
    let id = input.usize()?;
    let name = input.text()?;
    let module = input.text()?;
    let is_library = input.bool()?;
    let params = input.usizes()?;
    let shape = match input.u8()? {
        0 => TypeShape::Opaque,
        1 => TypeShape::Record(input.list(read_field)?),
        2 => TypeShape::Sum(input.list(|input| {
            Ok(VariantMeta {
                name: input.text()?,
                fields: input.list(read_field)?,
            })
        })?),
        3 => TypeShape::Subtype {
            base: read_type(input)?,
        },
        other => return Err(format!("the program holds {other} for a shape")),
    };
    Ok(TypeMeta {
        id,
        name,
        module,
        is_library,
        params,
        shape,
        derives: input.usizes()?,
        compare_by: input.opt(Reader::usizes)?,
        refinements: input.usizes()?,
    })
}

fn read_function(input: &mut Reader) -> Read<FunctionMeta> {
    Ok(FunctionMeta {
        module: input.text()?,
        name: input.text()?,
        is_library: input.bool()?,
        is_method: input.bool()?,
        params: input.texts()?,
        receiver: input.opt(Reader::text)?,
        param_types: input.texts()?,
        param_type_indices: input
            .list(|input| Ok(input.opt(Reader::usize)?.map(|index| index as u32)))?,
        returns: input.opt(read_type)?,
        fails: input.list(read_type)?,
        needs: input.list(read_grant)?,
        purpose: input.opt(Reader::text)?,
        foreign: input.opt(|input| {
            Ok(Foreign {
                libraries: input.texts()?,
                symbol: input.text()?,
                parameters: input.texts()?,
                result: input.text()?,
            })
        })?,
        python: input.opt(|input| {
            Ok(Python {
                package: input.text()?,
                symbol: input.text()?,
                interpreter: input.opt(Reader::text)?,
                root: input.text()?,
            })
        })?,
    })
}

fn read_constant(input: &mut Reader) -> Read<Value> {
    Ok(match input.u8()? {
        0 => Value::Nothing,
        1 => Value::Boolean(input.bool()?),
        2 => {
            let digits = input.text()?;
            Value::Integer(
                Int::parse(&digits).ok_or_else(|| format!("`{digits}` is not an Integer"))?,
            )
        }
        3 => {
            let digits = input.text()?;
            Value::decimal(
                Decimal::parse(&digits).ok_or_else(|| format!("`{digits}` is not a Decimal"))?,
            )
        }
        4 => Value::Float(input.float()?),
        5 => Value::text(input.text()?),
        6 => Value::Function(input.usize()?),
        other => return Err(format!("the program holds {other} for a constant")),
    })
}

fn read_op(input: &mut Reader) -> Read<Op> {
    let kind = input.u8()?;
    Ok(match kind {
        0 => Op::Const(input.u32()?),
        1 => Op::Nothing,
        2 => Op::Global(input.u32()?),
        3 => Op::Load(input.u16()?),
        4 => Op::LoadMove(input.u16()?),
        5 => Op::Store(input.u16()?),
        6 => Op::Pop,
        7 => Op::Dup,
        8 => Op::MakeList(input.u16()?),
        9 => Op::MakeMap(input.u16()?),
        10 => Op::MakePair,
        11 => Op::MakeRange {
            stepped: input.bool()?,
        },
        12 => Op::Construct {
            ty: input.usize()?,
            fields: input.u16()?,
        },
        13 => Op::ConstructVariant {
            ty: input.usize()?,
            tag: input.u16()?,
            fields: input.u16()?,
        },
        14 => Op::Field {
            name: input.u32()?,
            site: input.u32()?,
        },
        15 => Op::With(input.u16()?),
        16 => Op::Call {
            function: input.usize()?,
            args: input.u16()?,
        },
        17 => Op::CallAbility {
            ability: input.usize()?,
            method: input.u16()?,
            args: input.u16()?,
        },
        18 => Op::CallValue(input.u16()?),
        19 => Op::ResultType(input.u32()?),
        20 => Op::Not,
        21 => {
            let code = input.u8()?;
            Op::Binary(
                OPERATORS
                    .get(code as usize)
                    .copied()
                    .ok_or_else(|| format!("the program holds {code} for an operator"))?,
            )
        }
        22 => Op::ToText,
        23 => Op::Concat(input.u16()?),
        24 => Op::Jump(input.u32()?),
        25 => Op::JumpIfFalse(input.u32()?),
        26 => Op::JumpIfTrue(input.u32()?),
        27 => Op::JumpIfAbsent(input.u32()?),
        28 => Op::JumpIfFailure(input.u32()?),
        29 => Op::PushHandler(input.u32()?),
        30 => Op::PopHandler,
        31 => Op::Return,
        32 => Op::ReturnNothing,
        33 => Op::Fail,
        34 => Op::Crash,
        35 => Op::IsVariant(input.u16()?),
        36 => Op::IsNothing,
        37 => Op::IsFailure,
        38 => Op::IsType(input.usize()?),
        39 => Op::Unpack(input.u16()?),
        40 => Op::UnwrapFailure,
        41 => Op::IterInit(input.u16()?),
        42 => Op::IterNext {
            slot: input.u16()?,
            exit: input.u32()?,
        },
        43 => Op::ListPush,
        44 => Op::GroupInsert,
        45 => Op::GroupFold(match input.u8()? {
            0 => GroupFold::Sum,
            1 => GroupFold::First,
            2 => GroupFold::Any,
            3 => GroupFold::All,
            other => return Err(format!("the program holds {other} for a fold")),
        }),
        46 => Op::SortByKey {
            descending: input.bool()?,
        },
        47 => Op::Deadline(input.u16()?),
        48 => Op::CheckDeadline(input.u16()?),
        49 => Op::MarkStack(input.u16()?),
        50 => Op::UnwindStack(input.u16()?),
        51 => Op::Check(input.u32()?),
        52 => Op::LoadField {
            slot: input.u16()?,
            name: input.u32()?,
            site: input.u32()?,
        },
        other => return Err(format!("the program holds {other} for an op")),
    })
}

fn read_code(input: &mut Reader) -> Read<Code> {
    Ok(Code {
        name: input.text()?,
        module: input.usize()?,
        kind: match input.u8()? {
            0 => CodeKind::Function,
            1 => CodeKind::Test,
            2 => CodeKind::Constant,
            3 => CodeKind::Example,
            4 => CodeKind::Refinement,
            other => return Err(format!("the program holds {other} for a code kind")),
        },
        function: input.opt(Reader::usize)?,
        params: input.u16()?,
        locals: input.u16()?,
        ops: input.list(read_op)?,
        spans: input.list(Reader::span)?,
        constants: input.list(read_constant)?,
        types: input.list(|input| Ok(input.opt(Reader::usize)?.map(|index| index as u32)))?,
    })
}

/// The program the bytes hold; the error says what does not fit. Every
/// index is checked to point inside the program, as the file's loader
/// checks (`file::check`).
pub fn decode(bytes: &[u8]) -> Read<Program> {
    let mut input = Reader { bytes, at: 0 };
    let format = input.uint()?;
    if format != FORMAT {
        return Err(format!(
            "the program is encoding {format}; this VM reads encoding {FORMAT}"
        ));
    }
    let modules: Vec<(String, Option<SourceLines>)> = input.list(|input| {
        Ok((
            input.text()?,
            input.opt(|input| {
                Ok(SourceLines {
                    name: input.text()?,
                    line_starts: input.usizes()?,
                })
            })?,
        ))
    })?;
    let builtins = read_builtins(&mut input)?;
    let metas = input.list(read_type_meta)?;
    let impls: Vec<((usize, usize), HashMap<String, usize>)> = input.list(|input| {
        let key = (input.usize()?, input.usize()?);
        let methods = input.list(|input| Ok((input.text()?, input.usize()?)))?;
        Ok((key, methods.into_iter().collect()))
    })?;
    let abilities = input.list(|input| Ok((input.text()?, input.texts()?)))?;
    let method_index: Vec<((usize, String), Vec<usize>)> = input.list(|input| {
        let key = (input.usize()?, input.text()?);
        Ok((key, input.usizes()?))
    })?;
    let functions: Vec<(FunctionMeta, Option<CodeId>)> =
        input.list(|input| Ok((read_function(input)?, input.opt(Reader::usize)?)))?;
    let constants = input.list(|input| {
        Ok(ConstantMeta {
            module: input.usize()?,
            name: input.text()?,
            code: input.usize()?,
        })
    })?;
    let tests = input.list(|input| {
        Ok(TestMeta {
            module: input.usize()?,
            name: input.text()?,
            needs: input.list(read_grant)?,
            replays: input.opt(Reader::text)?,
            span: input.span()?,
            code: input.usize()?,
        })
    })?;
    let examples = input.list(|input| {
        Ok(ExampleMeta {
            function: input.usize()?,
            module: input.usize()?,
            text: input.text()?,
            span: input.span()?,
            call: input.usize()?,
            expected: match input.u8()? {
                0 => Expected::Is(input.usize()?),
                1 => Expected::FailsWith(input.usize()?),
                other => return Err(format!("the program holds {other} for an expectation")),
            },
        })
    })?;
    let codes = input.list(read_code)?;
    let result_types = input.list(read_type)?;
    let specials = input.list(|input| {
        Ok(Specials {
            equals: input.opt(Reader::usize)?,
            compare: input.opt(Reader::usize)?,
            to_text: input.opt(Reader::usize)?,
            to_list: input.opt(Reader::usize)?,
        })
    })?;
    let field_sites = input.usize()?;
    let main = input.opt(Reader::usize)?;
    let date = input.opt(Reader::usize)?;
    if input.remaining() != 0 {
        return Err(format!("{} bytes after the program", input.remaining()));
    }
    let (module_names, sources): (Vec<String>, Vec<Option<SourceLines>>) =
        modules.into_iter().unzip();
    let function_codes: Vec<Option<CodeId>> = functions.iter().map(|(_, code)| *code).collect();
    let function_table: HashMap<usize, CodeId> = function_codes
        .iter()
        .enumerate()
        .filter_map(|(id, code)| code.map(|code| (id, code)))
        .collect();
    let program = Program {
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
    };
    crate::file::check(&program)?;
    Ok(program)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_round_trip() {
        let mut out = Writer::default();
        let values = [
            0u64,
            1,
            127,
            128,
            300,
            16_383,
            16_384,
            u32::MAX as u64,
            u64::MAX,
        ];
        for value in values {
            out.uint(value);
        }
        let mut input = Reader {
            bytes: &out.out,
            at: 0,
        };
        for value in values {
            assert_eq!(input.uint().expect("an integer"), value);
        }
        assert_eq!(input.remaining(), 0);
    }

    fn refusal(bytes: &[u8]) -> String {
        match decode(bytes) {
            Err(message) => message,
            Ok(_) => panic!("the bytes decoded"),
        }
    }

    #[test]
    fn a_truncated_program_is_refused() {
        assert_eq!(refusal(&[]), "the program is truncated");
        let other = (FORMAT + 1) as u8;
        assert!(refusal(&[other]).contains(&format!("encoding {other}")));
        // the format, then a count of modules past the end
        assert_eq!(refusal(&[FORMAT as u8, 9]), "the program is truncated");
    }
}
