//! A small x86-64 assembler for the template tier (decisions AU18 and
//! AU19): the thirty-nine instruction forms the sequences per op need, and
//! nothing else. The code it makes is position-independent (every jump is
//! relative, every address comes from a register), so what the tier
//! places holds no address of its own, as decision AS1 requires of the
//! generated code.
//! Memory operands are `[base + disp]` with a byte displacement where
//! one fits and a double word otherwise; `rsp` and `r12` as a base take
//! the SIB byte the encoding demands.

/// The sixteen general registers, numbered as the encoding numbers them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Reg {
    Rax = 0,
    Rcx = 1,
    Rdx = 2,
    Rbx = 3,
    Rsp = 4,
    Rbp = 5,
    Rsi = 6,
    Rdi = 7,
    R8 = 8,
    R9 = 9,
    R10 = 10,
    R11 = 11,
    R12 = 12,
    R13 = 13,
    R14 = 14,
    R15 = 15,
}

impl Reg {
    fn low(self) -> u8 {
        self as u8 & 7
    }

    fn high(self) -> bool {
        self as u8 >= 8
    }
}

/// A condition for `jcc` and `setcc`, by its number in the opcode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Cond {
    /// Below (unsigned).
    B = 2,
    /// Above or equal (unsigned).
    Ae = 3,
    E = 4,
    Ne = 5,
    /// Below or equal (unsigned).
    Be = 6,
    /// Above (unsigned).
    A = 7,
    /// Less (signed).
    L = 0xC,
    Ge = 0xD,
    Le = 0xE,
    G = 0xF,
}

/// A place in the code a jump goes to, bound once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Label(usize);

/// The bytes of a function being assembled.
#[derive(Default)]
pub struct Asm {
    bytes: Vec<u8>,
    /// Per label, where it was bound.
    labels: Vec<Option<usize>>,
    /// The relative displacements to patch: where the four bytes lie and
    /// the label they reach for.
    fixups: Vec<(usize, Label)>,
}

/// The calling convention of the helpers: the registers the arguments go
/// in, the bytes the caller reserves below the return address for the
/// callee (the Windows shadow space), and the registers a function keeps.
pub struct Abi {
    pub args: &'static [Reg],
    pub shadow: i32,
    pub callee_saved: &'static [Reg],
}

pub const SYSTEM_V: Abi = Abi {
    args: &[Reg::Rdi, Reg::Rsi, Reg::Rdx, Reg::Rcx, Reg::R8, Reg::R9],
    shadow: 0,
    callee_saved: &[Reg::Rbx, Reg::Rbp, Reg::R12, Reg::R13, Reg::R14, Reg::R15],
};

pub const WINDOWS_X64: Abi = Abi {
    args: &[Reg::Rcx, Reg::Rdx, Reg::R8, Reg::R9],
    shadow: 32,
    callee_saved: &[
        Reg::Rbx,
        Reg::Rbp,
        Reg::Rdi,
        Reg::Rsi,
        Reg::R12,
        Reg::R13,
        Reg::R14,
        Reg::R15,
    ],
};

/// The convention of this host.
pub fn host_abi() -> &'static Abi {
    if cfg!(windows) {
        &WINDOWS_X64
    } else {
        &SYSTEM_V
    }
}

impl Asm {
    pub fn new() -> Asm {
        Asm::default()
    }

    pub fn position(&self) -> usize {
        self.bytes.len()
    }

    pub fn label(&mut self) -> Label {
        self.labels.push(None);
        Label(self.labels.len() - 1)
    }

    /// The label bound here.
    pub fn bind(&mut self, label: Label) {
        debug_assert!(self.labels[label.0].is_none(), "a label is bound once");
        self.labels[label.0] = Some(self.bytes.len());
    }

    pub fn is_bound(&self, label: Label) -> bool {
        self.labels[label.0].is_some()
    }

    /// The bytes, with every jump patched; an error names a label a jump
    /// reaches for that was never bound.
    pub fn finish(mut self) -> Result<Vec<u8>, String> {
        for (at, label) in std::mem::take(&mut self.fixups) {
            let target =
                self.labels[label.0].ok_or_else(|| format!("label {} unbound", label.0))?;
            let rel = target as i64 - (at as i64 + 4);
            let rel = i32::try_from(rel).map_err(|_| "a jump too far".to_string())?;
            self.bytes[at..at + 4].copy_from_slice(&rel.to_le_bytes());
        }
        Ok(self.bytes)
    }

