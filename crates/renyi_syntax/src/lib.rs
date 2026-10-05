//! Front end of the Renyi toolchain: source positions, tokens, the lexer and
//! diagnostics. The parser and the formatter join this crate at M1.
//!
//! The surface syntax is specified in `docs/design/02-syntax-sketch.md`; the
//! lexer follows its sections 1 and 17 (lexical structure, reserved words and
//! phrases).

pub mod ast;
pub mod diagnostics;
pub mod format;
pub mod layout;
pub mod lexer;
pub mod parser;
pub mod span;
pub mod token;

pub use diagnostics::{Diagnostic, Severity};
pub use format::format;
pub use lexer::{lex, Lexed};
pub use parser::{parse, Parsed};
pub use span::{Position, SourceFile, Span};
pub use token::{TextPart, Token, TokenKind, Word};
