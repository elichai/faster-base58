use std::hint::black_box;
use std::sync::LazyLock;

use criterion::{BenchmarkId, Criterion, SamplingMode, Throughput, criterion_group, criterion_main};
use rand_xoshiro::Xoshiro256PlusPlus;
use rand_xoshiro::rand_core::{Rng, SeedableRng};

/// Decodes `input` into `out` and returns the number of bytes written.
type DecodeFn = fn(&str, &mut [u8]) -> usize;

/// Add new implementations here, with the largest decoded size to time them at.
const DECODERS: &[(&str, DecodeFn, usize)] = &[
    ("bs58", |s, out| bs58::decode(s).onto(out).unwrap(), QUADRATIC_MAX),
    ("basexx", |s, out| copy_out(&BASEXX.decode(s).unwrap(), out), QUADRATIC_MAX),
    ("basexx-b", |s, out| copy_out(&BASEXX_B.decode(s).unwrap(), out), QUADRATIC_MAX),
    ("batched", |s, out| base58_bench::decode_batched(s, out).unwrap(), QUADRATIC_MAX),
    ("dc", |s, out| base58_bench::decode(s, out).unwrap(), usize::MAX),
    ("dc-toom3", |s, out| base58_bench::decode_toom3(s, out).unwrap(), usize::MAX),
    ("ntt", |s, out| base58_bench::decode_ntt(s, out).unwrap(), usize::MAX),
    ("ntt-cached", |s, out| base58_bench::decode_ntt_cached(s, out).unwrap(), usize::MAX),
    ("ntt-portable", |s, out| base58_bench::decode_ntt_portable(s, out).unwrap(), usize::MAX),
    ("dc-malachite", |s, out| base58_bench::decode_malachite(s, out).unwrap(), usize::MAX),
    // Above 5632 characters (about 4 KiB) these call `decode`.
    ("matrix", |s, out| base58_bench::decode_matrix(s, out).unwrap(), MATRIX_MAX),
    (
        "matrix-portable",
        |s, out| base58_bench::decode_matrix_portable(s, out).unwrap(),
        MATRIX_MAX,
    ),
];

/// The matrix decoders only call `decode` above 4 KiB; time that once.
const MATRIX_MAX: usize = 32 * 1024;

/// Quadratic decoders take seconds per iteration above this decoded size.
const QUADRATIC_MAX: usize = 32 * 1024;

// Constructors allocate lookup tables; build them once.
static BASEXX: LazyLock<basexx::Base58> = LazyLock::new(basexx::Base58::new);
static BASEXX_B: LazyLock<basexx::Base58B> = LazyLock::new(basexx::Base58B::new);

/// Adapter for decoders that only return a `Vec`.
fn copy_out(v: &[u8], out: &mut [u8]) -> usize {
    out[..v.len()].copy_from_slice(v);
    v.len()
}

/// Decoded sizes in bytes. 32 KiB, 128 KiB and 1 MiB fill most of a power-of-two
/// transform at every `dc` level; 36 KiB and 144 KiB are just past that.
const SIZES: [usize; 14] = [
    31,
    32,
    128,
    256,
    512,
    1024,
    1875,
    4096,
    32 * 1024,
    36 * 1024,
    48 * 1024,
    128 * 1024,
    144 * 1024,
    1024 * 1024,
];

/// From this size on, base58's quadratic cost makes each iteration slow
/// enough to use few, flat samples.
const LARGE: usize = 32 * 1024;

/// Deterministic random bytes, first byte forced non-zero so the
/// encoded length is not skewed by leading-zero '1' characters.
fn random_bytes(len: usize, seed: u64) -> Vec<u8> {
    let mut v = vec![0u8; len];
    Xoshiro256PlusPlus::seed_from_u64(seed).fill_bytes(&mut v);
    if let Some(b) = v.first_mut() {
        *b |= 1;
    }
    v
}

/// An input that decodes to about `size` bytes, and its decoding.
///
/// Up to [`QUADRATIC_MAX`]: random bytes, encoded by bs58. Above, bs58's quadratic
/// encoder is too slow, so the input is a random string of `8 size / log2(58)`
/// characters, and `decode` (checked against bs58 by the unit tests) gives the bytes.
fn input(size: usize) -> (String, Vec<u8>) {
    if size <= QUADRATIC_MAX {
        let data = random_bytes(size, size as u64);
        return (bs58::encode(&data).into_string(), data);
    }
    let alphabet = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let len = (8.0 * size as f64 / 58f64.log2()) as usize;
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(size as u64);
    // No leading '1', so no leading zero bytes.
    let digit = |i: usize, r: u32| if i == 0 { 1 + r % 57 } else { r % 58 };
    let s: String = (0..len).map(|i| alphabet[digit(i, rng.next_u32()) as usize] as char).collect();
    // 58^len <= 2^(8 size) up to rounding, so `size + 1` bytes always suffice.
    let mut data = vec![0; size + 1];
    let n = base58_bench::decode(&s, &mut data).unwrap();
    data.truncate(n);
    (s, data)
}

fn decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode");
    for size in SIZES {
        let (encoded, data) = input(size);
        let mut out = vec![0u8; data.len()];
        let decoders = || DECODERS.iter().filter(|&&(_, _, max)| size <= max);

        // Sanity check every decoder before timing it.
        for &(name, f, _) in decoders() {
            out.fill(0);
            assert_eq!(f(&encoded, &mut out), data.len(), "{name}: wrong length");
            assert_eq!(out, data, "{name}: wrong output");
        }

        if size >= LARGE {
            group.sample_size(10).sampling_mode(SamplingMode::Flat);
        }
        // Throughput is reported in decoded bytes.
        group.throughput(Throughput::Bytes(data.len() as u64));
        for &(name, f, _) in decoders() {
            group.bench_with_input(BenchmarkId::new(name, size), &encoded, |b, s| {
                b.iter(|| f(black_box(s), black_box(&mut out)))
            });
        }
    }
    group.finish();
}

criterion_group!(benches, decode);
criterion_main!(benches);