    // ------------------------------------------------------------ pieces

    fn byte(&mut self, byte: u8) {
        self.bytes.push(byte);
    }

    fn imm32(&mut self, imm: i32) {
        self.bytes.extend_from_slice(&imm.to_le_bytes());
    }

    /// The REX prefix when the operation needs one: a 64-bit operand, or
    /// a high register in the reg or the rm field.
    fn rex(&mut self, wide: bool, reg: u8, rm: Reg) {
        let prefix = 0x40 | (wide as u8) << 3 | ((reg >> 3) & 1) << 2 | rm.high() as u8;
        if prefix != 0x40 {
            self.byte(prefix);
        }
    }

    /// ModRM (and SIB) for `[base + disp]` with `reg` in the reg field.
    fn mem(&mut self, reg: u8, base: Reg, disp: i32) {
        let reg = reg & 7;
        let short = (-128..128).contains(&disp);
        let modbits = if short { 0x40 } else { 0x80 };
        self.byte(modbits | reg << 3 | base.low());
        if base.low() == 4 {
            // rsp and r12 need the SIB byte: no index, the base itself
            self.byte(0x24);
        }
        if short {
            self.byte(disp as i8 as u8);
        } else {
            self.imm32(disp);
        }
    }

    /// ModRM for a register operand with `reg` in the reg field.
    fn reg(&mut self, reg: u8, rm: Reg) {
        self.byte(0xC0 | (reg & 7) << 3 | rm.low());
    }

    // ------------------------------------------------------------ moves

    /// `mov dst, src` (64 bits).
    pub fn mov_rr(&mut self, dst: Reg, src: Reg) {
        self.rex(true, src as u8, dst);
        self.byte(0x89);
        self.reg(src as u8, dst);
    }

    /// `mov dst, imm` (64 bits): the shortest form that gives the value.
    pub fn mov_ri(&mut self, dst: Reg, imm: i64) {
        if imm >= 0 && imm <= u32::MAX as i64 {
            // a 32-bit move zero-extends
            self.mov_ri32(dst, imm as u32);
        } else if imm >= i32::MIN as i64 && imm <= i32::MAX as i64 {
            self.rex(true, 0, dst);
            self.byte(0xC7);
            self.reg(0, dst);
            self.imm32(imm as i32);
        } else {
            self.rex(true, 0, dst);
            self.byte(0xB8 | dst.low());
            self.bytes.extend_from_slice(&imm.to_le_bytes());
        }
    }

    /// `mov dst32, imm32`, which zero-extends into the 64-bit register.
    pub fn mov_ri32(&mut self, dst: Reg, imm: u32) {
        self.rex(false, 0, dst);
        self.byte(0xB8 | dst.low());
        self.imm32(imm as i32);
    }

    /// `mov dst, qword [base + disp]`.
    pub fn mov_rm(&mut self, dst: Reg, base: Reg, disp: i32) {
        self.rex(true, dst as u8, base);
        self.byte(0x8B);
        self.mem(dst as u8, base, disp);
    }

    /// `mov qword [base + disp], src`.
    pub fn mov_mr(&mut self, base: Reg, disp: i32, src: Reg) {
        self.rex(true, src as u8, base);
        self.byte(0x89);
        self.mem(src as u8, base, disp);
    }

    /// `mov dst32, dword [base + disp]`, zero-extended.
    pub fn mov_rm32(&mut self, dst: Reg, base: Reg, disp: i32) {
        self.rex(false, dst as u8, base);
        self.byte(0x8B);
        self.mem(dst as u8, base, disp);
    }

    /// `mov dword [base + disp], src32`.
    pub fn mov_mr32(&mut self, base: Reg, disp: i32, src: Reg) {
        self.rex(false, src as u8, base);
        self.byte(0x89);
        self.mem(src as u8, base, disp);
    }

