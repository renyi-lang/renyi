//! The image of a program (decision AS1): its bytecode with the machine
//! code of every code object, written by `renyi build` and loaded by a run
//! in place of compiling. The generated code holds no address (it reaches
//! the VM's helpers and tables through the VM pointer), so the image is a
//! plain block of bytes placed in executable memory at load; its header
//! names the `renyi` that built it, the format of the generated code and
//! the target with its CPU features, and a run refuses an image that does
//! not match, naming the fix: build again.
//!
//! The file: the magic `RYI` and a zero byte, the image format, the
//! header (the renyi version, the code format, the target, the
//! optimisation level), the hash of the program's bytecode file, the
//! program in the binary encoding of `binary.rs` (decision AT3), then
//! per code object a presence byte and, when present, where its body and
//! its trampoline lie in the code section, the loop headers and the
//! deopt points (each with the handled regions open there, format 7);
//! then the section's offset and length, padding to
//! `SECTION_ALIGN`, and the section itself, every body and trampoline
//! sixteen-aligned in it. The section is mapped executable straight from
//! the file where the system allows it (decision AT5), so that a run
//! copies no machine code and faults in only the pages it runs; where it
//! does not (a `noexec` mount), the section is read and placed as the
//! JIT places what it compiles. Every number is little-endian; a text and
//! a block of bytes carry their length first.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use cranelift_codegen::isa::TargetIsa;

use super::infer::{Abs, SlotKind};
use super::{DeoptPoint, Jit};
use crate::compile::Program;
use crate::extension::Registry;

/// The first four bytes of an image file.
pub const MAGIC: &[u8; 4] = b"RYI\0";

/// The layout of the file: bumped when it changes. 3: the code section
/// (decision AT5); 7: the handled regions of a deopt point (decision
/// AU18).
pub const IMAGE_FORMAT: u32 = 7;

/// The alignment of the code section in the file: a multiple of every
/// page size the toolchain runs on (16 KB on Apple silicon), so that the
/// section maps executable from the file at any of them.
pub const SECTION_ALIGN: usize = 16384;

/// The alignment of a body or a trampoline in the section.
const PART_ALIGN: usize = 16;

/// What the generated code assumes of the VM that runs it: the layout
/// of a value, of the VM's native state and of a frame, the order of the
/// helpers and their signatures, the statuses. Bumped whenever one of
/// them changes, so that an image of another `renyi` is refused even
/// when the version is the same (a development build). 2: decision AT2
/// (the record's tag word, the callee's prologue writes its locals, no
/// call counter in the state, `rt_direct_entry` gone from the helpers).
/// 3: `rt_retain_at` among the helpers (decision AT6). 4: `rt_take_field`
/// and `rt_with_slot` among the helpers (decision AU11). 5: `rt_compare`
/// among them (decision AU13). 6: `rt_osr` among them and the state's
/// `hotness` (decision AU19). 7: `rt_finish_frame`, `rt_abandon_frame`,
/// `rt_promote` and `rt_clear_slots` among them and the state's
/// `entries` (decision AU21). 8: the VM's variants without fields read in
/// place (decision AU24). 9: the texts held in the value, compared in
/// place (decision AU26).
pub const CODE_FORMAT: u32 = 9;

/// The extension of an image file.
pub const EXTENSION: &str = "ryi";

/// Whether a path names an image file.
pub fn is_image(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .is_some_and(|extension| extension == EXTENSION)
}

/// The `renyi` version an image names: this crate's.
pub fn this_renyi() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The target a Cranelift ISA names: the triple and every ISA flag, so
/// that an image built with one set of CPU features loads only where the
/// same set was detected.
pub fn target_of(isa: &dyn TargetIsa) -> String {
    let mut flags: Vec<String> = isa
        .isa_flags()
        .iter()
        .map(|flag| format!("{}={}", flag.name, flag.value_string()))
        .collect();
    flags.sort();
    format!("{} [{}]", isa.triple(), flags.join(" "))
}

