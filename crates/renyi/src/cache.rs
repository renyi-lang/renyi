//! The image cache of `renyi run` (decision AU10): a program whose run
//! compiled machine code gets an image (decision AS1) in a directory of
//! the user's, built in the background once the run is over by `renyi
//! build --cache`, and every later run of the same program on the same
//! `renyi` and the same machine loads its code from there and compiles
//! nothing.
//!
//! An entry is named by a hash of this `renyi`'s version, the machine's
//! target and the program's binary encoding (decision AT3), and a hit is
//! an image whose header fits this machine and whose program is, byte for
//! byte, the program the run compiled: a file changed, corrupted or
//! written for another program is a miss, never a different program run.
//! The cache is off under `RENYI_NO_CACHE` and `--no-cache`, and for a
//! run on the interpreter; `RENYI_CACHE_DIR` names its directory.

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

use renyi_vm::native::image::{self, Image};
use renyi_vm::native::Jit;
use renyi_vm::recording::sha256_of;
use renyi_vm::Program;

/// What the images of the cache may take together; the least recently
/// used go first when a new one would exceed it.
const LIMIT: u64 = 256 * 1024 * 1024;

/// How long a build in the background is waited for before another run
/// starts its own: a lock older than this is a build that died.
const LOCK_LIFE: Duration = Duration::from_secs(600);

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
        let target = Jit::host_target()?;
        let encoding = renyi_vm::binary::encode(program);
        let named = format!(
            "{}\n{}\n{}",
            image::this_renyi(),
            target,
            sha256_of(&encoding)
        );
        let hash = sha256_of(named.as_bytes());
        let key = hash.trim_start_matches("sha256:");
        let name = format!("{}.{}", &key[..32.min(key.len())], image::EXTENSION);
        Some(Entry {
            path: directory.join(name),
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
        let directory = self
            .path
            .parent()
            .ok_or_else(|| "the cache has no directory".to_string())?;
        fs::create_dir_all(directory)
            .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
        let partial = self
            .path
            .with_extension(format!("{}.part", std::process::id()));
        let written = fs::write(&partial, bytes)
            .and_then(|()| fs::rename(&partial, &self.path))
            .map_err(|error| format!("cannot write {}: {error}", self.path.display()));
        if written.is_err() {
            let _ = fs::remove_file(&partial);
        }
        let _ = fs::remove_file(self.lock());
        written?;
        prune(directory, &self.path);
        Ok(())
    }
}

/// The file's modification time set to now: what the pruning orders by.
fn touch(path: &Path) {
    if let Ok(file) = File::options().write(true).open(path) {
        let _ = file.set_modified(SystemTime::now());
    }
}

/// The least recently used images removed until the rest fit the limit,
/// the one just stored kept whatever its size.
fn prune(directory: &Path, kept: &Path) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    let mut images: Vec<(SystemTime, u64, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let is_image = path
                .extension()
                .is_some_and(|extension| extension == image::EXTENSION);
            if !is_image || path == kept {
                return None;
            }
            let meta = entry.metadata().ok()?;
            Some((meta.modified().ok()?, meta.len(), path))
        })
        .collect();
    let kept_size = fs::metadata(kept).map(|meta| meta.len()).unwrap_or(0);
    let mut total: u64 = kept_size + images.iter().map(|(_, size, _)| size).sum::<u64>();
    images.sort();
    for (_, size, path) in images {
        if total <= LIMIT {
            break;
        }
        if fs::remove_file(&path).is_ok() {
            total -= size;
        }
    }
}