    /// `mov byte [base + disp], src8` (the low byte of the register; a
    /// REX prefix makes `sil`, `dil`, `spl` and `bpl` addressable).
    pub fn mov_m8r(&mut self, base: Reg, disp: i32, src: Reg) {
        let needs_rex =
            src.high() || base.high() || matches!(src, Reg::Rsp | Reg::Rbp | Reg::Rsi | Reg::Rdi);
        if needs_rex {
            self.byte(0x40 | (src.high() as u8) << 2 | base.high() as u8);
        }
        self.byte(0x88);
        self.mem(src as u8, base, disp);
    }

    /// `movzx dst32, byte [base + disp]`.
    pub fn movzx_rm8(&mut self, dst: Reg, base: Reg, disp: i32) {
        self.rex(false, dst as u8, base);
        self.byte(0x0F);
        self.byte(0xB6);
        self.mem(dst as u8, base, disp);
    }

    /// `movzx dst32, src8` (the low byte of `src`: `al` for `rax`).
    pub fn movzx_rr8(&mut self, dst: Reg, src: Reg) {
        let needs_rex =
            dst.high() || src.high() || matches!(src, Reg::Rsp | Reg::Rbp | Reg::Rsi | Reg::Rdi);
        if needs_rex {
            self.byte(0x40 | (dst.high() as u8) << 2 | src.high() as u8);
        }
        self.byte(0x0F);
        self.byte(0xB6);
        self.reg(dst as u8, src);
    }

    /// `mov byte [base + disp], imm8`.
    pub fn mov_m8i(&mut self, base: Reg, disp: i32, imm: u8) {
        self.rex(false, 0, base);
        self.byte(0xC6);
        self.mem(0, base, disp);
        self.byte(imm);
    }

    /// `mov dword [base + disp], imm32`.
    pub fn mov_m32i(&mut self, base: Reg, disp: i32, imm: u32) {
        self.rex(false, 0, base);
        self.byte(0xC7);
        self.mem(0, base, disp);
        self.imm32(imm as i32);
    }

    /// `mov qword [base + disp], imm32` sign-extended.
    pub fn mov_m64i(&mut self, base: Reg, disp: i32, imm: i32) {
        self.rex(true, 0, base);
        self.byte(0xC7);
        self.mem(0, base, disp);
        self.imm32(imm);
    }

    /// `lea dst, [base + disp]`.
    pub fn lea(&mut self, dst: Reg, base: Reg, disp: i32) {
        self.rex(true, dst as u8, base);
        self.byte(0x8D);
        self.mem(dst as u8, base, disp);
    }

    // ------------------------------------------------------------ arithmetic

    fn alu_ri(&mut self, extension: u8, dst: Reg, imm: i32, wide: bool) {
        self.rex(wide, 0, dst);
        if (-128..128).contains(&imm) {
            self.byte(0x83);
            self.reg(extension, dst);
            self.byte(imm as i8 as u8);
        } else {
            self.byte(0x81);
            self.reg(extension, dst);
            self.imm32(imm);
        }
    }

    fn alu_mi(&mut self, extension: u8, base: Reg, disp: i32, imm: i32, wide: bool) {
        self.rex(wide, 0, base);
        if (-128..128).contains(&imm) {
            self.byte(0x83);
            self.mem(extension, base, disp);
            self.byte(imm as i8 as u8);
        } else {
            self.byte(0x81);
            self.mem(extension, base, disp);
            self.imm32(imm);
        }
    }

    /// `add dst, imm` (64 bits).
    pub fn add_ri(&mut self, dst: Reg, imm: i32) {
        self.alu_ri(0, dst, imm, true);
    }

    /// `sub dst, imm` (64 bits).
    pub fn sub_ri(&mut self, dst: Reg, imm: i32) {
        self.alu_ri(5, dst, imm, true);
    }

    /// `add dst, src` (64 bits).
    pub fn add_rr(&mut self, dst: Reg, src: Reg) {
        self.rex(true, src as u8, dst);
        self.byte(0x01);
        self.reg(src as u8, dst);
    }

    /// `add dword [base + disp], imm32`.
    pub fn add_m32i(&mut self, base: Reg, disp: i32, imm: i32) {
        self.alu_mi(0, base, disp, imm, false);
    }

    /// `add qword [base + disp], imm32` sign-extended.
    pub fn add_m64i(&mut self, base: Reg, disp: i32, imm: i32) {
        self.alu_mi(0, base, disp, imm, true);
    }

