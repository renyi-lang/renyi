//! The official `renyi` binary: the toolchain with the standard library
//! and no other extension (decision AK1). A binary with extensions is a
//! crate of these lines with its extensions in the list.

/// The allocator of the binary (decisions X6 and AP1): mimalloc, with the
/// counting the memory budget of a sandbox needs.
#[global_allocator]
static ALLOCATOR: renyi::Allocator = renyi::Allocator;

fn main() -> std::process::ExitCode {
    renyi::main_with(Vec::new())
}