/// The image of a program: every code object compiled at the
/// optimisation level (`speed` by default, decision AS3: a build can
/// afford Cranelift's optimiser, which measured below `none` on the
/// self-check; `none` on request) for this machine; `Err` when Cranelift
/// generates no code here.
pub fn build(
    program: &Program,
    opt_level: Option<&str>,
    registry: &Registry,
) -> Result<Image, String> {
    let calls = crate::vm::call_kinds(program, registry);
    // the machine code of the program as the VM runs it, with the last
    // reads of the slots as moves (decision AU17); the image carries the
    // program as the emitters wrote it
    let prepared = crate::liveness::prepared(program);
    let mut jit = Jit::new(&prepared, Some(opt_level.unwrap_or("speed")), calls)
        .ok_or_else(|| "this machine generates no machine code".to_string())?;
    let compiled = jit.compile_everything(&prepared);
    // the section: every body and trampoline in order, sixteen-aligned
    let mut section = Vec::new();
    let mut place = |bytes: &[u8]| -> Placement {
        let offset = section.len().div_ceil(PART_ALIGN) * PART_ALIGN;
        section.resize(offset, 0);
        section.extend_from_slice(bytes);
        Placement {
            offset: offset as u32,
            len: bytes.len() as u32,
        }
    };
    let codes = compiled
        .into_iter()
        .map(|compiled| {
            compiled.map(|compiled| ImageCode {
                body: place(&compiled.body),
                trampoline: place(&compiled.trampoline),
                headers: compiled.headers,
                deopts: compiled.deopts,
            })
        })
        .collect();
    let section_len = section.len();
    Ok(Image {
        header: Header {
            renyi: this_renyi().to_string(),
            code_format: CODE_FORMAT,
            target: jit.target(),
            opt_level: jit.opt_level().to_string(),
        },
        code_hash: crate::recording::sha256_of(crate::file::render(program).as_bytes()),
        program: crate::binary::encode(program),
        codes,
        section,
        section_offset: 0,
        section_len,
        mapped: None,
    })
}

/// What an image was built by and for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub renyi: String,
    pub code_format: u32,
    pub target: String,
    pub opt_level: String,
}

impl Header {
    /// Why this image cannot run here, with the fix; `None` when it can.
    pub fn mismatch(&self, target: &str) -> Option<String> {
        let here = format!(
            "renyi {} (code format {}) on {}",
            this_renyi(),
            CODE_FORMAT,
            target
        );
        let there = format!(
            "renyi {} (code format {}) for {}",
            self.renyi, self.code_format, self.target
        );
        if here == there.replacen(" for ", " on ", 1) {
            return None;
        }
        Some(format!(
            "the image was built by {there}; this is {here}; run `renyi build` again on this machine"
        ))
    }
}

/// Where a body or a trampoline lies in the code section.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    pub offset: u32,
    pub len: u32,
}

/// The machine code of one code object: where its body and its
/// trampoline lie in the section, with what the VM keeps beside them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageCode {
    pub body: Placement,
    pub trampoline: Placement,
    pub headers: Vec<u32>,
    pub deopts: Vec<DeoptPoint>,
}

/// The code section as it lies in the image's file (decision AT5): the
/// file and the section's offset in it, a multiple of the page size, so
/// that the JIT maps it executable without a copy.
#[derive(Debug)]
pub struct MappedSection {
    pub file: File,
    pub offset: u64,
}

