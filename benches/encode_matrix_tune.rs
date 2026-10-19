//! `encode_matrix` against the other encoders in one binary.
//!
//! - `cargo bench --bench encode_matrix_tune`: the matrix path at every size (threshold
//!   0), interleaved rounds, the minimum time per call. For tuning `MATRIX_MIN`, and, with
//!   `SIZES` from 4 KiB, the cap.
//! - `cargo bench --bench encode_matrix_tune -- cold`: single calls after touching every
//!   line of 64 MiB to evict the caches, the median of 31 calls. The table is built
//!   before. The timer ticks every 41.7 ns on Apple Silicon.
//! - `cargo bench --bench encode_matrix_tune -- first`: the first call in each table size
//!   class, which builds the table, against the second.
//!
//! `SIZES` (comma-separated) replaces the default list. Above its cap, `matrix` times
//! `encode_matrix`'s fallback.

use std::hint::black_box;
use std::time::Instant;

use base58_bench::bench::encode_matrix_with;
use base58_bench::{
    encode, encode_batched, encode_malachite, encode_ntt_cached, encode_toom3, max_encoded_len,
};
use malachite_base::num::conversion::traits::{Digits, PowerOf2Digits};
use malachite_nz::natural::Natural;
use rand_xoshiro::Xoshiro256PlusPlus;
use rand_xoshiro::rand_core::{Rng, SeedableRng};

type Encode = fn(&[u8], &mut [u8]) -> usize;
/// A name, the encoder, and the sizes to time it at.
type Config = (&'static str, Encode, fn(usize) -> bool);

/// `matrix` is last.
const CONFIGS: [Config; 7] = [
    ("batched", |d, o| encode_batched(d, o).unwrap(), |size| size <= 8192),
    ("dc", |d, o| encode(d, o).unwrap(), large),
    ("dc-toom3", |d, o| encode_toom3(d, o).unwrap(), large),
    ("ntt-cached", |d, o| encode_ntt_cached(d, o).unwrap(), large),
    ("dc-malachite", |d, o| encode_malachite(d, o).unwrap(), large),
    ("malachite", malachite, large),
    ("matrix", |d, o| encode_matrix_with(d, o, 0).unwrap(), |_| true),
];
const SWEEP: [usize; 14] = [32, 48, 64, 80, 96, 112, 128, 144, 160, 192, 224, 256, 384, 512];
const COLD: [usize; 6] = [512, 1024, 1875, 4096, 6144, 8192];
const ROUNDS: usize = 31;
/// Target length of one timed block.
const BLOCK_NS: u128 = 2_000_000;

/// The divide-and-conquer encoders run `encode_batched` up to 3-5 KiB.
fn large(size: usize) -> bool {
    size >= 4096
}

/// malachite-nz's own conversion (a port of GMP's `mpn_get_str`), then the alphabet.
fn malachite(d: &[u8], out: &mut [u8]) -> usize {
    const ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let zeros = d.iter().take_while(|&&b| b == 0).count();
    let x = Natural::from_power_of_2_digits_desc(8, d[zeros..].iter().copied()).unwrap();
    let digits: Vec<u8> = x.to_digits_desc(&58);
    out[..zeros].fill(b'1');
    for (o, &v) in out[zeros..].iter_mut().zip(&digits) {
        *o = ALPHABET[v as usize];
    }
    zeros + digits.len()
}

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
    println!("| bytes | {} | speedup |", CONFIGS.map(|c| c.0).join(" | "));
    println!("|---:|{}---:|", "---:|".repeat(CONFIGS.len()));
    let mut evict = vec![0u8; 64 << 20];
    for size in sizes {
        let data = data(size);
        let mut out = vec![0u8; max_encoded_len(size)];
        let configs: Vec<_> = CONFIGS.iter().filter(|c| c.2(size)).collect();
        let n = encode_batched(&data, &mut out).unwrap();
        let want = out[..n].to_vec();
        for (name, f, _) in &configs {
            let n = f(&data, &mut out);
            assert!(out[..n] == want[..], "{name}: wrong output at {size} bytes");
        }
        let times = if cold {
            cold_times(&configs, &data, &mut out, &mut evict)
        } else {
            warm_times(&configs, &data, &mut out)
        };
        let mut cells = vec![String::new(); CONFIGS.len()];
        for ((name, ..), t) in configs.iter().zip(&times) {
            let i = CONFIGS.iter().position(|c| c.0 == *name).unwrap();
            cells[i] = format!("{:.3}", t / 1000.0);
        }
        // The fastest other config against `matrix`.
        let (others, matrix) = times.split_at(times.len() - 1);
        let (best, name) = others
            .iter()
            .zip(&configs)
            .map(|(&t, c)| (t, c.0))
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .unwrap();
        println!("| {size} | {} | {:.2} ({name}) |", cells.join(" | "), best / matrix[0]);
    }
}

/// The first call in each table class against the second, in a fresh process: the first
/// builds the table.
fn first_calls() {
    println!("| bytes | first call (µs) | second call (µs) |");
    println!("|---:|---:|---:|");
    for size in [512, 1024, 2048, 4096, 8192] {
        let data = data(size);
        let mut out = vec![0u8; max_encoded_len(size)];
        let times: Vec<f64> = (0..2)
            .map(|_| {
                let start = Instant::now();
                black_box(encode_matrix_with(black_box(&data), black_box(&mut out), 0).unwrap());
                start.elapsed().as_nanos() as f64 / 1000.0
            })
            .collect();
        println!("| {size} | {:.1} | {:.1} |", times[0], times[1]);
    }
}

/// The minimum time per call of each config over interleaved rounds, in ns.
fn warm_times(configs: &[&Config], data: &[u8], out: &mut [u8]) -> Vec<f64> {
    let iters: Vec<u128> = configs
        .iter()
        .map(|(_, f, _)| {
            let start = Instant::now();
            f(data, out);
            (BLOCK_NS / start.elapsed().as_nanos().max(1)).max(1)
        })
        .collect();
    let mut best = vec![f64::INFINITY; configs.len()];
    for _ in 0..ROUNDS {
        for (i, (_, f, _)) in configs.iter().enumerate() {
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
fn cold_times(configs: &[&Config], data: &[u8], out: &mut [u8], evict: &mut [u8]) -> Vec<f64> {
    let mut samples = vec![Vec::new(); configs.len()];
    for round in 0..ROUNDS {
        for (i, (_, f, _)) in configs.iter().enumerate() {
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
