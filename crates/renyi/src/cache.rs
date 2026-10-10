//! The caches of `renyi run`, both in one directory of the user's.
//!
//! The image cache (decision AU10): a program whose run compiled machine
//! code gets an image (decision AS1), built in the background once the
//! run is over by `renyi build --cache`, and every later run of the same
//! program on the same `renyi` and the same machine loads its code from
//! there and compiles nothing. An entry is named by a hash of this
//! `renyi`'s version, the machine's target and the program's binary
//! encoding (decision AT3), and a hit is an image whose header fits this
//! machine and whose program is, byte for byte, the program the run
//! compiled: a file changed, corrupted or written for another program is
//! a miss, never a different program run.
//!
//! The cache keyed by the sources (decisions AU40 and AU43): every run of
//! `run`, `record` or `test` compiled from its sources leaves the program
//! it compiled, in the same binary encoding, with every file the compile
//! read (its content hash, or that it was looked for and not there), the
//! warnings the compile printed and, when the run wanted a manifest, the
//! code hash and the dependencies it computed; a later run of the same
//! path in the same working directory, by the same binary, whose files
//! all read as they did loads that program in place of running the front
//! end, prints the warnings again, and then finds the image as before. An
//! entry is named by a hash of the binary (its version, its executable's
//! path, size and modification time, the library's declaration files),
//! the working directory and the path as given.
//!
//! Both are off under `RENYI_NO_CACHE` and `--no-cache`; the image cache
//! also for a run on the interpreter; `RENYI_CACHE_DIR` names the
//! directory, and the entries of both together are kept under the limit.

use std::fmt::Write as _;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use renyi_check::Library;
use renyi_json::{read_json, write_json, Json};
use renyi_package::Read;
use renyi_vm::native::image::{self, Image};
use renyi_vm::native::Jit;
use renyi_vm::recording::{sha256_of, Dependency};
use renyi_vm::Program;

/// What the entries of the cache may take together; the least recently
/// used go first when a new one would exceed it.
const LIMIT: u64 = 256 * 1024 * 1024;

/// How long a build in the background is waited for before another run
/// starts its own: a lock older than this is a build that died.
const LOCK_LIFE: Duration = Duration::from_secs(600);

/// The extension of an entry of the cache keyed by the sources.
pub(crate) const SOURCES_EXTENSION: &str = "rys";

/// The first line of such an entry: what it is and its format.
const SOURCES_FORMAT: &str = "renyi sources 1";

/// The cache's directory: `RENYI_CACHE_DIR` when set (empty turns the
/// cache off), else `renyi/images` under the system's cache directory;
/// `None` under `RENYI_NO_CACHE` or when no directory is known.
pub(crate) fn directory() -> Option<PathBuf> {
    if std::env::var_os("RENYI_NO_CACHE").is_some() {
        return None;
    }
    if let Some(directory) = std::env::var_os("RENYI_CACHE_DIR") {
        return (!directory.is_empty()).then(|| PathBuf::from(directory));
    }
    let home = || std::env::var_os("HOME").filter(|home| !home.is_empty());
    let base = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        home().map(|home| PathBuf::from(home).join("Library").join("Caches"))
    } else {
        std::env::var_os("XDG_CACHE_HOME")
            .filter(|directory| !directory.is_empty())
            .map(PathBuf::from)
            .or_else(|| home().map(|home| PathBuf::from(home).join(".cache")))
    }?;
    Some(base.join("renyi").join("images"))
}

/// Whether `RENYI_CACHE_REPORT` asks for a line on the standard error at
/// every hit and every store, a development aid like `RENYI_NATIVE_REPORT`.
pub(crate) fn reporting() -> bool {
    std::env::var_os("RENYI_CACHE_REPORT").is_some()
}

/// The first 32 hex digits of a key's hash, the name of an entry.
fn name_of(key: &str, extension: &str) -> String {
    let hash = sha256_of(key.as_bytes());
    let key = hash.trim_start_matches("sha256:");
    format!("{}.{}", &key[..32.min(key.len())], extension)
}

/// A program's place in the cache: the file its image is kept in, the
/// machine's target and the program's binary encoding, which a hit must
/// hold byte for byte.
pub(crate) struct Entry {
    path: PathBuf,
    target: String,
    encoding: Vec<u8>,
}

impl Entry {
    /// The entry of a program in a directory; `None` when the machine
    /// generates no code.
    pub(crate) fn of(program: &Program, directory: &Path) -> Option<Entry> {
        Entry::of_encoding(renyi_vm::binary::encode(program), directory)
    }

