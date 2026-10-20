//! Same-binary sweep of the divide-and-conquer encoders' leaf size, against
//! `encode_batched`: interleaved rounds, the minimum time per call of each configuration.
//! The threshold is 0, so every input of more than one leaf uses divide and conquer.
//!
//! `cargo bench --bench encode_tune [-- name...]` runs only the named backends.
//! `LEAVES` and `SIZES` (comma-separated) replace the default lists; `KINDS` picks leaf
//! conversions (`horner`, `matrix`; default both).

use std::hint::black_box;
use std::time::Instant;

use base58_bench::bench::{Leaves, encode_karatsuba_with, encode_malachite_with, encode_ntt_with};
use base58_bench::{BufferTooSmall, encode_batched, encode_matrix, max_encoded_len};
use rand_xoshiro::Xoshiro256PlusPlus;
use rand_xoshiro::rand_core::{Rng, SeedableRng};

type Tuned = fn(&[u8], &mut [u8], usize, usize, Leaves) -> Result<usize, BufferTooSmall>;

const BACKENDS: [(&str, Tuned); 3] = [
    ("ntt-cached", encode_ntt_with),
    ("dc-malachite", encode_malachite_with),
    ("dc", encode_karatsuba_with),
];
const LEAVES: [usize; 7] = [32, 48, 64, 96, 128, 192, 256];
const SIZES: [usize; 8] = [1024, 1536, 2048, 3072, 4096, 8192, 16384, 32768];
const ROUNDS: usize = 11;
/// Target length of one timed block.
const BLOCK_NS: u128 = 10_000_000;

/// The comma-separated list in environment variable `var`, or `default`.
fn list(var: &str, default: &[usize]) -> Vec<usize> {
    std::env::var(var).map_or(default.to_vec(), |s| s.split(',').map(|v| v.parse().unwrap()).collect())
}

fn main() {
    let (leaves, sizes) = (list("LEAVES", &LEAVES), list("SIZES", &SIZES));
    let kinds_var = std::env::var("KINDS").unwrap_or_else(|_| "horner,matrix".into());
    let kinds: Vec<Leaves> = kinds_var
        .split(',')
        .map(|k| match k {
            "horner" => Leaves::Horner,
            "matrix" => Leaves::Matrix,
            _ => panic!("unknown leaf kind {k}"),
        })
        .collect();
    let names: Vec<String> = std::env::args().skip(1).filter(|a| !a.starts_with('-')).collect();
    let backends: Vec<_> =
        BACKENDS.iter().filter(|(n, _)| names.is_empty() || names.iter().any(|a| a == n)).collect();
    let mut configs: Vec<(String, Box<dyn Fn(&[u8], &mut [u8]) -> usize>)> =
        vec![("batched".into(), Box::new(|d, o| encode_batched(d, o).unwrap()))];
    configs.push(("matrix".into(), Box::new(|d, o| encode_matrix(d, o).unwrap())));
    for &&(name, f) in &backends {
        for &leaf in &leaves {
            for &kind in &kinds {
                let run = move |d: &[u8], o: &mut [u8]| f(d, o, 0, leaf, kind).unwrap();
                let tag = if kind == Leaves::Matrix { "m" } else { "h" };
                configs.push((format!("{name}/{leaf}{tag}"), Box::new(run)));
            }
        }
    }
    println!("| bytes | {} |", configs.iter().map(|c| c.0.as_str()).collect::<Vec<_>>().join(" | "));
    println!("|---:|{}", "---:|".repeat(configs.len()));
    for &size in &sizes {
        let mut data = vec![0u8; size];
        Xoshiro256PlusPlus::seed_from_u64(size as u64).fill_bytes(&mut data);
        data[0] |= 1;
        let mut out = vec![0u8; max_encoded_len(size)];
        let n = configs[0].1(&data, &mut out);
        let want = out[..n].to_vec();
        let mut iters = Vec::new();
        for (name, f) in &configs {
            let start = Instant::now();
            let n = f(&data, &mut out);
            assert!(out[..n] == want[..], "{name}: wrong output at {size} bytes");
            iters.push((BLOCK_NS / start.elapsed().as_nanos().max(1)).max(1));
        }
        let mut best = vec![f64::INFINITY; configs.len()];
        for _ in 0..ROUNDS {
            for (i, (_, f)) in configs.iter().enumerate() {
                let start = Instant::now();
                for _ in 0..iters[i] {
                    black_box(f(black_box(&data), black_box(&mut out)));
                }
                let per_call = start.elapsed().as_nanos() as f64 / iters[i] as f64;
                best[i] = best[i].min(per_call);
            }
        }
        let min = best.iter().copied().fold(f64::INFINITY, f64::min);
        let cells: Vec<String> = best
            .iter()
            .map(|&t| {
                let s = format!("{:.1}", t / 1000.0);
                if t == min { format!("**{s}**") } else { s }
            })
            .collect();
        println!("| {size} | {} |", cells.join(" | "));
    }
}
