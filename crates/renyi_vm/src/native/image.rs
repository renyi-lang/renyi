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
//! optimisation level), the bytecode as the `.ryc` text, then per code
//! object a presence byte and, when present, the body, the trampoline,
//! the loop headers and the deopt points. Every number is little-endian;
//! a text and a block of bytes carry their length first.

use cranelift_codegen::isa::TargetIsa;

use super::infer::{Abs, SlotKind};
use super::{DeoptPoint, Jit};
use crate::compile::Program;

/// The first four bytes of an image file.
pub const MAGIC: &[u8; 4] = b"RYI\0";

/// The layout of the file: bumped when it changes.
pub const IMAGE_FORMAT: u32 = 2;

/// What the generated code assumes of the VM that runs it: the layout
/// of a value, of the VM's native state and of a frame, the order of the
/// helpers and their signatures, the statuses. Bumped whenever one of
/// them changes, so that an image of another `renyi` is refused even
/// when the version is the same (a development build). 2: decision AT2
/// (the record's tag word, the callee's prologue writes its locals, no
/// call counter in the state, `rt_direct_entry` gone from the helpers).
pub const CODE_FORMAT: u32 = 2;

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
pub fn build(program: &Program, opt_level: Option<&str>) -> Result<Image, String> {
    let mut jit = Jit::new(program, Some(opt_level.unwrap_or("speed")))
        .ok_or_else(|| "this machine generates no machine code".to_string())?;
    let codes = jit.compile_everything(program);
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

/// The machine code of one code object, with what the VM keeps beside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageCode {
    pub body: Vec<u8>,
    pub trampoline: Vec<u8>,
    pub headers: Vec<u32>,
    pub deopts: Vec<DeoptPoint>,
}

/// An image: the header, the hash of the program's bytecode file (the
/// code hash a run manifest names, so that a recording made from the
/// `.ryc` or from the image reproduces against either), the program in
/// the binary encoding of `binary.rs` (decision AT3), and per code object
/// its machine code (`None` for one the analysis left to the interpreter).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub header: Header,
    pub code_hash: String,
    pub program: Vec<u8>,
    pub codes: Vec<Option<ImageCode>>,
}

impl Image {
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
                    out.block(&code.body);
                    out.block(&code.trampoline);
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
                    }
                }
            }
        }
        out.bytes
    }

    /// An image from a file's bytes; the error says what is wrong with
    /// the file (not whether it matches this machine: `Header::mismatch`).
    pub fn read(bytes: &[u8]) -> Result<Image, String> {
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
            let body = input.block()?.to_vec();
            let trampoline = input.block()?.to_vec();
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
                deopts.push(DeoptPoint {
                    pc,
                    locals,
                    stack,
                    slots,
                    marks,
                });
            }
            codes.push(Some(ImageCode {
                body,
                trampoline,
                headers,
                deopts,
            }));
        }
        if input.at != bytes.len() {
            return Err("the image has bytes after its end".to_string());
        }
        Ok(Image {
            header,
            code_hash,
            program,
            codes,
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
                    body: vec![0xc3],
                    trampoline: vec![0x90, 0xc3],
                    headers: vec![7],
                    deopts: vec![DeoptPoint {
                        pc: 3,
                        locals: 2,
                        stack: vec![Abs::Int, Abs::Boxed],
                        slots: vec![SlotKind::Mark, SlotKind::Boxed],
                        marks: vec![Some(1), None],
                    }],
                }),
            ],
        };
        let bytes = image.write();
        assert_eq!(Image::read(&bytes).unwrap(), image);
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
