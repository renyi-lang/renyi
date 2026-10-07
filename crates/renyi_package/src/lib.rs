//! Packages (decision AC1): the project manifest `renyi.json`, the lockfile
//! `renyi.lock.json`, versions, the layout of a registry and of the store
//! under `.renyi/packages`, the content hash of a package, and the
//! resolution of a program's imports to its own files and the files of its
//! dependencies, which the checker and the toolchain share. The commands
//! that write the files (`renyi add`, `update`, `fetch`, `publish`, `audit`)
//! are in the binary; the front end written in Renyi resolves the same way
//! in `compiler/project.ry`.

pub mod manifest;
pub mod registry;
pub mod resolve;
pub mod version;

pub use manifest::{
    Budgets, Effect, Lock, Locked, Manifest, PackageFile, Versions, LOCK_FILE, MANIFEST_FILE,
    PACKAGE_FILE, VERSIONS_FILE,
};
pub use registry::{hash_of, is_absolute, join, Registry, STORE};
pub use resolve::{resolve, Problem, Project, Resolved};
pub use version::Version;
