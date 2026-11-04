//! Compare separate checksum storage with decoding the same contiguous frame.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use faster_base58::{Decoder, Encoder};
use rand_xoshiro::Xoshiro256PlusPlus;
use rand_xoshiro::rand_core::{Rng, SeedableRng};
use sha2::{Digest, Sha256};
use std::hint::black_box;

const ENCODER: Encoder = Encoder::bitcoin().with_check();
const DECODER: Decoder = Decoder::bitcoin().with_check();
const VERSIONED_ENCODER: Encoder = Encoder::bitcoin().with_check_version(42);
const VERSIONED_DECODER: Decoder = Decoder::bitcoin().with_check_version(42);

fn checksum(data: &[u8]) -> [u8; 4] {
    Sha256::digest(Sha256::digest(data))[..4]
        .try_into()
        .unwrap()
}

fn check_mode<const VERSIONED: bool>(c: &mut Criterion) {
    let encoder = if VERSIONED {
        VERSIONED_ENCODER
    } else {
        ENCODER
    };
    let decoder = if VERSIONED {
        VERSIONED_DECODER
    } else {
        DECODER
    };
    let mut group = c.benchmark_group(if VERSIONED { "check-version" } else { "check" });
    for size in [0, 21, 28, 32, 60, 64, 512, 4096, 8193, 32768] {
        let mut data = vec![0; size];
        Xoshiro256PlusPlus::seed_from_u64(size as u64).fill_bytes(&mut data);
        let text = if VERSIONED {
            bs58::encode(&data).with_check_version(42).into_string()
        } else {
            bs58::encode(&data).with_check().into_string()
        };
        let mut frame = bs58::decode(&text).into_vec().unwrap();
        let len = frame.len() - 4;
        assert_eq!(&frame[len..], checksum(&frame[..len]));
        let expected = frame[..len].to_vec();
        let mut output = vec![0; frame.len()];
        let mut encoded = vec![0; encoder.max_encoded_len(size)];
        assert_eq!(
            decoder.decode(&text).onto(&mut output[..len]).unwrap(),
            expected
        );
        assert_eq!(
            Encoder::bitcoin()
                .encode(&frame)
                .onto(&mut encoded)
                .unwrap(),
            text
        );
        assert_eq!(encoder.encode(&data).onto(&mut encoded).unwrap(), text);
        group.throughput(Throughput::Bytes(len as u64));
        group.bench_function(BenchmarkId::new("decode/checked", size), |b| {
            b.iter(|| {
                (if VERSIONED {
                    VERSIONED_DECODER
                } else {
                    DECODER
                })
                .decode(black_box(&text))
                .onto(black_box(&mut output[..len]))
                .map(|s| s.len())
                .unwrap()
            });
        });
        group.bench_function(BenchmarkId::new("decode/continuous", size), |b| {
            b.iter(|| {
                Decoder::bitcoin()
                    .decode(black_box(&text))
                    .onto(black_box(&mut output))
                    .map(|s| s.len())
                    .unwrap()
            });
        });
        group.bench_function(BenchmarkId::new("decode/continuous-verified", size), |b| {
            b.iter(|| {
                let bytes = Decoder::bitcoin()
                    .decode(black_box(&text))
                    .onto(black_box(&mut output))
                    .unwrap();
                let len = bytes.len() - 4;
                assert_eq!(&bytes[len..], checksum(&bytes[..len]));
                if VERSIONED {
                    assert_eq!(bytes.first(), Some(&42));
                }
                black_box(len)
            });
        });
        group.bench_function(BenchmarkId::new("encode/checked", size), |b| {
            b.iter(|| {
                (if VERSIONED {
                    VERSIONED_ENCODER
                } else {
                    ENCODER
                })
                .encode(black_box(&data))
                .onto(black_box(&mut encoded))
                .map(str::len)
                .unwrap()
            });
        });
        group.bench_function(BenchmarkId::new("encode/continuous", size), |b| {
            b.iter(|| {
                Encoder::bitcoin()
                    .encode(black_box(&frame))
                    .onto(black_box(&mut encoded))
                    .map(str::len)
                    .unwrap()
            });
        });
        group.bench_function(BenchmarkId::new("encode/continuous-hashed", size), |b| {
            b.iter(|| {
                let data = black_box(&data);
                let frame = black_box(&mut frame);
                if VERSIONED {
                    frame[0] = 42;
                }
                frame[usize::from(VERSIONED)..len].copy_from_slice(data);
                let sum = checksum(&frame[..len]);
                frame[len..].copy_from_slice(&sum);
                Encoder::bitcoin()
                    .encode(frame)
                    .onto(black_box(&mut encoded))
                    .map(str::len)
                    .unwrap()
            });
        });
    }
    group.finish();
}

fn check(c: &mut Criterion) {
    check_mode::<false>(c);
    check_mode::<true>(c);
}

criterion_group!(benches, check);
criterion_main!(benches);
