//! The files a compile reads, noted for the cache keyed by the sources
//! (decisions AU40 and AU43): a command that wants to know what a compile
//! depended on runs it inside `noting`, and every file read through
//! `read_text` is noted with the content hash of what was read, or as not
//! there, which covers the manifests looked for in the directories above a
//! program and the `.ry` file looked for before a `.renyi` one. `unchanged`
//! answers whether every file noted would read the same now.

use std::cell::RefCell;

use crate::registry::hash_of;

/// A file read, or looked for and not there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Read {
    /// The path as it was read: relative to the working directory or not.
    pub path: String,
    /// The content hash of the text read; `None` for a file that could not
    /// be read as text, missing most often.
    pub hash: Option<String>,
}

thread_local! {
    static NOTED: RefCell<Option<Vec<Read>>> = const { RefCell::new(None) };
}

/// Run `work` with every file it reads through `read_text` noted, each path
/// once, in the order first read; a `noting` inside another notes its
/// reads for both.
pub fn noting<T>(work: impl FnOnce() -> T) -> (T, Vec<Read>) {
    let outer = NOTED.with(|noted| noted.replace(Some(Vec::new())));
    let value = work();
    let reads = NOTED.with(|noted| noted.replace(outer)).unwrap_or_default();
    for read in &reads {
        note(read);
    }
    (value, reads)
}

/// A file's text, the read noted when a caller of `noting` asks for it.
pub fn read_text(path: &str) -> std::io::Result<String> {
    let text = std::fs::read_to_string(path);
    note(&Read {
        path: path.to_string(),
        hash: text.as_ref().ok().map(|text| hash_of(text.as_bytes())),
    });
    text
}

fn note(read: &Read) {
    NOTED.with(|noted| {
        if let Some(reads) = noted.borrow_mut().as_mut() {
            if !reads.iter().any(|known| known.path == read.path) {
                reads.push(read.clone());
            }
        }
    });
}

/// Whether every file noted reads as it did: the same text where there
/// was one, still nothing where there was none.
pub fn unchanged(reads: &[Read]) -> bool {
    reads.iter().all(|read| {
        let now = std::fs::read_to_string(&read.path)
            .ok()
            .map(|text| hash_of(text.as_bytes()));
        now == read.hash
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_read_is_noted_once_with_its_hash_and_a_missing_file_as_none() {
        let directory = std::env::temp_dir().join(format!("renyi-reads-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("a scratch directory");
        let present = directory.join("present.ry").to_string_lossy().to_string();
        let missing = directory.join("missing.ry").to_string_lossy().to_string();
        std::fs::write(&present, "module present\n").expect("the file written");
        let ((), reads) = noting(|| {
            let _ = read_text(&present);
            let _ = read_text(&missing);
            let _ = read_text(&present);
        });
        assert_eq!(reads.len(), 2);
        assert_eq!(reads[0].path, present);
        assert_eq!(reads[0].hash, Some(hash_of(b"module present\n")));
        assert_eq!(
            reads[1],
            Read {
                path: missing.clone(),
                hash: None
            }
        );
        assert!(unchanged(&reads));
        // a file changed, or one that appears, is no longer as noted
        std::fs::write(&present, "module present\n\n").expect("the file changed");
        assert!(!unchanged(&reads));
        std::fs::write(&present, "module present\n").expect("the file restored");
        std::fs::write(&missing, "module missing\n").expect("the file made");
        assert!(!unchanged(&reads));
        // nothing is noted outside `noting`
        let ((), inner) = noting(|| {});
        assert!(inner.is_empty());
        let _ = std::fs::remove_dir_all(&directory);
    }
}
