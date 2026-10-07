//! Patterns test the value in a slot and bind names to slots. A pattern
//! returns the jumps taken when it does not fit; the caller patches them to
//! the next arm.

use renyi_check::Target;
use renyi_syntax::ast::{BinaryOp, Pattern, Type};

use super::Compiler;
use crate::bytecode::Op;

impl Compiler<'_, '_> {
    pub fn pattern(&mut self, pattern: &Pattern, slot: u16) -> Vec<usize> {
        let span = pattern.span();
        let mut next = Vec::new();
        match pattern {
            Pattern::Binding(name) => self.alias(&name.text, slot),
            Pattern::Nothing(_) => {
                self.emit(Op::Load(slot), span);
                self.emit(Op::IsNothing, span);
                next.push(self.emit(Op::JumpIfFalse(0), span));
            }
            Pattern::Some(inner, _) => {
                self.emit(Op::Load(slot), span);
                self.emit(Op::IsNothing, span);
                next.push(self.emit(Op::JumpIfTrue(0), span));
                next.extend(self.pattern(inner, slot));
            }
            Pattern::Success(inner, _) => {
                self.emit(Op::Load(slot), span);
                self.emit(Op::IsFailure, span);
                next.push(self.emit(Op::JumpIfTrue(0), span));
                next.extend(self.pattern(inner, slot));
            }
            Pattern::Failure(inner, _) => {
                self.emit(Op::Load(slot), span);
                self.emit(Op::IsFailure, span);
                next.push(self.emit(Op::JumpIfFalse(0), span));
                let error = self.temp();
                self.emit(Op::Load(slot), span);
                self.emit(Op::UnwrapFailure, span);
                self.emit(Op::Store(error), span);
                next.extend(self.pattern(inner, error));
            }
            Pattern::Literal(literal) => {
                self.emit(Op::Load(slot), span);
                self.expr(literal);
                self.emit(Op::Binary(BinaryOp::Is), span);
                next.push(self.emit(Op::JumpIfFalse(0), span));
            }
            Pattern::Typed { name, ty, .. } => {
                match self.type_of(ty) {
                    Some(id) => {
                        self.emit(Op::Load(slot), span);
                        self.emit(Op::IsType(id), span);
                        next.push(self.emit(Op::JumpIfFalse(0), span));
                    }
                    None => self.unsupported("the type of this pattern", span),
                }
                self.alias(&name.text, slot);
            }
            Pattern::Variant { name, fields, .. } => {
                let mut found = None;
                for target in self.targets(name.span).to_vec() {
                    match target {
                        Target::Variant(ty, tag) => found = Some((ty, Some(tag))),
                        Target::Type(ty) => found = Some((ty, None)),
                        _ => {}
                    }
                }
                let Some((_, tag)) = found else {
                    self.unsupported(&format!("the pattern `{}`", name.text), span);
                    return next;
                };
                // a record matched like a single variant (decision K6) has no tag
                if let Some(tag) = tag {
                    self.emit(Op::Load(slot), span);
                    self.emit(Op::IsVariant(tag as u16), span);
                    next.push(self.emit(Op::JumpIfFalse(0), span));
                }
                for field in fields {
                    let field_slot = self.temp();
                    let name = self.name_constant(&field.field.text);
                    let site = self.field_site();
                    self.emit(Op::Load(slot), field.field.span);
                    self.emit(Op::Field { name, site }, field.field.span);
                    self.emit(Op::Store(field_slot), field.field.span);
                    match &field.pattern {
                        Some(inner) => next.extend(self.pattern(inner, field_slot)),
                        None => self.alias(&field.field.text, field_slot),
                    }
                }
            }
        }
        next
    }

    fn type_of(&self, ty: &Type) -> Option<usize> {
        match ty {
            Type::Named { name, .. } => self.lookup_type(&name.text),
            Type::Maybe(inner, _) => self.type_of(inner),
            Type::Function { .. } => None,
        }
    }
}
