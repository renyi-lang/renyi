//! The official `renyi` binary: the toolchain with the standard library
//! and no other extension (decision AK1). A binary with extensions is a
//! crate of these lines with its extensions in the list.

fn main() -> std::process::ExitCode {
    renyi::main_with(Vec::new())
}