/// An image: the header, the hash of the program's bytecode file (the
/// code hash a run manifest names, so that a recording made from the
/// `.ryc` or from the image reproduces against either), the program in
/// the binary encoding of `binary.rs` (decision AT3), per code object
/// its machine code's place in the section (`None` for one the analysis
/// left to the interpreter), and the section: its bytes when the image
/// was built or read into memory, else the file to map it from.
#[derive(Debug)]
pub struct Image {
    pub header: Header,
    pub code_hash: String,
    pub program: Vec<u8>,
    pub codes: Vec<Option<ImageCode>>,
    /// The section's bytes; empty when `mapped` names the file instead.
    pub section: Vec<u8>,
    /// Where `write` puts the section in the file, a multiple of
    /// `SECTION_ALIGN` (zero before the image is written).
    pub section_offset: usize,
    pub section_len: usize,
    pub mapped: Option<MappedSection>,
}

impl Image {
    /// The image in a file, its code section to be mapped from the file
    /// where the page size allows (else read into memory); the error
    /// says what is wrong with the file, not whether it matches this
    /// machine (`Header::mismatch`).
    pub fn open(path: &str) -> Result<Image, String> {
        let file = File::open(path).map_err(|error| format!("cannot read {path}: {error}"))?;
        let length = file
            .metadata()
            .map_err(|error| format!("cannot read {path}: {error}"))?
            .len();
        let length = usize::try_from(length).map_err(|_| "the image is too large".to_string())?;
        Image::open_at(file, 0, length)
    }

    /// The image that lies at `offset` in a file (an executable's,
    /// decision AS4), `length` bytes long.
    pub fn open_at(file: File, offset: u64, length: usize) -> Result<Image, String> {
        // SAFETY: the mapping is read-only and private, and the bytes are
        // read as an image: a file changed underneath would be a corrupt
        // image, which the checks on reading refuse or the program's own
        // checks catch, as with any file read.
        let mapping = unsafe {
            memmap2::MmapOptions::new()
                .offset(offset)
                .len(length)
                .map(&file)
        };
        match mapping {
            Ok(mapping) => {
                let mut image = Image::read_in(&mapping, false)?;
                let section_at = offset + image.section_offset as u64;
                if section_at.is_multiple_of(region::page::size() as u64) {
                    image.mapped = Some(MappedSection {
                        file,
                        offset: section_at,
                    });
                } else {
                    let section = &mapping[image.section_offset..][..image.section_len];
                    image.section = section.to_vec();
                }
                Ok(image)
            }
            Err(_) => {
                // a file that cannot be mapped is read whole
                let mut bytes = vec![0u8; length];
                let mut reader = &file;
                reader
                    .seek(SeekFrom::Start(offset))
                    .and_then(|_| reader.read_exact(&mut bytes))
                    .map_err(|error| format!("cannot read the image: {error}"))?;
                Image::read(&bytes)
            }
        }
    }

    /// The file's bytes.
    pub fn write(&self) -> Vec<u8> {
        let mut out = Writer::default();
        out.bytes.extend_from_slice(MAGIC);
        out.u32(IMAGE_FORMAT);
        out.text(&self.header.renyi);
        out.u32(self.header.code_format);
        out.text(&self.header.target);
        out.text(&self.header.opt_level);
        out.text(&self.code_hash);
        out.block(&self.program);
        out.u32(self.codes.len() as u32);
        for code in &self.codes {
            match code {
                None => out.u8(0),
                Some(code) => {
                    out.u8(1);
                    out.u32(code.body.offset);
                    out.u32(code.body.len);
                    out.u32(code.trampoline.offset);
                    out.u32(code.trampoline.len);
                    out.u32(code.headers.len() as u32);
                    for header in &code.headers {
                        out.u32(*header);
                    }
                    out.u32(code.deopts.len() as u32);
                    for point in &code.deopts {
                        out.u32(point.pc);
                        out.u16(point.locals);
                        out.u32(point.stack.len() as u32);
                        for abs in &point.stack {
                            out.u8(abs.code());
                        }
                        out.u32(point.slots.len() as u32);
                        for kind in &point.slots {
                            out.u8(kind.code());
                        }
                        out.u32(point.marks.len() as u32);
                        for mark in &point.marks {
                            match mark {
                                None => out.u8(0),
                                Some(depth) => {
                                    out.u8(1);
                                    out.u32(*depth as u32);
                                }
                            }
                        }
                        out.u32(point.handlers.len() as u32);
                        for (target, depth) in &point.handlers {
                            out.u32(*target);
                            out.u32(*depth as u32);
                        }
                    }
                }
            }
        }
        // the section, at the alignment that maps from the file
        let offset = (out.bytes.len() + 8).div_ceil(SECTION_ALIGN) * SECTION_ALIGN;
        out.u32(offset as u32);
        out.u32(self.section.len() as u32);
        out.bytes.resize(offset, 0);
        out.bytes.extend_from_slice(&self.section);
        out.bytes
    }

