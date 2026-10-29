//! Process-wide bounded twiddle retention, present only with `cache`.

use super::Tables;
use std::sync::OnceLock;

/// Largest transform log kept for the process lifetime; larger tables are per call.
/// This limits retention without changing the transforms supported by [`super::MAX_LOG`].
pub(crate) const CACHE_MAX_LOG: u32 = 18;

static CACHE: [OnceLock<Tables>; CACHE_MAX_LOG as usize + 1] =
    [const { OnceLock::new() }; CACHE_MAX_LOG as usize + 1];

impl Tables {
    /// Tables serving at least `2^log` points, from a process-wide cache. A smaller table
    /// is a prefix of a larger one, so any larger cached table serves; a size is built
    /// only when none is cached yet, and then kept. Requires `1 <= log <= CACHE_MAX_LOG`.
    /// All slots together hold at most `48 * (2^(CACHE_MAX_LOG + 1) - 2)` data bytes.
    pub fn cached(log: u32) -> &'static Tables {
        assert!(
            (1..=CACHE_MAX_LOG).contains(&log),
            "uncacheable log = {log}"
        );
        let log = log as usize;
        CACHE[log..]
            .iter()
            .find_map(OnceLock::get)
            .unwrap_or_else(|| CACHE[log].get_or_init(|| Tables::new(log as u32)))
    }
}

/// Static header bytes and (log, initialized, vector length bytes, vector capacity
/// bytes), without initializing a table.
#[cfg(feature = "__internals")]
#[must_use]
pub fn cache_memory() -> (
    usize,
    [(u32, bool, usize, usize); super::MAX_LOG as usize + 1],
) {
    let rows = core::array::from_fn(|log| match CACHE.get(log).and_then(OnceLock::get) {
        Some(t) => {
            let vectors =
                t.tw.iter()
                    .flat_map(|tw| [&tw.fw, &tw.fwq, &tw.iw, &tw.iwq]);
            let (len, cap) = vectors.fold((0, 0), |(len, cap), v| {
                (
                    len + v.len() * size_of::<i32>(),
                    cap + v.capacity() * size_of::<i32>(),
                )
            });
            (log as u32, true, len, cap)
        }
        None => (log as u32, false, 0, 0),
    });
    (size_of_val(&CACHE), rows)
}
