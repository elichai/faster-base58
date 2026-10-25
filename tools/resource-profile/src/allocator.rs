//! Requested Rust layout bytes, not allocator usable bytes or RSS.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};

pub struct Counting;

static ACTIVE: AtomicBool = AtomicBool::new(false);
static INVALID: AtomicBool = AtomicBool::new(false);
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static COUNTERS: [AtomicUsize; 10] = [const { AtomicUsize::new(0) }; 10];

pub const NAMES: [&str; 10] = [
    "alloc_calls", "alloc_bytes", "zeroed_calls", "zeroed_bytes", "realloc_calls",
    "realloc_old_bytes", "realloc_new_bytes", "dealloc_calls", "dealloc_bytes", "failed_calls",
];

#[derive(Debug)]
pub struct Stats {
    pub counters: [usize; 10],
    pub live_before: usize,
    pub peak: usize,
    pub live_after: usize,
}

fn add(counter: &AtomicUsize, n: usize) -> usize {
    match counter.try_update(Relaxed, Relaxed, |v| v.checked_add(n)) {
        Ok(old) => old + n,
        Err(old) => {
            INVALID.store(true, Relaxed);
            old
        }
    }
}

fn sub_live(n: usize) {
    if LIVE.try_update(Relaxed, Relaxed, |v| v.checked_sub(n)).is_err() {
        INVALID.store(true, Relaxed);
    }
}

fn added_live(n: usize) {
    let live = add(&LIVE, n);
    if ACTIVE.load(Relaxed) {
        PEAK.fetch_max(live, Relaxed);
    }
}

fn count(index: usize, n: usize) {
    add(&COUNTERS[index], n);
}

// No allocator callback locks, formats, allocates, or panics. Windows are entered only
// by this single-threaded worker, and every allocation's live size is tracked always.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ACTIVE.load(Relaxed) {
            count(0, 1);
            count(1, layout.size());
        }
        let p = unsafe { System.alloc(layout) };
        if p.is_null() {
            if ACTIVE.load(Relaxed) { count(9, 1); }
        } else {
            added_live(layout.size());
        }
        p
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if ACTIVE.load(Relaxed) {
            count(0, 1);
            count(1, layout.size());
            count(2, 1);
            count(3, layout.size());
        }
        let p = unsafe { System.alloc_zeroed(layout) };
        if p.is_null() {
            if ACTIVE.load(Relaxed) { count(9, 1); }
        } else {
            added_live(layout.size());
        }
        p
    }

    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        if ACTIVE.load(Relaxed) {
            count(7, 1);
            count(8, layout.size());
        }
        unsafe { System.dealloc(p, layout) };
        sub_live(layout.size());
    }

    unsafe fn realloc(&self, p: *mut u8, old: Layout, new_size: usize) -> *mut u8 {
        if ACTIVE.load(Relaxed) {
            count(4, 1);
            count(5, old.size());
            count(6, new_size);
        }
        let q = unsafe { System.realloc(p, old, new_size) };
        if q.is_null() {
            if ACTIVE.load(Relaxed) { count(9, 1); }
        } else if new_size >= old.size() {
            added_live(new_size - old.size());
        } else {
            sub_live(old.size() - new_size);
        }
        q
    }
}

pub fn begin() -> usize {
    assert!(!ACTIVE.load(Relaxed), "nested allocation window");
    assert!(!INVALID.load(Relaxed), "allocation counter overflow/underflow");
    for counter in &COUNTERS { counter.store(0, Relaxed); }
    let before = LIVE.load(Relaxed);
    PEAK.store(before, Relaxed);
    ACTIVE.store(true, Relaxed);
    before
}

pub fn end(before: usize) -> Stats {
    assert!(ACTIVE.swap(false, Relaxed), "no allocation window");
    assert!(!INVALID.load(Relaxed), "allocation counter overflow/underflow");
    Stats {
        counters: std::array::from_fn(|i| COUNTERS[i].load(Relaxed)),
        live_before: before,
        peak: PEAK.load(Relaxed),
        live_after: LIVE.load(Relaxed),
    }
}

pub fn measure<T>(f: impl FnOnce() -> T) -> (T, Stats) {
    let before = begin();
    let result = f();
    (result, end(before))
}

pub fn self_check() {
    let a = Layout::from_size_align(16, 8).unwrap();
    let b = Layout::from_size_align(8, 8).unwrap();
    let before = begin();
    // Direct allocator calls keep the checks independent of Vec growth policy.
    unsafe {
        let p = Counting.alloc(a);
        let q = Counting.alloc_zeroed(b);
        assert!(!p.is_null() && !q.is_null());
        let p = Counting.realloc(p, a, 32);
        assert!(!p.is_null());
        Counting.dealloc(q, b);
        Counting.dealloc(p, Layout::from_size_align(32, 8).unwrap());
    }
    let stats = end(before);
    assert_eq!(stats.counters, [2, 24, 1, 8, 1, 16, 32, 2, 40, 0]);
    assert_eq!(stats.peak - stats.live_before, 40);
    assert_eq!(stats.live_before, stats.live_after);

    let p = unsafe { Counting.alloc(a) };
    assert!(!p.is_null());
    let (_, stats) = measure(|| unsafe { Counting.dealloc(p, a) });
    assert_eq!(stats.live_before - stats.live_after, 16);
    assert_eq!(stats.counters[7..], [1, 16, 0]);
}