    /// `of` from the program's binary encoding, when a caller holds it
    /// already (the cache keyed by the sources does, decision AU43).
    pub(crate) fn of_encoding(encoding: Vec<u8>, directory: &Path) -> Option<Entry> {
        let target = Jit::host_target()?;
        let named = format!(
            "{}\n{}\n{}",
            image::this_renyi(),
            target,
            sha256_of(&encoding)
        );
        Some(Entry {
            path: directory.join(name_of(&named, image::EXTENSION)),
            target,
            encoding,
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    fn lock(&self) -> PathBuf {
        self.path.with_extension("lock")
    }

    /// The image of the entry, when the cache holds one for this program
    /// and this machine; its use marks it recently used.
    pub(crate) fn lookup(&self) -> Option<Image> {
        let loaded = Image::open(self.path.to_str()?).ok()?;
        if loaded.header.mismatch(&self.target).is_some() || loaded.program != self.encoding {
            return None;
        }
        touch(&self.path);
        Some(loaded)
    }

    /// The image built in the background by `renyi build --cache <path>`
    /// (this binary, the same extensions), unless a build of the same
    /// program is under way; nothing is reported, and a build that fails
    /// leaves the cache as it was.
    pub(crate) fn store_later(&self, path: &str) {
        let Some(directory) = self.path.parent() else {
            return;
        };
        if fs::create_dir_all(directory).is_err() {
            return;
        }
        let lock = self.lock();
        let busy = fs::metadata(&lock)
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|modified| SystemTime::now().duration_since(modified).ok())
            .is_some_and(|age| age < LOCK_LIFE);
        if busy || fs::write(&lock, std::process::id().to_string()).is_err() {
            return;
        }
        let Ok(binary) = std::env::current_exe() else {
            let _ = fs::remove_file(&lock);
            return;
        };
        let mut command = Command::new(binary);
        command
            .args(["build", "--cache", path])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        // out of the terminal's process group, so that an interrupt meant
        // for the next command leaves the build alone
        #[cfg(unix)]
        std::os::unix::process::CommandExt::process_group(&mut command, 0);
        if command.spawn().is_err() {
            let _ = fs::remove_file(&lock);
        }
    }

    /// The image written to the entry: to a file of its own first, then
    /// renamed, so that a run never maps half an image; the lock removed,
    /// the cache pruned to its limit.
    pub(crate) fn store(&self, bytes: &[u8]) -> Result<(), String> {
        let written = write_whole(&self.path, bytes);
        let _ = fs::remove_file(self.lock());
        written
    }
}

/// What the cache keyed by the sources keeps for a program (decisions
/// AU40 and AU43).
pub(crate) struct Sources {
    /// Every file the compile read, with the content hash of what it
    /// read, or looked for and did not find; a hit needs them all as they
    /// were.
    pub reads: Vec<Read>,
    /// The warnings the compile printed, as `check` renders them; empty
    /// when none.
    pub warnings: String,
    /// The manifest's code hash and dependencies, when the run that
    /// compiled the program wanted a manifest (`record`, `--manifest`).
    pub manifest: Option<Kept>,
    /// The program in the binary encoding of decision AT3.
    pub encoding: Vec<u8>,
}

/// What a run manifest takes from the sources, kept with the program
/// (decision AU40 ii) so that a hit computes nothing from them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Kept {
    /// The content hash of `main`; `None` for a program without one.
    pub code: Option<String>,
    pub dependencies: Vec<Dependency>,
}

impl Sources {
    /// The entry's bytes: the format line, one line of JSON (the reads,
    /// the warnings, the manifest or null), then the encoding.
    fn render(&self) -> Vec<u8> {
        let reads = self
            .reads
            .iter()
            .map(|read| {
                Json::Object(vec![
                    ("path".to_string(), Json::Text(read.path.clone())),
                    (
                        "hash".to_string(),
                        read.hash.clone().map_or(Json::Null, Json::Text),
                    ),
                ])
            })
            .collect();
        let manifest = match &self.manifest {
            None => Json::Null,
            Some(kept) => Json::Object(vec![
                (
                    "code".to_string(),
                    kept.code.clone().map_or(Json::Null, Json::Text),
                ),
                (
                    "dependencies".to_string(),
                    Json::Array(
                        kept.dependencies
                            .iter()
                            .map(|dependency| {
                                Json::Object(vec![
                                    ("name".to_string(), Json::Text(dependency.name.clone())),
                                    (
                                        "version".to_string(),
                                        Json::Text(dependency.version.clone()),
                                    ),
                                    ("hash".to_string(), Json::Text(dependency.hash.clone())),
                                ])
                            })
                            .collect(),
                    ),
                ),
            ]),
        };
        let line = Json::Object(vec![
            ("reads".to_string(), Json::Array(reads)),
            ("warnings".to_string(), Json::Text(self.warnings.clone())),
            ("manifest".to_string(), manifest),
        ]);
        let mut text = format!("{SOURCES_FORMAT}\n");
        write_json(&line, &mut text, None, 0);
        text.push('\n');
        let mut bytes = text.into_bytes();
        bytes.extend_from_slice(&self.encoding);
        bytes
    }

