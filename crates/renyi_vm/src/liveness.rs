//! The last read of a slot as a move (decision AU17): a `Load` of a slot
//! that no path reads again before the slot is written or the frame ends
//! becomes a `LoadMove`, so that the value reaches its consumer held once
//! and an update in place (decisions O1 and AU11) finds it so. The VM does
//! this to a copy of the program before it runs it or builds its image:
//! the bytecode file and the image's program stay what the emitters wrote,
//! as the machine code is the VM's own business (decision AG1).
//!
//! The liveness is the usual backward one over the ops: a slot is live
//! out of an op when some successor reads it before writing it. The
//! successors are the next op unless the op leaves (`Jump`, `Return`,
//! `ReturnNothing`, `Fail`, `Crash`), the target of a jump, the exit of an
//! `IterNext`, and, for every op between a `PushHandler` and its target,
//! that target: a failure inside the handled region lands there with the
//! slots as they are, so what the fallback reads stays live through the
//! region, whichever op fails.

use crate::bytecode::{Code, Op};
use crate::compile::Program;

/// The program with the last reads of its slots as moves.
pub fn prepared(program: &Program) -> Program {
    let mut prepared = program.clone();
    for code in &mut prepared.codes {
        move_last_reads(code);
    }
    prepared
}

/// Every `Load` of the code that is the last read of its slot on every
/// path from it replaced by `LoadMove`.
pub fn move_last_reads(code: &mut Code) {
    let count = code.ops.len();
    let slots = code.locals as usize;
    if count == 0 || slots == 0 {
        return;
    }
    let words = slots.div_ceil(64);
    // the targets of the handled regions each op lies in
    let mut handlers: Vec<Vec<usize>> = vec![Vec::new(); count];
    for (at, op) in code.ops.iter().enumerate() {
        if let Op::PushHandler(target) = op {
            let target = *target as usize;
            for inside in handlers.iter_mut().take(target.min(count)).skip(at + 1) {
                inside.push(target);
            }
        }
    }
    let successors = |pc: usize| -> Vec<usize> {
        let mut next = Vec::with_capacity(3);
        let op = &code.ops[pc];
        let leaves = matches!(
            op,
            Op::Jump(_) | Op::Return | Op::ReturnNothing | Op::Fail | Op::Crash
        );
        if !leaves && pc + 1 < count {
            next.push(pc + 1);
        }
        let target = match op {
            Op::Jump(target)
            | Op::JumpIfFalse(target)
            | Op::JumpIfTrue(target)
            | Op::JumpIfAbsent(target)
            | Op::JumpIfFailure(target) => Some(*target as usize),
            Op::IterNext { exit, .. } => Some(*exit as usize),
            _ => None,
        };
        if let Some(target) = target {
            if target < count {
                next.push(target);
            }
        }
        next.extend(handlers[pc].iter().copied());
        next
    };
    // what each op reads and writes
    let uses_and_defs = |op: &Op| -> (Option<u16>, Option<u16>) {
        match op {
            Op::Load(slot)
            | Op::LoadMove(slot)
            | Op::LoadField { slot, .. }
            | Op::TakeField { slot, .. }
            | Op::WithSlot { slot, .. }
            | Op::IterNext { slot, .. }
            | Op::CheckDeadline(slot)
            | Op::UnwindStack(slot) => (Some(*slot), None),
            Op::Store(slot) | Op::IterInit(slot) | Op::Deadline(slot) | Op::MarkStack(slot) => {
                (None, Some(*slot))
            }
            _ => (None, None),
        }
    };
    let bit = |slot: u16| -> (usize, u64) { (slot as usize / 64, 1u64 << (slot as usize % 64)) };
    let mut live_in = vec![0u64; count * words];
    let mut out = vec![0u64; words];
    loop {
        let mut changed = false;
        for pc in (0..count).rev() {
            out.iter_mut().for_each(|word| *word = 0);
            for next in successors(pc) {
                for word in 0..words {
                    out[word] |= live_in[next * words + word];
                }
            }
            let (used, defined) = uses_and_defs(&code.ops[pc]);
            if let Some(slot) = defined {
                let (word, mask) = bit(slot);
                out[word] &= !mask;
            }
            if let Some(slot) = used {
                let (word, mask) = bit(slot);
                out[word] |= mask;
            }
            let own = &mut live_in[pc * words..(pc + 1) * words];
            if own != out.as_slice() {
                own.copy_from_slice(&out);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let moves: Vec<(usize, u16)> = (0..count)
        .filter_map(|pc| {
            let Op::Load(slot) = code.ops[pc] else {
                return None;
            };
            let (word, mask) = bit(slot);
            let live_out = successors(pc)
                .into_iter()
                .any(|next| live_in[next * words + word] & mask != 0);
            (!live_out).then_some((pc, slot))
        })
        .collect();
    for (pc, slot) in moves {
        code.ops[pc] = Op::LoadMove(slot);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::CodeKind;

    fn code(locals: u16, ops: Vec<Op>) -> Code {
        let mut code = Code::new("demo", 0, CodeKind::Function);
        code.locals = locals;
        for op in ops {
            code.emit(op, renyi_syntax::Span::new(0, 0));
        }
        code
    }

    fn moved(code: &Code) -> Vec<usize> {
        code.ops
            .iter()
            .enumerate()
            .filter(|(_, op)| matches!(op, Op::LoadMove(_)))
            .map(|(pc, _)| pc)
            .collect()
    }

    #[test]
    fn the_last_read_on_a_straight_line_moves_and_an_earlier_one_stays() {
        let mut code = code(
            2,
            vec![
                Op::Load(0),
                Op::Store(1),
                Op::Load(0),
                Op::Load(1),
                Op::Return,
            ],
        );
        move_last_reads(&mut code);
        assert_eq!(moved(&code), vec![2, 3]);
    }

    #[test]
    fn a_read_inside_a_loop_stays_live_through_the_back_edge() {
        // 0: Load 0; 1: Pop; 2: JumpIfTrue 0 (reads a Boolean pushed
        // before, elided here); 3: Load 0; 4: Return
        let mut code = code(
            1,
            vec![
                Op::Load(0),
                Op::Pop,
                Op::JumpIfTrue(0),
                Op::Load(0),
                Op::Return,
            ],
        );
        move_last_reads(&mut code);
        assert_eq!(moved(&code), vec![3]);
    }

    #[test]
    fn a_read_inside_a_handled_region_stays_live_for_the_fallback() {
        // 0: PushHandler 5; 1: Load 0; 2: Call; 3: PopHandler; 4: Jump 7;
        // 5: Pop; 6: Load 0; 7: Return
        let mut code = code(
            1,
            vec![
                Op::PushHandler(5),
                Op::Load(0),
                Op::Call {
                    function: 0,
                    args: 1,
                },
                Op::PopHandler,
                Op::Jump(7),
                Op::Pop,
                Op::Load(0),
                Op::Return,
            ],
        );
        move_last_reads(&mut code);
        assert_eq!(moved(&code), vec![6]);
    }

    #[test]
    fn a_store_on_every_path_ends_the_life_of_the_old_value() {
        // 0: Load 0; 1: JumpIfFalse 4; 2: Const 0; 3: Store 0; 4: Load 0;
        // 5: Return: the first read is the last before a store on one
        // path and a read on the other, so it stays; the read at 4 moves
        let mut code = code(
            1,
            vec![
                Op::Load(0),
                Op::JumpIfFalse(4),
                Op::Const(0),
                Op::Store(0),
                Op::Load(0),
                Op::Return,
            ],
        );
        move_last_reads(&mut code);
        assert_eq!(moved(&code), vec![4]);
    }

    #[test]
    fn an_iterator_slot_stays_live_across_its_loop() {
        // 0: IterInit 0; 1: IterNext 0 exit 5; 2: Store 1; 3: Load 1;
        // 4: Jump 1; 5: Return
        let mut code = code(
            2,
            vec![
                Op::IterInit(0),
                Op::IterNext { slot: 0, exit: 5 },
                Op::Store(1),
                Op::Load(1),
                Op::Jump(1),
                Op::Return,
            ],
        );
        move_last_reads(&mut code);
        assert_eq!(moved(&code), vec![3]);
    }
}
