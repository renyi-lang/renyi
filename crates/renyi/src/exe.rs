//! The self-contained executable of `renyi build --exe` (decision AS1,
//! its second step; decision AS4): this binary copied with the image
//! appended and a trailer that says where the image lies. At startup the
//! binary looks at its own end and, when the trailer is there, runs the
//! embedded image with the whole command line as the program's
//! arguments, as `renyi run <image> <arguments>` would.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// The last eight bytes of a self-contained executable.
pub const TRAILER_MAGIC: &[u8; 8] = b"RENYIEXE";

/// The trailer: the image's offset and length, then the magic.
pub const TRAILER_LEN: u64 = 24;

/// The image this executable carries, when it carries one: a binary
/// without a trailer costs one open and one read of its last bytes.
pub fn embedded_image() -> Option<Vec<u8>> {
    let path = std::env::current_exe().ok()?;
    let mut file = File::open(&path).ok()?;
    let len = file.metadata().ok()?.len();
    if len < TRAILER_LEN {
        return None;
    }
    file.seek(SeekFrom::End(-(TRAILER_LEN as i64))).ok()?;
    let mut trailer = [0u8; TRAILER_LEN as usize];
    file.read_exact(&mut trailer).ok()?;
    if &trailer[16..24] != TRAILER_MAGIC {
        return None;
    }
    let offset = u64::from_le_bytes(trailer[0..8].try_into().ok()?);
    let length = u64::from_le_bytes(trailer[8..16].try_into().ok()?);
    if offset.checked_add(length)? != len - TRAILER_LEN {
        return None;
    }
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut bytes = vec![0u8; usize::try_from(length).ok()?];
    file.read_exact(&mut bytes).ok()?;
    Some(bytes)
}

/// The executable written: this binary's own bytes (a binary that
/// carried an image would have run it instead of building), the image,
/// the trailer; executable on Unix. The note, when there is one, is a
/// step the system needs by hand before the file runs (the signature on
/// macOS).
pub fn write(target: &Path, image: &[u8]) -> Result<Option<String>, String> {
    let own =
        std::env::current_exe().map_err(|error| format!("cannot find this binary: {error}"))?;
    let mut bytes =
        std::fs::read(&own).map_err(|error| format!("cannot read {}: {error}", own.display()))?;
    let offset = bytes.len() as u64;
    bytes.extend_from_slice(image);
    bytes.extend_from_slice(&offset.to_le_bytes());
    bytes.extend_from_slice(&(image.len() as u64).to_le_bytes());
    bytes.extend_from_slice(TRAILER_MAGIC);
    std::fs::write(target, &bytes)
        .map_err(|error| format!("cannot write {}: {error}", target.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(target)
            .map_err(|error| format!("cannot read {}: {error}", target.display()))?
            .permissions();
        permissions.set_mode(permissions.mode() | 0o111);
        std::fs::set_permissions(target, permissions)
            .map_err(|error| format!("cannot make {} executable: {error}", target.display()))?;
    }
    Ok(sign(target))
}

/// macOS runs only signed executables, an ad hoc signature being enough,
/// and the copy's signature no longer covers the file: signed again here
/// when `codesign` is at hand, else the command to run is the note.
#[cfg(target_os = "macos")]
fn sign(target: &Path) -> Option<String> {
    let signed = std::process::Command::new("codesign")
        .args(["--force", "-s", "-"])
        .arg(target)
        .output();
    match signed {
        Ok(output) if output.status.success() => None,
        _ => Some(format!(
            "sign it before running it: codesign --force -s - {}",
            target.display()
        )),
    }
}

#[cfg(not(target_os = "macos"))]
fn sign(_target: &Path) -> Option<String> {
    None
}