    /// `sub qword [base + disp], imm32` sign-extended.
    pub fn sub_m64i(&mut self, base: Reg, disp: i32, imm: i32) {
        self.alu_mi(5, base, disp, imm, true);
    }

    /// `cmp a, imm` (64 bits).
    pub fn cmp_ri(&mut self, a: Reg, imm: i32) {
        self.alu_ri(7, a, imm, true);
    }

    /// `cmp a32, imm32`.
    pub fn cmp_r32i(&mut self, a: Reg, imm: i32) {
        self.alu_ri(7, a, imm, false);
    }

    /// `cmp a, b` (64 bits).
    pub fn cmp_rr(&mut self, a: Reg, b: Reg) {
        self.rex(true, b as u8, a);
        self.byte(0x39);
        self.reg(b as u8, a);
    }

    /// `cmp byte [base + disp], imm8`.
    pub fn cmp_m8i(&mut self, base: Reg, disp: i32, imm: u8) {
        self.rex(false, 0, base);
        self.byte(0x80);
        self.mem(7, base, disp);
        self.byte(imm);
    }

    /// `cmp dword [base + disp], imm32`.
    pub fn cmp_m32i(&mut self, base: Reg, disp: i32, imm: i32) {
        self.alu_mi(7, base, disp, imm, false);
    }

    /// `cmp qword [base + disp], imm32` sign-extended.
    pub fn cmp_m64i(&mut self, base: Reg, disp: i32, imm: i32) {
        self.alu_mi(7, base, disp, imm, true);
    }

    /// `test a32, b32`.
    pub fn test_rr32(&mut self, a: Reg, b: Reg) {
        self.rex(false, b as u8, a);
        self.byte(0x85);
        self.reg(b as u8, a);
    }

    /// `test a, b` (64 bits).
    pub fn test_rr(&mut self, a: Reg, b: Reg) {
        self.rex(true, b as u8, a);
        self.byte(0x85);
        self.reg(b as u8, a);
    }

    /// `xor dst32, dst32`: the register zeroed.
    pub fn zero(&mut self, dst: Reg) {
        self.rex(false, dst as u8, dst);
        self.byte(0x31);
        self.reg(dst as u8, dst);
    }

    /// `imul dst, src, imm32` (64 bits).
    pub fn imul_rri(&mut self, dst: Reg, src: Reg, imm: i32) {
        self.rex(true, dst as u8, src);
        self.byte(0x69);
        self.reg(dst as u8, src);
        self.imm32(imm);
    }

    /// `setcc dst8`: the low byte of the register set to the condition.
    pub fn setcc(&mut self, cond: Cond, dst: Reg) {
        let needs_rex = dst.high() || matches!(dst, Reg::Rsp | Reg::Rbp | Reg::Rsi | Reg::Rdi);
        if needs_rex {
            self.byte(0x40 | dst.high() as u8);
        }
        self.byte(0x0F);
        self.byte(0x90 | cond as u8);
        self.reg(0, dst);
    }

    // ------------------------------------------------------------ control

    /// `jcc label` with a 32-bit displacement.
    pub fn jcc(&mut self, cond: Cond, label: Label) {
        self.byte(0x0F);
        self.byte(0x80 | cond as u8);
        self.fixups.push((self.bytes.len(), label));
        self.imm32(0);
    }

    /// `jmp label` with a 32-bit displacement.
    pub fn jmp(&mut self, label: Label) {
        self.byte(0xE9);
        self.fixups.push((self.bytes.len(), label));
        self.imm32(0);
    }

    /// `jmp r`.
    pub fn jmp_r(&mut self, r: Reg) {
        self.rex(false, 0, r);
        self.byte(0xFF);
        self.reg(4, r);
    }

    /// `call r`.
    pub fn call_r(&mut self, r: Reg) {
        self.rex(false, 0, r);
        self.byte(0xFF);
        self.reg(2, r);
    }

    /// `call qword [base + disp]`.
    pub fn call_m(&mut self, base: Reg, disp: i32) {
        self.rex(false, 0, base);
        self.byte(0xFF);
        self.mem(2, base, disp);
    }

    pub fn push(&mut self, r: Reg) {
        if r.high() {
            self.byte(0x41);
        }
        self.byte(0x50 | r.low());
    }

