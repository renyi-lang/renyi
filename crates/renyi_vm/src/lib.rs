//! The Renyi virtual machine (milestone M3): a stack-based bytecode
//! interpreter over the checked program, with the library's primitives in
//! Rust behind one boundary. Values are immutable and reference counted
//! (decision O1); effects happen only through declared capabilities.
//!
//! The VM runs `main`, `example:` lines and `test` blocks of programs that
//! use the prelude, console, environment, time, random, filesystem, JSON,
//! CSV and regular expressions. Every primitive call under a capability
//! passes one boundary (`Vm::call_native`), where the grant and its budgets
//! are checked and where a run is recorded, replayed or narrated (decisions
//! P1 and P2). `run concurrently` and `concurrently` queries run their tasks
//! one after the other; `std.http`, `std.server` and `std.sqlite` crash
//! with a message when called live.

pub mod bytecode;
pub mod compile;
pub mod decimal;
pub mod grant;
pub mod integer;
pub mod natives;
pub mod recording;
pub mod render;
pub mod runner;
pub mod types;
pub mod value;
pub mod vm;

pub use compile::{compile_project, Program};
pub use decimal::Decimal;
pub use grant::Narrowing;
pub use integer::Int;
pub use recording::Recording;
pub use runner::{
    denied_functions, run_main, run_program, run_tests, Run, RunOutcome, TestOutcome, TestReport,
    TestResult,
};
pub use value::Value;
pub use vm::{Interrupt, Options, Vm};
