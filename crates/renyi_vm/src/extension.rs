//! Extensions (decisions AJ1 and AK1 to AK4): native functions registered
//! with the VM under declaration files and built into a `renyi` binary.
//! The standard library is the first extension (`natives::standard`);
//! another is a Rust crate that holds an [`Extension`] value, and a binary
//! that carries it is `renyi::main_with(vec![EXTENSION])`. A registered
//! native is checked, recorded, replayed and narrated like a library
//! primitive: the boundary of `vm.rs` does not know which extension a
//! function came from.
//!
//! A declaration file and its natives meet by name (AK2): a [`Native`]
//! names its module, its function and the type of the function's first
//! parameter as the checker spells it, in full (`List of Integer`) or by
//! its head (`List`), and [`Registry::verify`] holds the table and the
//! declarations equal both ways. A native sees what the standard
//! library's natives see (AK3): the VM, the plain values of its arguments
//! and the helpers of [`crate::natives`]. It declares its capabilities
//! from the kinds of reference section 11 and adds none (AK4).

use std::collections::HashMap;

use renyi_check::Library;
use renyi_syntax::parse_declarations;

use crate::natives::NativeFn;

/// A native function of an extension: which declared function it
/// implements, and the Rust function.
#[derive(Clone, Copy)]
pub struct Native {
    /// The module of the declaration file: `std.json`, `python.pandas`.
    pub module: &'static str,
    /// The function's name in the declaration file.
    pub name: &'static str,
    /// The type of the function's first parameter as the checker spells
    /// it (`Text`, `List of Integer`) or its head (`List`); `None` for a
    /// function matched by its name alone, whatever its first parameter.
    pub receiver: Option<&'static str>,
    pub run: NativeFn,
}

impl Native {
    /// A function matched by its name alone:
    /// `Native::function("std.json", "parse", parse)`.
    pub const fn function(module: &'static str, name: &'static str, run: NativeFn) -> Native {
        Native {
            module,
            name,
            receiver: None,
            run,
        }
    }

    /// A function matched by its name and the type of its first parameter,
    /// a method most often:
    /// `Native::method("std.prelude", "length", "Text", text_length)`.
    pub const fn method(
        module: &'static str,
        name: &'static str,
        receiver: &'static str,
        run: NativeFn,
    ) -> Native {
        Native {
            module,
            name,
            receiver: Some(receiver),
            run,
        }
    }
}

/// What an extension brings: its declaration files, one per module, and
/// its natives. The fields are static so that an extension is a constant
/// of its crate.
#[derive(Clone, Copy)]
pub struct Extension {
    pub name: &'static str,
    pub version: &'static str,
    /// Each module's name and the text of its declaration file, in the
    /// order they are declared: a module before one that imports it.
    pub modules: &'static [(&'static str, &'static str)],
    pub natives: &'static [Native],
}

