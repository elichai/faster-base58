//! Same-binary sweeps for the dispatch thresholds, against a reference arm.
//!
//! `cargo bench --features __internals --bench threshold_tune -- MODE`, also with `cache`:
//!
//! - `decode`: every decoder backend (batched, dc, ntt, ntt-cached, matrix) at each size,
//!   for the batched/Karatsuba/NTT and Matrix crossovers.
//! - `encode`: every encoder backend (batched, dc, ntt, ntt-cached, radix, matrix).
//! - `ntt`: `decode_ntt_with` over a grid of the NTT product threshold `T` (`TS`) and the
//!   prepared-power threshold `P` (`PS`); `CACHED=1` uses the cached tables. The
//!   reference arm uses `REF_T` and `REF_P` (default 128 and 96).
//! - `ifma` (x86-64 with AVX-512 IFMA): `mul_with` on `n x n`-limb products for each IFMA
//!   leaf threshold in `TS`; the reference arm uses the current threshold.
//!
//! Each round times one block (at least `BLOCK_MS`, default 10 ms) per arm, rotating the
//! arm order. A row gives, per arm, the median time per call and the median [min, max]
//! of the per-round ratios against the reference (first) arm. `SIZES` (bytes, or limbs
//! for `ifma`), `ROUNDS` (default 11), `TS` and `PS` (comma-separated) replace the defaults.

use std::hint::black_box;
use std::time::Instant;

use faster_base58::internals::{
    decode_batched, decode_karatsuba, decode_matrix, decode_ntt, decode_ntt_cached,
    decode_ntt_with, encode_batched, encode_karatsuba, encode_matrix, encode_ntt,
    encode_ntt_cached, encode_radix, ifma_threshold, mul_with, scratch_len,
};
use rand_xoshiro::Xoshiro256PlusPlus;
use rand_xoshiro::rand_core::{Rng, SeedableRng};

type Arm<'a> = (String, Box<dyn FnMut() -> usize + 'a>);
type Decode = fn(&str, &mut [u8]) -> Result<usize, faster_base58::Error>;
type Encode = fn(&[u8], &mut [u8]) -> Result<usize, faster_base58::BufferTooSmall>;

const CODEC_SIZES: [usize; 27] = [
    256, 320, 384, 420, 448, 512, 640, 768, 1024, 1536, 2048, 2560, 3072, 3584, 3750, 4096, 4224,
    4352, 4608, 4864, 5120, 5376, 5632, 6144, 7168, 8192, 16384,
];
const NTT_SIZES: [usize; 8] = [4096, 6144, 8192, 16384, 32768, 65536, 262_144, 1_048_576];
const IFMA_SIZES: [usize; 7] = [64, 128, 256, 512, 1024, 2048, 4096];

fn list(name: &str, default: &[usize]) -> Vec<usize> {
    std::env::var(name).map_or_else(
        |_| default.to_vec(),
        |s| s.split(',').map(|v| v.parse().unwrap()).collect(),
    )
}

fn number(name: &str, default: usize) -> usize {
    std::env::var(name).map_or(default, |v| v.parse().unwrap())
}

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

/// Random bytes without leading zeros, fixed per size.
fn bytes(size: usize) -> Vec<u8> {
    let mut data = vec![0u8; size];
    Xoshiro256PlusPlus::seed_from_u64(size as u64 ^ 0x5eed).fill_bytes(&mut data);
    if let Some(first) = data.first_mut() {
        *first |= 1;
    }
    data
}

/// Times `arms` in interleaved rounds and prints one row per arm.
fn sweep(label: &str, arms: &mut [Arm]) {
    let rounds = number("ROUNDS", 11);
    let block_ns = number("BLOCK_MS", 10) as u128 * 1_000_000;
    // Warm up (tables, caches) and size each arm's block.
    let iters: Vec<u128> = arms
        .iter_mut()
        .map(|(_, f)| {
            f();
            let start = Instant::now();
            black_box(f());
            (block_ns / start.elapsed().as_nanos().max(1)).max(1)
        })
        .collect();
    let mut times = vec![Vec::with_capacity(rounds); arms.len()];
    for round in 0..rounds {
        for k in 0..arms.len() {
            let i = (round + k) % arms.len();
            let start = Instant::now();
            for _ in 0..iters[i] {
                black_box((arms[i].1)());
            }
            times[i].push(start.elapsed().as_nanos() as f64 / iters[i] as f64);
        }
    }
    for (i, (name, _)) in arms.iter().enumerate() {
        let mut r: Vec<f64> = (0..rounds).map(|j| times[i][j] / times[0][j]).collect();
        let m = median(&mut r);
        println!(
            "| {label} | {name} | {:.1} | {m:.3} [{:.3}, {:.3}] |",
            median(&mut times[i].clone()) / 1000.0,
            r[0],
            r[rounds - 1]
        );
    }
}

