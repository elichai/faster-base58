//! Requested allocation layouts only; this probe must never supply timing or size results.

#[path = "../../resource-profile/src/allocator.rs"]
mod allocator;

use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

#[global_allocator]
static ALLOCATOR: allocator::Counting = allocator::Counting;

const METRICS: usize = allocator::NAMES.len() + 3;
static BEFORE: AtomicUsize = AtomicUsize::new(0);
static SNAPSHOT: [AtomicUsize; METRICS] = [const { AtomicUsize::new(0) }; METRICS];

#[unsafe(no_mangle)]
pub extern "C" fn size_count_self_check() {
    allocator::self_check();
}

#[unsafe(no_mangle)]
pub extern "C" fn size_count_metrics() -> usize {
    METRICS
}

/// One active window per instance. Caller buffers are allocated outside the window.
#[unsafe(no_mangle)]
pub extern "C" fn size_count_begin() {
    BEFORE.store(allocator::begin(), Relaxed);
}

#[unsafe(no_mangle)]
pub extern "C" fn size_count_end() {
    let stats = allocator::end(BEFORE.load(Relaxed));
    for (index, value) in stats.counters.into_iter().enumerate() {
        SNAPSHOT[index].store(value, Relaxed);
    }
    for (offset, value) in [stats.live_before, stats.peak, stats.live_after]
        .into_iter()
        .enumerate()
    {
        SNAPSHOT[allocator::NAMES.len() + offset].store(value, Relaxed);
    }
}

/// Indices 0 through 9 follow allocator::NAMES, then live_before, peak and live_after.
#[unsafe(no_mangle)]
pub extern "C" fn size_count_get(index: usize) -> usize {
    SNAPSHOT[index].load(Relaxed)
}