    /// The entry an entry's bytes hold; `None` when they are not one.
    fn parse(bytes: &[u8]) -> Option<Sources> {
        let format = bytes.iter().position(|byte| *byte == b'\n')?;
        if &bytes[..format] != SOURCES_FORMAT.as_bytes() {
            return None;
        }
        let rest = &bytes[format + 1..];
        let line = rest.iter().position(|byte| *byte == b'\n')?;
        let json = read_json(std::str::from_utf8(&rest[..line]).ok()?).ok()?;
        let Json::Object(fields) = json else {
            return None;
        };
        let field = |name: &str| fields.iter().find(|(key, _)| key == name).map(|(_, v)| v);
        let text = |value: &Json| match value {
            Json::Text(text) => Some(text.clone()),
            _ => None,
        };
        let optional = |value: &Json| match value {
            Json::Null => Some(None),
            Json::Text(text) => Some(Some(text.clone())),
            _ => None,
        };
        let Json::Array(reads) = field("reads")? else {
            return None;
        };
        let reads = reads
            .iter()
            .map(|read| {
                let Json::Object(fields) = read else {
                    return None;
                };
                let field = |name: &str| fields.iter().find(|(key, _)| key == name).map(|(_, v)| v);
                Some(Read {
                    path: text(field("path")?)?,
                    hash: optional(field("hash")?)?,
                })
            })
            .collect::<Option<Vec<Read>>>()?;
        let warnings = text(field("warnings")?)?;
        let manifest = match field("manifest")? {
            Json::Null => None,
            Json::Object(fields) => {
                let field = |name: &str| fields.iter().find(|(key, _)| key == name).map(|(_, v)| v);
                let Json::Array(dependencies) = field("dependencies")? else {
                    return None;
                };
                let dependencies = dependencies
                    .iter()
                    .map(|dependency| {
                        let Json::Object(fields) = dependency else {
                            return None;
                        };
                        let field =
                            |name: &str| fields.iter().find(|(key, _)| key == name).map(|(_, v)| v);
                        Some(Dependency {
                            name: text(field("name")?)?,
                            version: text(field("version")?)?,
                            hash: text(field("hash")?)?,
                        })
                    })
                    .collect::<Option<Vec<Dependency>>>()?;
                Some(Kept {
                    code: optional(field("code")?)?,
                    dependencies,
                })
            }
            _ => return None,
        };
        Some(Sources {
            reads,
            warnings,
            manifest,
            encoding: rest[line + 1..].to_vec(),
        })
    }
}

/// A program's place in the cache keyed by the sources: the file its
/// entry is kept in.
pub(crate) struct SourceEntry {
    path: PathBuf,
}

impl SourceEntry {
    /// The entry of the program at `path`, as given, compiled by this
    /// binary (its version and its executable's path, size and
    /// modification time, so that a build with the same version and
    /// another front end misses) with its library in the working
    /// directory, in a directory.
    pub(crate) fn of(path: &str, library: &Library, directory: &Path) -> SourceEntry {
        let mut named = format!("{SOURCES_FORMAT}\n{}\n", image::this_renyi());
        // a build with the same version and another front end must miss:
        // the executable's path, size and modification time
        let executable = std::env::current_exe().ok();
        let meta = executable.as_ref().and_then(|exe| fs::metadata(exe).ok());
        let modified = meta
            .as_ref()
            .and_then(|meta| meta.modified().ok())
            .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |since| since.as_nanos());
        let _ = writeln!(
            named,
            "{}\n{} {modified}",
            executable.map_or_else(String::new, |exe| exe.display().to_string()),
            meta.map_or(0, |meta| meta.len())
        );
        // the library's declaration files, the extensions' among them, by
        // name and size: their text is compiled into the executable, which
        // the lines above identify, and hashing it at every run would cost
        // more than the front end a hit skips
        for (module, source) in library.modules() {
            let _ = writeln!(named, "{module} {}", source.len());
        }
        let working = std::env::current_dir()
            .map(|directory| directory.display().to_string())
            .unwrap_or_default();
        let _ = writeln!(named, "{working}\n{path}");
        SourceEntry {
            path: directory.join(name_of(&named, SOURCES_EXTENSION)),
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// What the cache holds for the program, when every file the compile
    /// read reads the same now and, when a manifest is wanted, the entry
    /// has one; the hit marks it recently used.
    pub(crate) fn lookup(&self, manifest: bool) -> Option<Sources> {
        let bytes = fs::read(&self.path).ok()?;
        let sources = Sources::parse(&bytes)?;
        if (manifest && sources.manifest.is_none()) || !renyi_package::unchanged(&sources.reads) {
            return None;
        }
        touch(&self.path);
        Some(sources)
    }

    /// The entry written: to a file of its own first, then renamed, so
    /// that a run never reads half an entry; the cache pruned to its
    /// limit.
    pub(crate) fn store(&self, sources: &Sources) -> Result<(), String> {
        write_whole(&self.path, &sources.render())
    }
}

/// The bytes written to an entry's file: to a file of its own first,
/// then renamed, so that a run never reads half an entry; the cache
/// pruned to its limit after.
fn write_whole(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let directory = path
        .parent()
        .ok_or_else(|| "the cache has no directory".to_string())?;
    fs::create_dir_all(directory)
        .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
    let partial = path.with_extension(format!("{}.part", std::process::id()));
    let written = fs::write(&partial, bytes)
        .and_then(|()| fs::rename(&partial, path))
        .map_err(|error| format!("cannot write {}: {error}", path.display()));
    if written.is_err() {
        let _ = fs::remove_file(&partial);
    }
    written?;
    prune(directory, path);
    Ok(())
}

/// The file's modification time set to now: what the pruning orders by.
fn touch(path: &Path) {
    if let Ok(file) = File::options().write(true).open(path) {
        let _ = file.set_modified(SystemTime::now());
    }
}

/// The least recently used entries, images and programs alike, removed
/// until the rest fit the limit, the one just stored kept whatever its
/// size.
fn prune(directory: &Path, kept: &Path) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    let mut files: Vec<(SystemTime, u64, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let is_entry = path.extension().is_some_and(|extension| {
                extension == image::EXTENSION || extension == SOURCES_EXTENSION
            });
            if !is_entry || path == kept {
                return None;
            }
            let meta = entry.metadata().ok()?;
            Some((meta.modified().ok()?, meta.len(), path))
        })
        .collect();
    let kept_size = fs::metadata(kept).map(|meta| meta.len()).unwrap_or(0);
    let mut total: u64 = kept_size + files.iter().map(|(_, size, _)| size).sum::<u64>();
    files.sort();
    for (_, size, path) in files {
        if total <= LIMIT {
            break;
        }
        if fs::remove_file(&path).is_ok() {
            total -= size;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_entry_of_the_sources_renders_and_parses_back() {
        let sources = Sources {
            reads: vec![
                Read {
                    path: "examples/hello.ry".to_string(),
                    hash: Some("sha256:00".to_string()),
                },
                Read {
                    path: "renyi.json".to_string(),
                    hash: None,
                },
            ],
            warnings: "examples/hello.ry:3:1: warning: \"quoted\"\n".to_string(),
            manifest: Some(Kept {
                code: Some("sha256:ab".to_string()),
                dependencies: vec![Dependency {
                    name: "greeting".to_string(),
                    version: "1.0.0".to_string(),
                    hash: "sha256:cd".to_string(),
                }],
            }),
            encoding: vec![0, 1, 2, b'\n', 255],
        };
        let bytes = sources.render();
        assert!(bytes.starts_with(b"renyi sources 1\n{\"reads\":["));
        let parsed = Sources::parse(&bytes).expect("the entry parses");
        assert_eq!(parsed.reads, sources.reads);
        assert_eq!(parsed.warnings, sources.warnings);
        assert_eq!(parsed.manifest, sources.manifest);
        assert_eq!(parsed.encoding, sources.encoding);
        // without a manifest, and with no warning
        let plain = Sources {
            reads: Vec::new(),
            warnings: String::new(),
            manifest: None,
            encoding: Vec::new(),
        };
        let parsed = Sources::parse(&plain.render()).expect("the entry parses");
        assert!(parsed.manifest.is_none() && parsed.encoding.is_empty());
        // not an entry
        assert!(Sources::parse(b"RYI\0").is_none());
        assert!(Sources::parse(b"renyi sources 1\n").is_none());
        assert!(Sources::parse(b"renyi sources 2\n{}\n").is_none());
        assert!(Sources::parse(b"renyi sources 1\n{\"reads\":[]}\n").is_none());
    }
}