fn decode_mode() {
    for size in list("SIZES", &CODEC_SIZES) {
        let data = bytes(size);
        let mut text = vec![0u8; faster_base58::Encoder::bitcoin().max_encoded_len(size)];
        let n = encode_batched(&data, &mut text).unwrap();
        let text = String::from_utf8(text[..n].to_vec()).unwrap();
        let backends: [(&str, Decode); 5] = [
            ("batched", decode_batched),
            ("dc", decode_karatsuba),
            ("ntt", decode_ntt),
            ("ntt-cached", decode_ntt_cached),
            ("matrix", decode_matrix),
        ];
        let mut out = vec![0u8; size];
        let mut arms: Vec<Arm> = Vec::new();
        for (name, f) in backends {
            assert_eq!(f(&text, &mut out).unwrap(), size, "{name} at {size} bytes");
            assert_eq!(out, data, "{name} at {size} bytes");
            let (text, mut out) = (text.as_str(), vec![0u8; size]);
            arms.push((
                name.into(),
                Box::new(move || f(black_box(text), &mut out).unwrap()),
            ));
        }
        sweep(&format!("{size} B / {} digits", text.len()), &mut arms);
    }
}

fn encode_mode() {
    for size in list("SIZES", &CODEC_SIZES) {
        let data = bytes(size);
        let cap = faster_base58::Encoder::bitcoin().max_encoded_len(size);
        let mut want = vec![0u8; cap];
        let n = encode_batched(&data, &mut want).unwrap();
        want.truncate(n);
        let backends: [(&str, Encode); 6] = [
            ("batched", encode_batched),
            ("dc", encode_karatsuba),
            ("ntt", encode_ntt),
            ("ntt-cached", encode_ntt_cached),
            ("radix", encode_radix),
            ("matrix", encode_matrix),
        ];
        let mut arms: Vec<Arm> = Vec::new();
        for (name, f) in backends {
            let mut out = vec![0u8; cap];
            let got = f(&data, &mut out).unwrap();
            assert_eq!(out[..got], want[..], "{name} at {size} bytes");
            let data = data.as_slice();
            arms.push((
                name.into(),
                Box::new(move || f(black_box(data), &mut out).unwrap()),
            ));
        }
        sweep(&format!("{size} B"), &mut arms);
    }
}

fn ntt_mode() {
    let cached = std::env::var("CACHED").is_ok_and(|v| v == "1");
    let (ref_t, ref_p) = (number("REF_T", 128), number("REF_P", 96));
    let ts = list("TS", &[64, 96, 128, 192, 256]);
    let ps = list("PS", &[48, 64, 96, 128, 192]);
    for size in list("SIZES", &NTT_SIZES) {
        let data = bytes(size);
        let mut text = vec![0u8; faster_base58::Encoder::bitcoin().max_encoded_len(size)];
        let n = encode_ntt_cached(&data, &mut text).unwrap();
        let text = String::from_utf8(text[..n].to_vec()).unwrap();
        let mut grid = vec![(ref_t, ref_p)];
        grid.extend(ts.iter().flat_map(|&t| ps.iter().map(move |&p| (t, p))));
        let mut arms: Vec<Arm> = Vec::new();
        for (i, (t, p)) in grid.into_iter().enumerate() {
            let mut out = vec![0u8; size];
            assert_eq!(
                decode_ntt_with(&text, &mut out, cached, t, p).unwrap(),
                size
            );
            assert_eq!(out, data, "T={t} P={p} at {size} bytes");
            let text = text.as_str();
            let name = if i == 0 {
                format!("ref T={t} P={p}")
            } else {
                format!("T={t} P={p}")
            };
            arms.push((
                name,
                Box::new(move || decode_ntt_with(black_box(text), &mut out, cached, t, p).unwrap()),
            ));
        }
        sweep(&format!("{size} B"), &mut arms);
    }
}

fn ifma_mode() {
    let current = ifma_threshold();
    if current == 0 {
        println!("no AVX-512 IFMA: nothing to tune");
        return;
    }
    let ts = list("TS", &[0, 128, 192, 256, 384, 512, 768, 1024]);
    for n in list("SIZES", &IFMA_SIZES) {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(n as u64);
        let a: Vec<u64> = (0..n).map(|_| rng.next_u64()).collect();
        let b: Vec<u64> = (0..n).map(|_| rng.next_u64()).collect();
        let mut want = vec![0u64; 2 * n];
        let mut scratch = vec![0u64; scratch_len(n)];
        mul_with(&mut want, &a, &b, &mut scratch, 0);
        let mut arms: Vec<Arm> = Vec::new();
        for (i, t) in std::iter::once(current)
            .chain(ts.iter().copied())
            .enumerate()
        {
            let (a, b) = (a.as_slice(), b.as_slice());
            let (mut out, mut scratch) = (vec![0u64; 2 * n], vec![0u64; scratch_len(n)]);
            mul_with(&mut out, a, b, &mut scratch, t);
            assert_eq!(out, want, "t={t} at {n} limbs");
            let name = if i == 0 {
                format!("ref t={t}")
            } else {
                format!("t={t}")
            };
            arms.push((
                name,
                Box::new(move || {
                    mul_with(&mut out, black_box(a), black_box(b), &mut scratch, t);
                    out[0] as usize
                }),
            ));
        }
        sweep(&format!("{n} limbs"), &mut arms);
    }
}

fn main() {
    let mode = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with('-'))
        .unwrap_or_default();
    println!("| size | arm | µs | ratio to first arm |");
    println!("|---|---|---:|---|");
    match mode.as_str() {
        "decode" => decode_mode(),
        "encode" => encode_mode(),
        "ntt" => ntt_mode(),
        "ifma" => ifma_mode(),
        _ => panic!("mode must be decode, encode, ntt or ifma"),
    }
}
