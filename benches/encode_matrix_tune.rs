//! `encode_matrix` against `encode_batched` in one binary.
//!
//! - `cargo bench --bench encode_matrix_tune`: the matrix path at every size (threshold
//!   0), interleaved rounds, the minimum time per call. For tuning `MATRIX_MIN`.
//! - `cargo bench --bench encode_matrix_tune -- cold`: single calls after touching every
//!   line of 64 MiB to evict the caches, the median of 31 calls. The table is built
//!   before. The timer ticks every 41.7 ns on Apple Silicon.
//! - `cargo bench --bench encode_matrix_tune -- first`: the first call in each table size
//!   class, which builds the table, against the second.
//!
//! `SIZES` (comma-separated) replaces the default list.

use std::hint::black_box;
use std::time::Instant;

use base58_bench::bench::encode_matrix_with;
use base58_bench::{encode_batched, max_encoded_len};
use rand_xoshiro::Xoshiro256PlusPlus;
use rand_xoshiro::rand_core::{Rng, SeedableRng};

type Encode = fn(&[u8], &mut [u8]) -> usize;

const CONFIGS: [(&str, Encode); 2] = [
    ("batched", |d, o| encode_batched(d, o).unwrap()),
    ("matrix", |d, o| encode_matrix_with(d, o, 0).unwrap()),
];
const SWEEP: [usize; 14] = [32, 48, 64, 80, 96, 112, 128, 144, 160, 192, 224, 256, 384, 512];
const COLD: [usize; 4] = [512, 1024, 1875, 4096];
const ROUNDS: usize = 31;
/// Target length of one timed block.
const BLOCK_NS: u128 = 2_000_000;

fn data(size: usize) -> Vec<u8> {
    let mut data = vec![0u8; size];
    Xoshiro256PlusPlus::seed_from_u64(size as u64).fill_bytes(&mut data);
    data[0] |= 1;
    data
}

fn main() {
    if std::env::args().skip(1).any(|a| a == "first") {
        return first_calls();
    }
    let cold = std::env::args().skip(1).any(|a| a == "cold");
    let default = if cold { &COLD[..] } else { &SWEEP[..] };
    let sizes: Vec<usize> = std::env::var("SIZES")
        .map_or(default.to_vec(), |s| s.split(',').map(|v| v.parse().unwrap()).collect());
    println!("| bytes | {} | ratio |", CONFIGS.map(|c| c.0).join(" | "));
    println!("|---:|{}---:|", "---:|".repeat(CONFIGS.len()));
    let mut evict = vec![0u8; 64 << 20];
    for size in sizes {
        let data = data(size);
        let mut out = vec![0u8; max_encoded_len(size)];
        let n = CONFIGS[0].1(&data, &mut out);
        let want = out[..n].to_vec();
        for (name, f) in CONFIGS {
            let n = f(&data, &mut out);
            assert!(out[..n] == want[..], "{name}: wrong output at {size} bytes");
        }
        let times = if cold {
            cold_times(&data, &mut out, &mut evict)
        } else {
            warm_times(&data, &mut out)
        };
        let cells: Vec<String> = times.iter().map(|t| format!("{:.3}", t / 1000.0)).collect();
        println!("| {size} | {} | {:.2} |", cells.join(" | "), times[0] / times[1]);
    }
}

/// The first call in each table class against the second, in a fresh process: the first
/// builds the table.
fn first_calls() {
    println!("| bytes | first call (µs) | second call (µs) |");
    println!("|---:|---:|---:|");
    for size in [512, 1024, 2048, 4096] {
        let data = data(size);
        let mut out = vec![0u8; max_encoded_len(size)];
        let times: Vec<f64> = (0..2)
            .map(|_| {
                let start = Instant::now();
                black_box(CONFIGS[1].1(black_box(&data), black_box(&mut out)));
                start.elapsed().as_nanos() as f64 / 1000.0
            })
            .collect();
        println!("| {size} | {:.1} | {:.1} |", times[0], times[1]);
    }
}

/// The minimum time per call of each config over interleaved rounds, in ns.
fn warm_times(data: &[u8], out: &mut [u8]) -> Vec<f64> {
    let iters: Vec<u128> = CONFIGS
        .iter()
        .map(|(_, f)| {
            let start = Instant::now();
            f(data, out);
            (BLOCK_NS / start.elapsed().as_nanos().max(1)).max(1)
        })
        .collect();
    let mut best = vec![f64::INFINITY; CONFIGS.len()];
    for _ in 0..ROUNDS {
        for (i, (_, f)) in CONFIGS.iter().enumerate() {
            let start = Instant::now();
            for _ in 0..iters[i] {
                black_box(f(black_box(data), black_box(out)));
            }
            best[i] = best[i].min(start.elapsed().as_nanos() as f64 / iters[i] as f64);
        }
    }
    best
}

/// The median time of single calls with cold caches, in ns.
fn cold_times(data: &[u8], out: &mut [u8], evict: &mut [u8]) -> Vec<f64> {
    let mut samples = vec![Vec::new(); CONFIGS.len()];
    for round in 0..ROUNDS {
        for (i, (_, f)) in CONFIGS.iter().enumerate() {
            // A load and a store per 128-byte line: a large `fill` may use stores that
            // bypass the caches.
            for line in evict.chunks_exact_mut(128) {
                line[0] = line[0].wrapping_add(round as u8 | 1);
            }
            black_box(&mut *evict);
            let start = Instant::now();
            black_box(f(black_box(data), black_box(out)));
            samples[i].push(start.elapsed().as_nanos() as f64);
        }
    }
    samples
        .iter_mut()
        .map(|s| {
            s.sort_by(f64::total_cmp);
            s[s.len() / 2]
        })
        .collect()
}
