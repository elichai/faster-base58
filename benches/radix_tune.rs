//! Same-binary comparison of the forced base-58³ multiplication tree (`encode_radix`)
//! with the forced division tree (`encode_ntt_cached`), for tuning the radix route's
//! utilization rule. A third arm runs the division tree again as an A/A control.
//!
//! Each round times one block (at least `BLOCK_MS`, default 10 ms) per arm, rotating the
//! arm order. A row reports the median and range of the per-round ratios against the
//! division tree, and each arm's median time per call.
//!
//! `SIZES` (comma-separated bytes) and `ROUNDS` (default 11) replace the defaults.
//! Run `cargo bench --features __internals --bench radix_tune`, also with `cache`.

use std::hint::black_box;
use std::time::Instant;

use faster_base58::BufferTooSmall;
use faster_base58::internals::{encode_ntt_cached, encode_radix};
use rand_xoshiro::Xoshiro256PlusPlus;
use rand_xoshiro::rand_core::{Rng, SeedableRng};

type Encode = fn(&[u8], &mut [u8]) -> Result<usize, BufferTooSmall>;

const ARMS: [(&str, Encode); 3] = [
    ("division", encode_ntt_cached),
    ("radix", encode_radix),
    ("division-aa", encode_ntt_cached),
];
const SIZES: [usize; 8] = [
    8192, 12288, 16384, 65536, 98304, 131072, 786_432, 1_048_576,
];

fn var(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn main() {
    let sizes: Vec<usize> = var("SIZES").map_or_else(
        || SIZES.to_vec(),
        |s| s.split(',').map(|v| v.parse().unwrap()).collect(),
    );
    let rounds: usize = var("ROUNDS").map_or(11, |v| v.parse().unwrap());
    let block_ns = var("BLOCK_MS").map_or(10, |v| v.parse::<u128>().unwrap()) * 1_000_000;
    println!("| bytes | radix/div | range | A/A | range | division µs | radix µs |");
    println!("|---:|---:|---|---:|---|---:|---:|");
    for &size in &sizes {
        let mut data = vec![0u8; size];
        Xoshiro256PlusPlus::seed_from_u64(size as u64).fill_bytes(&mut data);
        data[0] |= 1;
        let mut out = vec![0u8; faster_base58::Encoder::bitcoin().max_encoded_len(size)];
        let n = encode_ntt_cached(&data, &mut out).unwrap();
        let want = out[..n].to_vec();
        // Warm up (and fill caches), check output, and size each arm's block.
        let iters: Vec<u128> = ARMS
            .iter()
            .map(|(name, f)| {
                f(&data, &mut out).unwrap();
                let start = Instant::now();
                let n = f(&data, &mut out).unwrap();
                assert!(out[..n] == want[..], "{name}: wrong output at {size} bytes");
                (block_ns / start.elapsed().as_nanos().max(1)).max(1)
            })
            .collect();
        let mut times = vec![Vec::with_capacity(rounds); ARMS.len()];
        for round in 0..rounds {
            for k in 0..ARMS.len() {
                let i = (round + k) % ARMS.len();
                let f = ARMS[i].1;
                let start = Instant::now();
                for _ in 0..iters[i] {
                    black_box(f(black_box(&data), black_box(&mut out))).unwrap();
                }
                times[i].push(start.elapsed().as_nanos() as f64 / iters[i] as f64);
            }
        }
        let ratios = |arm: usize| {
            let mut r: Vec<f64> = (0..rounds).map(|j| times[arm][j] / times[0][j]).collect();
            let m = median(&mut r);
            (m, r[0], r[rounds - 1])
        };
        let (radix, rlo, rhi) = ratios(1);
        let (aa, alo, ahi) = ratios(2);
        println!(
            "| {size} | {radix:.3} | {rlo:.3}–{rhi:.3} | {aa:.3} | {alo:.3}–{ahi:.3} | {:.1} | {:.1} |",
            median(&mut times[0]) / 1000.0,
            median(&mut times[1]) / 1000.0,
        );
    }
}