    pub fn pop(&mut self, r: Reg) {
        if r.high() {
            self.byte(0x41);
        }
        self.byte(0x58 | r.low());
    }

    pub fn ret(&mut self) {
        self.byte(0xC3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(build: impl FnOnce(&mut Asm)) -> Vec<u8> {
        let mut asm = Asm::new();
        build(&mut asm);
        asm.finish().expect("assembled")
    }

    #[test]
    fn moves_between_registers_and_memory_encode_as_the_manual_says() {
        assert_eq!(bytes(|a| a.mov_rr(Reg::Rbx, Reg::Rdi)), [0x48, 0x89, 0xFB]);
        assert_eq!(bytes(|a| a.mov_rr(Reg::R12, Reg::Rsi)), [0x49, 0x89, 0xF4]);
        assert_eq!(bytes(|a| a.mov_rr(Reg::Rax, Reg::R13)), [0x4C, 0x89, 0xE8]);
        assert_eq!(
            bytes(|a| a.mov_rm(Reg::Rax, Reg::Rbx, 0x10)),
            [0x48, 0x8B, 0x43, 0x10]
        );
        assert_eq!(
            bytes(|a| a.mov_rm(Reg::Rax, Reg::Rbx, 0x1000)),
            [0x48, 0x8B, 0x83, 0x00, 0x10, 0x00, 0x00]
        );
        assert_eq!(
            bytes(|a| a.mov_mr(Reg::R12, 8, Reg::Rax)),
            [0x49, 0x89, 0x44, 0x24, 0x08]
        );
        assert_eq!(
            bytes(|a| a.mov_mr(Reg::Rsp, 0x40, Reg::R10)),
            [0x4C, 0x89, 0x54, 0x24, 0x40]
        );
        assert_eq!(
            bytes(|a| a.mov_rm(Reg::R13, Reg::R13, 0)),
            [0x4D, 0x8B, 0x6D, 0x00]
        );
        assert_eq!(
            bytes(|a| a.mov_rm32(Reg::Rcx, Reg::Rax, 4)),
            [0x8B, 0x48, 0x04]
        );
        assert_eq!(
            bytes(|a| a.mov_mr32(Reg::R8, -8, Reg::Rdx)),
            [0x41, 0x89, 0x50, 0xF8]
        );
        assert_eq!(
            bytes(|a| a.movzx_rm8(Reg::Rax, Reg::R13, 0)),
            [0x41, 0x0F, 0xB6, 0x45, 0x00]
        );
        assert_eq!(
            bytes(|a| a.movzx_rr8(Reg::Rax, Reg::Rax)),
            [0x0F, 0xB6, 0xC0]
        );
        assert_eq!(
            bytes(|a| a.mov_m8r(Reg::R13, 8, Reg::Rax)),
            [0x41, 0x88, 0x45, 0x08]
        );
        assert_eq!(
            bytes(|a| a.mov_m8i(Reg::R13, 24, 12)),
            [0x41, 0xC6, 0x45, 0x18, 0x0C]
        );
        assert_eq!(
            bytes(|a| a.mov_m32i(Reg::Rsp, 32, 7)),
            [0xC7, 0x44, 0x24, 0x20, 0x07, 0x00, 0x00, 0x00]
        );
        assert_eq!(
            bytes(|a| a.mov_m64i(Reg::Rax, 16, -1)),
            [0x48, 0xC7, 0x40, 0x10, 0xFF, 0xFF, 0xFF, 0xFF]
        );
        assert_eq!(
            bytes(|a| a.mov_ri(Reg::Rdi, 5)),
            [0xBF, 0x05, 0x00, 0x00, 0x00]
        );
        assert_eq!(
            bytes(|a| a.mov_ri(Reg::R9, 5)),
            [0x41, 0xB9, 0x05, 0x00, 0x00, 0x00]
        );
        assert_eq!(
            bytes(|a| a.mov_ri(Reg::Rax, -2)),
            [0x48, 0xC7, 0xC0, 0xFE, 0xFF, 0xFF, 0xFF]
        );
        assert_eq!(
            bytes(|a| a.mov_ri(Reg::R10, 0x1_0000_0000)),
            [0x49, 0xBA, 0, 0, 0, 0, 1, 0, 0, 0]
        );
        assert_eq!(
            bytes(|a| a.lea(Reg::Rdi, Reg::R13, 48)),
            [0x49, 0x8D, 0x7D, 0x30]
        );
        assert_eq!(
            bytes(|a| a.lea(Reg::Rax, Reg::Rsp, 8)),
            [0x48, 0x8D, 0x44, 0x24, 0x08]
        );
    }

    #[test]
    fn arithmetic_comparisons_and_tests_encode_as_the_manual_says() {
        assert_eq!(bytes(|a| a.add_ri(Reg::Rsp, 16)), [0x48, 0x83, 0xC4, 0x10]);
        assert_eq!(
            bytes(|a| a.sub_ri(Reg::Rsp, 0x100)),
            [0x48, 0x81, 0xEC, 0x00, 0x01, 0x00, 0x00]
        );
        assert_eq!(bytes(|a| a.add_rr(Reg::R13, Reg::R15)), [0x4D, 0x01, 0xFD]);
        assert_eq!(
            bytes(|a| a.add_m32i(Reg::Rax, 8, 200)),
            [0x81, 0x40, 0x08, 0xC8, 0x00, 0x00, 0x00]
        );
        assert_eq!(bytes(|a| a.cmp_ri(Reg::Rax, 3)), [0x48, 0x83, 0xF8, 0x03]);
        assert_eq!(bytes(|a| a.cmp_r32i(Reg::Rax, 3)), [0x83, 0xF8, 0x03]);
        assert_eq!(bytes(|a| a.cmp_rr(Reg::Rax, Reg::Rcx)), [0x48, 0x39, 0xC8]);
        assert_eq!(
            bytes(|a| a.cmp_m8i(Reg::R13, 0, 18)),
            [0x41, 0x80, 0x7D, 0x00, 0x12]
        );
        assert_eq!(
            bytes(|a| a.cmp_m32i(Reg::Rax, 4, 0x10000)),
            [0x81, 0x78, 0x04, 0x00, 0x00, 0x01, 0x00]
        );
        assert_eq!(
            bytes(|a| a.cmp_m64i(Reg::Rbx, 16, 0)),
            [0x48, 0x83, 0x7B, 0x10, 0x00]
        );
        assert_eq!(bytes(|a| a.test_rr32(Reg::Rax, Reg::Rax)), [0x85, 0xC0]);
        assert_eq!(bytes(|a| a.test_rr(Reg::R10, Reg::R10)), [0x4D, 0x85, 0xD2]);
        assert_eq!(
            bytes(|a| a.add_m64i(Reg::Rbx, 0x20, 1)),
            [0x48, 0x83, 0x43, 0x20, 0x01]
        );
        assert_eq!(
            bytes(|a| a.sub_m64i(Reg::Rbx, 0x20, 1)),
            [0x48, 0x83, 0x6B, 0x20, 0x01]
        );
        assert_eq!(bytes(|a| a.zero(Reg::Rax)), [0x31, 0xC0]);
        assert_eq!(bytes(|a| a.zero(Reg::R10)), [0x45, 0x31, 0xD2]);
        assert_eq!(
            bytes(|a| a.imul_rri(Reg::R15, Reg::R12, 24)),
            [0x4D, 0x69, 0xFC, 0x18, 0x00, 0x00, 0x00]
        );
        assert_eq!(bytes(|a| a.setcc(Cond::E, Reg::Rax)), [0x0F, 0x94, 0xC0]);
        assert_eq!(
            bytes(|a| a.setcc(Cond::Ne, Reg::Rdi)),
            [0x40, 0x0F, 0x95, 0xC7]
        );
    }

    #[test]
    fn jumps_calls_and_the_stack_encode_as_the_manual_says() {
        assert_eq!(bytes(|a| a.push(Reg::Rbx)), [0x53]);
        assert_eq!(bytes(|a| a.push(Reg::R12)), [0x41, 0x54]);
        assert_eq!(bytes(|a| a.pop(Reg::R15)), [0x41, 0x5F]);
        assert_eq!(bytes(|a| a.ret()), [0xC3]);
        assert_eq!(bytes(|a| a.call_r(Reg::Rax)), [0xFF, 0xD0]);
        assert_eq!(bytes(|a| a.call_r(Reg::R11)), [0x41, 0xFF, 0xD3]);
        assert_eq!(
            bytes(|a| a.call_m(Reg::R14, 0x40)),
            [0x41, 0xFF, 0x56, 0x40]
        );
        assert_eq!(bytes(|a| a.jmp_r(Reg::Rax)), [0xFF, 0xE0]);
        // a forward jump over two bytes, a backward one to the start
        let code = bytes(|a| {
            let start = a.label();
            let over = a.label();
            a.bind(start);
            a.jcc(Cond::Ne, over);
            a.ret();
            a.ret();
            a.bind(over);
            a.jmp(start);
        });
        assert_eq!(
            code,
            [0x0F, 0x85, 2, 0, 0, 0, 0xC3, 0xC3, 0xE9, 0xF3, 0xFF, 0xFF, 0xFF]
        );
    }

    #[test]
    fn a_jump_to_a_label_never_bound_is_refused() {
        let mut asm = Asm::new();
        let nowhere = asm.label();
        asm.jmp(nowhere);
        assert!(asm.finish().is_err());
    }

    /// Every form once, written to the file `RENYI_X64_DUMP` names, for a
    /// look with `objdump -D -b binary -m i386:x86-64` (a development
    /// aid, not a test of anything).
    #[test]
    fn every_form_dumped_on_request() {
        let Some(path) = std::env::var_os("RENYI_X64_DUMP") else {
            return;
        };
        let code = bytes(|a| {
            let l = a.label();
            a.mov_rr(Reg::Rbx, Reg::Rdi);
            a.mov_ri(Reg::R9, 5);
            a.mov_ri(Reg::Rax, -2);
            a.mov_ri(Reg::R10, 0x1_0000_0000);
            a.mov_ri32(Reg::Rsi, 0xFFFF_FFFF);
            a.mov_rm(Reg::Rax, Reg::R12, 0x100);
            a.mov_mr(Reg::Rsp, 0x40, Reg::R10);
            a.mov_rm32(Reg::Rcx, Reg::Rax, 4);
            a.mov_mr32(Reg::R8, -8, Reg::Rdx);
            a.mov_m8r(Reg::R13, 8, Reg::Rax);
            a.mov_m8r(Reg::Rax, 8, Reg::Rsi);
            a.movzx_rm8(Reg::Rax, Reg::R13, 0);
            a.movzx_rr8(Reg::Rax, Reg::Rax);
            a.movzx_rr8(Reg::R10, Reg::Rdi);
            a.mov_m8i(Reg::R13, 24, 12);
            a.mov_m32i(Reg::Rsp, 32, 7);
            a.mov_m64i(Reg::Rax, 16, -1);
            a.lea(Reg::Rdi, Reg::R13, 48);
            a.lea(Reg::Rax, Reg::Rsp, 8);
            a.add_ri(Reg::Rsp, 16);
            a.sub_ri(Reg::Rsp, 0x100);
            a.add_rr(Reg::R13, Reg::R15);
            a.add_m32i(Reg::Rax, 8, 200);
            a.cmp_ri(Reg::Rax, 3);
            a.cmp_r32i(Reg::Rax, 3);
            a.cmp_rr(Reg::Rax, Reg::Rcx);
            a.cmp_m8i(Reg::R13, 0, 18);
            a.cmp_m32i(Reg::Rax, 4, 0x10000);
            a.cmp_m64i(Reg::Rbx, 16, 0);
            a.test_rr32(Reg::Rax, Reg::Rax);
            a.test_rr(Reg::R10, Reg::R10);
            a.add_m64i(Reg::Rbx, 0x20, 1);
            a.sub_m64i(Reg::Rbx, 0x400, 1);
            a.zero(Reg::R10);
            a.imul_rri(Reg::R15, Reg::R12, 24);
            a.setcc(Cond::E, Reg::Rax);
            a.setcc(Cond::Ne, Reg::Rdi);
            a.push(Reg::R12);
            a.pop(Reg::R15);
            a.call_r(Reg::R11);
            a.call_m(Reg::R14, 0x40);
            a.jmp_r(Reg::Rax);
            a.bind(l);
            a.jcc(Cond::Ae, l);
            a.jmp(l);
            a.ret();
        });
        std::fs::write(path, code).expect("the dump written");
    }
}
