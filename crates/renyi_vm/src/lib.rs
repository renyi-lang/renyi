//! The Renyi virtual machine (milestone M3): a stack-based bytecode
//! interpreter over the checked program, with the library's primitives in
//! Rust behind one boundary. Values are immutable and reference counted
//! (decision O1); effects happen only through declared capabilities.
//!
//! The first slice runs `main`, `example:` lines and `test` blocks of
//! programs that use the prelude, console, environment, time, random,
//! filesystem, JSON, CSV and regular expressions. `run concurrently` and
//! `concurrently` queries run their tasks one after the other; `std.http`,
//! `std.server` and `std.sqlite` crash with a message when called.

pub mod bytecode;
pub mod compile;
pub mod decimal;
pub mod integer;
pub mod natives;
pub mod render;
pub mod runner;
pub mod types;
pub mod value;
pub mod vm;

pub use compile::{compile_project, Program};
pub use decimal::Decimal;
pub use integer::Int;
pub use runner::{run_main, run_tests, RunOutcome, TestOutcome, TestReport, TestResult};
pub use value::Value;
pub use vm::{Interrupt, Options, Vm};
