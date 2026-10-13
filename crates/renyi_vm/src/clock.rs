//! The clock of the VM (decision AW29): the milliseconds since the epoch
//! that the budget counters, the `within` deadlines and `std.time` read,
//! the entropy a run seeds its random generator with, and a stopwatch for
//! the elapsed times the reports and the polls measure. On the native
//! targets they are `std`'s. On `wasm32-unknown-unknown` `std`'s clocks
//! panic ("time not implemented on this platform"), so the host hands the
//! time in through one import, `renyi_host_now_ms` (the page's
//! `Date.now()`), which the web crate's script provides.

/// Milliseconds since the Unix epoch.
#[cfg(not(target_arch = "wasm32"))]
pub fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(target_arch = "wasm32")]
extern "C" {
    /// The host's clock: milliseconds since the Unix epoch, as a number.
    fn renyi_host_now_ms() -> f64;
}

/// Milliseconds since the Unix epoch, from the host.
#[cfg(target_arch = "wasm32")]
pub fn now_millis() -> i64 {
    // SAFETY: the import takes nothing and answers a number; the host
    // that instantiates the module provides it, and a module without it
    // does not instantiate.
    unsafe { renyi_host_now_ms() as i64 }
}

/// Entropy for the seed of a run's random generator: the nanoseconds
/// since the epoch, or on wasm32 the milliseconds spread over the word.
pub fn seed_entropy() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15)
    }
    #[cfg(target_arch = "wasm32")]
    {
        (now_millis() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
    }
}

/// A stopwatch: the milliseconds since it was started.
pub struct Stopwatch {
    #[cfg(not(target_arch = "wasm32"))]
    started: std::time::Instant,
    #[cfg(target_arch = "wasm32")]
    started: i64,
}

impl Stopwatch {
    pub fn start() -> Stopwatch {
        Stopwatch {
            #[cfg(not(target_arch = "wasm32"))]
            started: std::time::Instant::now(),
            #[cfg(target_arch = "wasm32")]
            started: now_millis(),
        }
    }

    /// The milliseconds elapsed since the start.
    pub fn elapsed_ms(&self) -> f64 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.started.elapsed().as_secs_f64() * 1000.0
        }
        #[cfg(target_arch = "wasm32")]
        {
            (now_millis() - self.started) as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clock_runs_forward() {
        let before = now_millis();
        let watch = Stopwatch::start();
        assert!(before > 1_600_000_000_000, "a date after 2020");
        assert!(now_millis() >= before);
        assert!(watch.elapsed_ms() >= 0.0);
        assert_ne!(seed_entropy(), 0);
    }
}