/// The natives registered under one module and name, in registration order.
type Entries = Vec<(Option<&'static str>, NativeFn)>;

/// The extensions a toolchain is built with, the standard library first,
/// and the natives they register, by module, name and receiver.
#[derive(Clone)]
pub struct Registry {
    extensions: Vec<Extension>,
    /// Per module and function name, the entries in registration order.
    table: HashMap<(&'static str, &'static str), Entries>,
}

impl Registry {
    /// The standard library alone: what the official binary carries.
    pub fn standard() -> Registry {
        let mut registry = Registry {
            extensions: Vec::new(),
            table: HashMap::new(),
        };
        registry.add(crate::natives::standard());
        registry
    }

    /// Register an extension after those already registered. Nothing is
    /// checked here: `verify` is.
    pub fn add(&mut self, extension: Extension) {
        for native in extension.natives {
            self.table
                .entry((native.module, native.name))
                .or_default()
                .push((native.receiver, native.run));
        }
        self.extensions.push(extension);
    }

    /// `add`, for a chain: `Registry::standard().with(EXTENSION)`.
    pub fn with(mut self, extension: Extension) -> Registry {
        self.add(extension);
        self
    }

    /// Every extension, the standard library first.
    pub fn extensions(&self) -> &[Extension] {
        &self.extensions
    }

    /// `name version` of every extension but the standard library, in
    /// registration order: what the run manifest names (decision AK1).
    pub fn extras(&self) -> Vec<String> {
        self.extensions
            .iter()
            .skip(1)
            .map(|extension| format!("{} {}", extension.name, extension.version))
            .collect()
    }

    /// The declaration files of every extension, in order, for the
    /// checker.
    pub fn library(&self) -> Library {
        let mut library = Library::empty();
        for extension in &self.extensions {
            for (name, source) in extension.modules {
                library.add(name, source);
            }
        }
        library
    }

    /// The native behind a declared function: the entry whose receiver is
    /// the type's full spelling, else its head, else the entry without a
    /// receiver.
    pub fn lookup(&self, module: &str, name: &str, receiver: Option<&str>) -> Option<NativeFn> {
        let entries = self.table.get(&(module, name))?;
        let find = |wanted: Option<&str>| {
            entries
                .iter()
                .find(|(entry, _)| *entry == wanted)
                .map(|(_, run)| *run)
        };
        let head = receiver.map(|receiver| receiver.split(' ').next().unwrap_or(receiver));
        receiver
            .and_then(|full| find(Some(full)))
            .or_else(|| head.and_then(|head| find(Some(head))))
            .or_else(|| find(None))
    }

    /// The table against the declarations, both ways (decision AK2):
    /// every declaration file parses and declares the module it is
    /// registered under, no module is declared twice, the declarations
    /// check, every declared function has a native, and every native
    /// implements a declared function. The first problem of the files
    /// stops the check; the mismatches of the table are all reported, one
    /// per line. A binary whose extensions fail this does not start; the
    /// standard library passes it in a test.
    pub fn verify(&self) -> Result<(), String> {
        let mut owners: HashMap<&'static str, &'static str> = HashMap::new();
        for extension in &self.extensions {
            for (module, source) in extension.modules {
                if let Some(other) = owners.insert(module, extension.name) {
                    return Err(format!(
                        "the module `{module}` is declared by the extensions `{other}` and `{}`",
                        extension.name
                    ));
                }
                let parsed = parse_declarations(source);
                if let Some(problem) = parsed.diagnostics.iter().find(|d| d.is_error()) {
                    return Err(format!(
                        "extension `{}`: the declaration file of `{module}` does not parse: {}",
                        extension.name, problem.message
                    ));
                }
                let declared: Vec<&str> =
                    parsed.module.name.iter().map(|n| n.text.as_str()).collect();
                let declared = declared.join(".");
                if declared != *module {
                    return Err(format!(
                        "extension `{}`: the declaration file registered as `{module}` declares `module {declared}`",
                        extension.name
                    ));
                }
            }
        }
        let owner = |module: &str| owners.get(module).copied().unwrap_or("?");
        let mut world = self.library().world();
        world.resolve_all();
        let mut problems = Vec::new();
        for (id, diagnostic) in &world.diagnostics {
            if diagnostic.is_error() {
                let module = &world.modules[*id].name;
                return Err(format!(
                    "extension `{}`: `{module}`: {}",
                    owner(module),
                    diagnostic.message
                ));
            }
        }
        for info in &world.functions {
            if !info.is_library {
                continue;
            }
            let module = &world.modules[info.module].name;
            let receiver = info.params.first().map(|(_, ty)| world.show(ty));
            if self
                .lookup(module, &info.name, receiver.as_deref())
                .is_none()
            {
                let on = receiver
                    .map(|receiver| format!(" on `{receiver}`"))
                    .unwrap_or_default();
                problems.push(format!(
                    "extension `{}`: `{module}.{}`{on} is declared but has no native",
                    owner(module),
                    info.name
                ));
            }
        }
        for extension in &self.extensions {
            for native in extension.natives {
                let implemented = world.functions.iter().any(|info| {
                    info.is_library
                        && world.modules[info.module].name == native.module
                        && info.name == native.name
                        && match native.receiver {
                            None => true,
                            Some(wanted) => info
                                .params
                                .first()
                                .map(|(_, ty)| world.show(ty))
                                .is_some_and(|full| {
                                    full == wanted || full.split(' ').next() == Some(wanted)
                                }),
                        }
                });
                if !implemented {
                    let on = native
                        .receiver
                        .map(|receiver| format!(" on `{receiver}`"))
                        .unwrap_or_default();
                    problems.push(format!(
                        "extension `{}`: the native `{}.{}`{on} implements no declared function",
                        extension.name, native.module, native.name
                    ));
                }
            }
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(problems.join("\n"))
        }
    }
}
