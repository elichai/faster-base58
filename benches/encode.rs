use std::hint::black_box;
use std::sync::LazyLock;

use criterion::{BenchmarkId, Criterion, SamplingMode, Throughput, criterion_group, criterion_main};
use malachite_base::num::conversion::traits::{Digits, PowerOf2Digits};
use malachite_nz::natural::Natural;
use rand_xoshiro::Xoshiro256PlusPlus;
use rand_xoshiro::rand_core::{Rng, SeedableRng};

/// Encodes `input` into `out` and returns the number of bytes written.
type EncodeFn = fn(&[u8], &mut [u8]) -> usize;

/// Add new implementations here, with the sizes to time them at.
const ENCODERS: &[(&str, EncodeFn, fn(usize) -> bool)] = &[
    ("bs58", |d, out| bs58::encode(d).onto(out).unwrap(), quadratic),
    ("basexx", |d, out| copy_out(BASEXX.encode(d).unwrap().as_bytes(), out), quadratic),
    ("basexx-b", |d, out| copy_out(BASEXX_B.encode(d).unwrap().as_bytes(), out), quadratic),
    ("five8", five8, |size| size == 32 || size == 64),
    ("malachite", malachite, all),
    ("batched", |d, out| base58_bench::encode_batched(d, out).unwrap(), quadratic),
    ("dc", |d, out| base58_bench::encode(d, out).unwrap(), dc),
    ("dc-toom3", |d, out| base58_bench::encode_toom3(d, out).unwrap(), dc),
    ("ntt", |d, out| base58_bench::encode_ntt(d, out).unwrap(), dc),
    ("ntt-cached", |d, out| base58_bench::encode_ntt_cached(d, out).unwrap(), dc),
    ("ntt-portable", |d, out| base58_bench::encode_ntt_portable(d, out).unwrap(), dc),
    ("dc-malachite", |d, out| base58_bench::encode_malachite(d, out).unwrap(), dc),
    ("matrix", |d, out| base58_bench::encode_matrix(d, out).unwrap(), |size| size <= 8192),
    ("matrix-portable", |d, out| base58_bench::encode_matrix_portable(d, out).unwrap(), |size| size <= 8192),
    ("matrix-general", matrix_general, |size| size == 32 || size == 64),
];

/// `encode_matrix` without its fixed-size path for 32 and 64 bytes (22 is its
/// `MATRIX_MIN`).
fn matrix_general(d: &[u8], out: &mut [u8]) -> usize {
    base58_bench::bench::encode_matrix_with(d, out, 22).unwrap()
}

/// The divide-and-conquer encoders run `encode_batched` below about 3.7 KiB.
fn dc(size: usize) -> bool {
    size >= 4096
}

const ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

fn all(_: usize) -> bool {
    true
}

/// Quadratic encoders take seconds per iteration above 32 KiB.
fn quadratic(size: usize) -> bool {
    size <= 32 * 1024
}

// Constructors allocate lookup tables; build them once.
static BASEXX: LazyLock<basexx::Base58> = LazyLock::new(basexx::Base58::new);
static BASEXX_B: LazyLock<basexx::Base58B> = LazyLock::new(basexx::Base58B::new);

/// Adapter for encoders that only return an owned string.
fn copy_out(v: &[u8], out: &mut [u8]) -> usize {
    out[..v.len()].copy_from_slice(v);
    v.len()
}

/// Firedancer's fixed-size encoders, for 32 and 64 bytes only.
fn five8(d: &[u8], out: &mut [u8]) -> usize {
    let n = match d.len() {
        32 => five8::encode_32(d.try_into().unwrap(), (&mut out[..44]).try_into().unwrap()),
        64 => five8::encode_64(d.try_into().unwrap(), (&mut out[..88]).try_into().unwrap()),
        _ => unreachable!("five8 has only 32 and 64 bytes"),
    };
    n as usize
}

/// malachite-nz's own conversion (a port of GMP's `mpn_get_str`), then the alphabet.
fn malachite(d: &[u8], out: &mut [u8]) -> usize {
    let zeros = d.iter().take_while(|&&b| b == 0).count();
    let x = Natural::from_power_of_2_digits_desc(8, d[zeros..].iter().copied()).unwrap();
    let digits: Vec<u8> = x.to_digits_desc(&58);
    out[..zeros].fill(b'1');
    for (o, &v) in out[zeros..].iter_mut().zip(&digits) {
        *o = ALPHABET[v as usize];
    }
    zeros + digits.len()
}

/// Input sizes in bytes.
const SIZES: [usize; 20] = [
    31,
    32,
    64,
    128,
    256,
    512,
    1024,
    1875,
    4096,
    6144,
    8192,
    12 * 1024,
    16 * 1024,
    24 * 1024,
    32 * 1024,
    36 * 1024,
    48 * 1024,
    128 * 1024,
    144 * 1024,
    1024 * 1024,
];

/// From this size on, each iteration is slow enough to use few, flat samples.
const LARGE: usize = 12 * 1024;

/// Deterministic random bytes, first byte forced non-zero so the encoded length is not
/// skewed by leading-zero '1' characters.
fn random_bytes(len: usize, seed: u64) -> Vec<u8> {
    let mut v = vec![0u8; len];
    Xoshiro256PlusPlus::seed_from_u64(seed).fill_bytes(&mut v);
    if let Some(b) = v.first_mut() {
        *b |= 1;
    }
    v
}

fn encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode");
    for size in SIZES {
        let data = random_bytes(size, size as u64);
        let mut out = vec![0u8; base58_bench::max_encoded_len(size)];
        let encoders = || ENCODERS.iter().filter(|&&(_, _, at)| at(size));

        // Sanity check every encoder against malachite before timing it.
        let mut reference = vec![0u8; out.len()];
        let want = malachite(&data, &mut reference);
        for &(name, f, _) in encoders() {
            out.fill(0);
            assert_eq!(f(&data, &mut out), want, "{name}: wrong length");
            assert_eq!(out[..want], reference[..want], "{name}: wrong output");
        }

        if size >= LARGE {
            group.sample_size(10).sampling_mode(SamplingMode::Flat);
        }
        // Throughput is reported in input bytes.
        group.throughput(Throughput::Bytes(size as u64));
        for &(name, f, _) in encoders() {
            group.bench_with_input(BenchmarkId::new(name, size), &data, |b, d| {
                b.iter(|| f(black_box(d), black_box(&mut out)))
            });
        }
        for &(name, _, f) in SWEEP.iter().filter(|&&(_, n, _)| n == size) {
            out.fill(0);
            assert_eq!(f(&data, &mut out), Ok(want), "{name}: wrong length");
            assert_eq!(out[..want], reference[..want], "{name}: wrong output");
            group.bench_with_input(BenchmarkId::new(name, size), &data, |b, d| {
                b.iter(|| f(black_box(d), black_box(&mut out)))
            });
        }
    }
    group.finish();
}

type Fixed = fn(&[u8], &mut [u8]) -> Result<usize, base58_bench::BufferTooSmall>;

/// Carry variants for 32 and 64 bytes (an M5 sweep; temporary).
#[cfg(all(target_arch = "aarch64", target_endian = "little", target_feature = "neon"))]
const SWEEP: &[(&str, usize, Fixed)] = &base58_bench::bench::fixed_sweep::VARIANTS;
#[cfg(not(all(target_arch = "aarch64", target_endian = "little", target_feature = "neon")))]
const SWEEP: &[(&str, usize, Fixed)] = &[];

criterion_group!(benches, encode);
criterion_main!(benches);
