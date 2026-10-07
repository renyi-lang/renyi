//! The profiler of `renyi run --profile` (decision X4). A timer thread
//! raises a flag every half millisecond; the interpreter loop takes the
//! flag down before it runs an operation and gives the sample to the
//! operation that ran last, and a primitive takes the flag down when it
//! returns, so that its time lands on the primitive. Every operation,
//! call and primitive call is counted. The report goes to the standard
//! error when the run ends.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use renyi_check::FunctionId;

use crate::bytecode::Op;
use crate::compile::{CodeId, Program};

/// How often the timer thread raises the flag.
pub const PERIOD: Duration = Duration::from_micros(500);

/// How many rows a table of the report shows.
const ROWS: usize = 20;

/// Raised by the timer thread, taken down by the interpreter.
static SAMPLE: AtomicBool = AtomicBool::new(false);
/// Whether the timer thread keeps running.
static TIMER: AtomicBool = AtomicBool::new(false);

/// Where a sample lands: an operation kind of a code object, or a
/// primitive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Site {
    Op(CodeId, usize),
    Primitive(FunctionId),
}

pub struct Profile {
    started: Instant,
    /// The operation that ran last: where the next sample lands.
    last: Option<Site>,
    samples: HashMap<Site, u64>,
    /// Per operation kind (`Op::kind`): its name once seen, and the count.
    ops: Vec<(&'static str, u64)>,
    calls: HashMap<CodeId, u64>,
    primitives: HashMap<FunctionId, u64>,
}

impl Profile {
    /// Start counting; the timer thread starts with the first profile.
    pub fn start() -> Profile {
        if !TIMER.swap(true, Ordering::SeqCst) {
            std::thread::spawn(|| {
                while TIMER.load(Ordering::Relaxed) {
                    std::thread::sleep(PERIOD);
                    SAMPLE.store(true, Ordering::Relaxed);
                }
            });
        }
        Profile {
            started: Instant::now(),
            last: None,
            samples: HashMap::new(),
            ops: vec![("", 0); Op::KINDS],
            calls: HashMap::new(),
            primitives: HashMap::new(),
        }
    }

    /// An operation of the code object is about to run; a raised flag is
    /// the sample of the operation that ran last.
    #[inline]
    pub fn op(&mut self, code: CodeId, op: &Op) {
        let (kind, name) = op.kind();
        let slot = &mut self.ops[kind];
        slot.0 = name;
        slot.1 += 1;
        if SAMPLE.swap(false, Ordering::Relaxed) {
            if let Some(site) = self.last {
                *self.samples.entry(site).or_insert(0) += 1;
            }
        }
        self.last = Some(Site::Op(code, kind));
    }

    /// A code object was entered.
    pub fn call(&mut self, code: CodeId) {
        *self.calls.entry(code).or_insert(0) += 1;
    }

    /// A primitive returned; a raised flag is its sample.
    pub fn primitive(&mut self, function: FunctionId) {
        *self.primitives.entry(function).or_insert(0) += 1;
        if SAMPLE.swap(false, Ordering::Relaxed) {
            *self.samples.entry(Site::Primitive(function)).or_insert(0) += 1;
        }
    }

    /// The report; the timer thread stops.
    pub fn render(self, program: &Program) -> String {
        TIMER.store(false, Ordering::SeqCst);
        let total_ops: u64 = self.ops.iter().map(|(_, count)| count).sum();
        let total_calls: u64 = self.calls.values().sum();
        let total_primitives: u64 = self.primitives.values().sum();
        let total_samples: u64 = self.samples.values().sum();
        let mut out = String::new();
        let _ = writeln!(
            out,
            "profile: {} operations, {} calls, {} primitive calls, {} samples in {:.2} s",
            commas(total_ops),
            commas(total_calls),
            commas(total_primitives),
            commas(total_samples),
            self.started.elapsed().as_secs_f64()
        );
        let code_name = |code: CodeId| {
            let code = &program.codes[code];
            format!("{}.{}", program.module_names[code.module], code.name)
        };
        let primitive_name = |function: FunctionId| {
            let meta = &program.function_metas[function];
            format!("primitive {}.{}", meta.module, meta.name)
        };
        let by_site: Vec<(String, u64)> = self
            .samples
            .iter()
            .map(|(site, count)| {
                let name = match site {
                    Site::Op(code, kind) => format!("{} {}", code_name(*code), self.ops[*kind].0),
                    Site::Primitive(function) => primitive_name(*function),
                };
                (name, *count)
            })
            .collect();
        table(
            &mut out,
            "samples by function and operation",
            by_site,
            total_samples,
        );
        let mut by_function: HashMap<String, u64> = HashMap::new();
        let mut by_kind: HashMap<String, u64> = HashMap::new();
        for (site, count) in &self.samples {
            let (function, kind) = match site {
                Site::Op(code, kind) => (code_name(*code), self.ops[*kind].0.to_string()),
                Site::Primitive(function) => (primitive_name(*function), primitive_name(*function)),
            };
            *by_function.entry(function).or_insert(0) += count;
            *by_kind.entry(kind).or_insert(0) += count;
        }
        table(
            &mut out,
            "samples by function",
            by_function.into_iter().collect(),
            total_samples,
        );
        table(
            &mut out,
            "samples by operation",
            by_kind.into_iter().collect(),
            total_samples,
        );
        let ops = self
            .ops
            .iter()
            .filter(|(_, count)| *count > 0)
            .map(|(name, count)| (name.to_string(), *count))
            .collect();
        table(&mut out, "operations by kind", ops, total_ops);
        let calls = self
            .calls
            .iter()
            .map(|(code, count)| (code_name(*code), *count))
            .collect();
        table(&mut out, "calls by function", calls, total_calls);
        let primitives = self
            .primitives
            .iter()
            .map(|(function, count)| (primitive_name(*function), *count))
            .collect();
        table(&mut out, "primitive calls", primitives, total_primitives);
        out
    }
}

/// One table of the report: the rows with the largest counts, each with
/// its share of the total.
fn table(out: &mut String, title: &str, mut rows: Vec<(String, u64)>, total: u64) {
    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    if rows.len() > ROWS {
        let _ = writeln!(out, "{title} (the top {ROWS} of {})", rows.len());
    } else {
        let _ = writeln!(out, "{title}");
    }
    for (name, count) in rows.iter().take(ROWS) {
        let share = if total == 0 {
            0.0
        } else {
            *count as f64 * 100.0 / total as f64
        };
        let _ = writeln!(out, "  {:>15} {share:5.1}%  {name}", commas(*count));
    }
}

/// A count with a comma every three digits.
fn commas(count: u64) -> String {
    let digits = count.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::commas;

    #[test]
    fn counts_are_grouped_by_thousands() {
        assert_eq!(commas(0), "0");
        assert_eq!(commas(999), "999");
        assert_eq!(commas(1000), "1,000");
        assert_eq!(commas(93_217_004), "93,217,004");
    }
}