    /// An image from a file's bytes, its section read into memory; the
    /// error says what is wrong with the file (not whether it matches
    /// this machine: `Header::mismatch`).
    pub fn read(bytes: &[u8]) -> Result<Image, String> {
        Image::read_in(bytes, true)
    }

    /// `read`, with the section's bytes copied or left to `open`.
    fn read_in(bytes: &[u8], copy_section: bool) -> Result<Image, String> {
        let mut input = Reader { bytes, at: 0 };
        if input.take(4)? != MAGIC {
            return Err("not an image file (the magic is missing)".to_string());
        }
        let format = input.u32()?;
        if format != IMAGE_FORMAT {
            return Err(format!(
                "image format {format}; this renyi reads format {IMAGE_FORMAT}: run `renyi build` again"
            ));
        }
        let header = Header {
            renyi: input.text()?,
            code_format: input.u32()?,
            target: input.text()?,
            opt_level: input.text()?,
        };
        let code_hash = input.text()?;
        let program = input.block()?.to_vec();
        let count = input.u32()? as usize;
        let mut codes = Vec::with_capacity(count.min(1 << 16));
        for _ in 0..count {
            if input.u8()? == 0 {
                codes.push(None);
                continue;
            }
            let body = Placement {
                offset: input.u32()?,
                len: input.u32()?,
            };
            let trampoline = Placement {
                offset: input.u32()?,
                len: input.u32()?,
            };
            let headers = (0..input.u32()?)
                .map(|_| input.u32())
                .collect::<Result<Vec<u32>, String>>()?;
            let deopt_count = input.u32()? as usize;
            let mut deopts = Vec::with_capacity(deopt_count.min(1 << 16));
            for _ in 0..deopt_count {
                let pc = input.u32()?;
                let locals = input.u16()?;
                let stack = (0..input.u32()?)
                    .map(|_| {
                        let code = input.u8()?;
                        Abs::from_code(code).ok_or_else(|| format!("unknown operand kind {code}"))
                    })
                    .collect::<Result<Vec<Abs>, String>>()?;
                let slots = (0..input.u32()?)
                    .map(|_| {
                        let code = input.u8()?;
                        SlotKind::from_code(code).ok_or_else(|| format!("unknown slot kind {code}"))
                    })
                    .collect::<Result<Vec<SlotKind>, String>>()?;
                let marks = (0..input.u32()?)
                    .map(|_| {
                        Ok(if input.u8()? == 0 {
                            None
                        } else {
                            Some(input.u32()? as usize)
                        })
                    })
                    .collect::<Result<Vec<Option<usize>>, String>>()?;
                let handlers = (0..input.u32()?)
                    .map(|_| Ok((input.u32()?, input.u32()? as usize)))
                    .collect::<Result<Vec<(u32, usize)>, String>>()?;
                deopts.push(DeoptPoint {
                    pc,
                    locals,
                    stack,
                    slots,
                    marks,
                    handlers,
                });
            }
            codes.push(Some(ImageCode {
                body,
                trampoline,
                headers,
                deopts,
            }));
        }
        let section_offset = input.u32()? as usize;
        let section_len = input.u32()? as usize;
        if !section_offset.is_multiple_of(SECTION_ALIGN) || section_offset < input.at {
            return Err("the code section is misplaced".to_string());
        }
        if section_offset.saturating_add(section_len) != bytes.len() {
            return Err("the image is truncated".to_string());
        }
        for code in codes.iter().flatten() {
            for part in [code.body, code.trampoline] {
                let end = (part.offset as usize).saturating_add(part.len as usize);
                if end > section_len {
                    return Err("a code object lies outside the code section".to_string());
                }
            }
        }
        let section = if copy_section {
            bytes[section_offset..].to_vec()
        } else {
            Vec::new()
        };
        Ok(Image {
            header,
            code_hash,
            program,
            codes,
            section,
            section_offset,
            section_len,
            mapped: None,
        })
    }
}

