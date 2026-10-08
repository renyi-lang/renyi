//! The memory budget of a run (decisions Q3 and AP1): a counting
//! allocator wrapped around the process's, which counts the bytes a run
//! holds above the level at its start while a budget is in force, and
//! the flag the VM reads at its safe points (a call, a primitive, a loop
//! turning). Without a budget in force the counting costs one load per
//! allocation. One budget at a time per process: the VM is one thread,
//! and a host runs one sandboxed call at a time.

use std::alloc::{GlobalAlloc, Layout};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

/// Set by the first allocation through `Counting`: whether the process's
/// allocator counts at all.
static INSTALLED: AtomicBool = AtomicBool::new(false);
/// Whether a budget is in force.
static ACTIVE: AtomicBool = AtomicBool::new(false);
/// The bytes allocated minus the bytes freed since `begin`; negative when
/// the run frees more than it allocates.
static NET: AtomicI64 = AtomicI64::new(0);
/// The highest `NET` since `begin`.
static PEAK: AtomicI64 = AtomicI64::new(0);
static LIMIT: AtomicI64 = AtomicI64::new(0);
/// Whether `NET` went over `LIMIT` since `begin`.
static OVER: AtomicBool = AtomicBool::new(false);

/// A global allocator that counts for the memory budget, around any
/// other: `#[global_allocator] static ALLOCATOR: Counting<System> =
/// Counting(System);`. The `renyi` crate wraps mimalloc in it as
/// `renyi::Allocator`.
pub struct Counting<A>(pub A);

unsafe impl<A: GlobalAlloc> GlobalAlloc for Counting<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = self.0.alloc(layout);
        noted(layout.size() as i64);
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = self.0.alloc_zeroed(layout);
        noted(layout.size() as i64);
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        self.0.dealloc(pointer, layout);
        freed(layout.size() as i64);
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let moved = self.0.realloc(pointer, layout, new_size);
        if !moved.is_null() {
            let delta = new_size as i64 - layout.size() as i64;
            if delta >= 0 {
                noted(delta);
            } else {
                freed(-delta);
            }
        }
        moved
    }
}

#[inline]
fn noted(size: i64) {
    if !INSTALLED.load(Ordering::Relaxed) {
        INSTALLED.store(true, Ordering::Relaxed);
    }
    if ACTIVE.load(Ordering::Relaxed) {
        let net = NET.fetch_add(size, Ordering::Relaxed) + size;
        PEAK.fetch_max(net, Ordering::Relaxed);
        if net > LIMIT.load(Ordering::Relaxed) {
            OVER.store(true, Ordering::Relaxed);
        }
    }
}

#[inline]
fn freed(size: i64) {
    if ACTIVE.load(Ordering::Relaxed) {
        NET.fetch_sub(size, Ordering::Relaxed);
    }
}

/// Whether the process allocates through `Counting`: a budget is
/// enforced only then.
pub fn installed() -> bool {
    INSTALLED.load(Ordering::Relaxed)
}

/// Start counting against `limit` bytes above the level now; `false` when
/// a budget is in force already.
pub(crate) fn begin(limit: u64) -> bool {
    if ACTIVE.load(Ordering::Relaxed) {
        return false;
    }
    NET.store(0, Ordering::Relaxed);
    PEAK.store(0, Ordering::Relaxed);
    OVER.store(false, Ordering::Relaxed);
    LIMIT.store(limit.min(i64::MAX as u64) as i64, Ordering::Relaxed);
    ACTIVE
        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
}

/// Stop counting; the peak of the run in bytes.
pub(crate) fn end() -> u64 {
    ACTIVE.store(false, Ordering::Relaxed);
    peak()
}

/// Whether the budget was exceeded since `begin`.
#[inline]
pub(crate) fn over() -> bool {
    OVER.load(Ordering::Relaxed)
}

/// The most bytes held above the level at `begin`.
pub(crate) fn peak() -> u64 {
    PEAK.load(Ordering::Relaxed).max(0) as u64
}