#[derive(Default)]
struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn block(&mut self, bytes: &[u8]) {
        self.u32(bytes.len() as u32);
        self.bytes.extend_from_slice(bytes);
    }

    fn text(&mut self, text: &str) {
        self.block(text.as_bytes());
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self
            .at
            .checked_add(count)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| "the image is truncated".to_string())?;
        let taken = &self.bytes[self.at..end];
        self.at = end;
        Ok(taken)
    }

    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, String> {
        let bytes = self.take(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&mut self) -> Result<u32, String> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn block(&mut self) -> Result<&'a [u8], String> {
        let length = self.u32()? as usize;
        self.take(length)
    }

    fn text(&mut self) -> Result<String, String> {
        String::from_utf8(self.block()?.to_vec()).map_err(|_| "a text is not UTF-8".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_image_round_trips() {
        let image = Image {
            header: Header {
                // another renyi than this one: the mismatch below names it
                renyi: "0.0.1".to_string(),
                code_format: CODE_FORMAT,
                target: "x86_64 [a=1]".to_string(),
                opt_level: "none".to_string(),
            },
            code_hash: "abc".to_string(),
            program: vec![1, 0],
            codes: vec![
                None,
                Some(ImageCode {
                    body: Placement { offset: 0, len: 1 },
                    trampoline: Placement { offset: 16, len: 2 },
                    headers: vec![7],
                    deopts: vec![DeoptPoint {
                        pc: 3,
                        locals: 2,
                        stack: vec![Abs::Int, Abs::Boxed(None)],
                        slots: vec![SlotKind::Mark, SlotKind::Boxed(None)],
                        marks: vec![Some(1), None],
                        handlers: vec![(9, 1)],
                    }],
                }),
            ],
            section: {
                let mut section = vec![0xc3];
                section.resize(16, 0);
                section.extend_from_slice(&[0x90, 0xc3]);
                section
            },
            section_offset: 0,
            section_len: 18,
            mapped: None,
        };
        let bytes = image.write();
        let read = Image::read(&bytes).unwrap();
        assert_eq!(read.header, image.header);
        assert_eq!(read.code_hash, image.code_hash);
        assert_eq!(read.program, image.program);
        assert_eq!(read.codes, image.codes);
        assert_eq!(read.section, image.section);
        assert_eq!(read.section_offset, SECTION_ALIGN);
        assert_eq!(read.section_len, 18);
        assert_eq!(read.write(), bytes);
        assert!(Image::read(&bytes[..bytes.len() - 1]).is_err());
        assert!(Image::read(b"nope").is_err());
        let mismatch = image.header.mismatch("x86_64 [a=1]").unwrap();
        assert!(mismatch.contains("run `renyi build` again"), "{mismatch}");
        let here = Header {
            renyi: this_renyi().to_string(),
            code_format: CODE_FORMAT,
            target: "x86_64 [a=1]".to_string(),
            opt_level: "speed".to_string(),
        };
        assert_eq!(here.mismatch("x86_64 [a=1]"), None);
        assert!(here.mismatch("x86_64 [a=2]").is_some());
    }
}
