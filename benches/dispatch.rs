//! Crossover measurements, including small lengths absent from the original benches.

#[path = "../tests/support/mod.rs"]
mod support;

use criterion::{
    BenchmarkId, Criterion, SamplingMode, Throughput, criterion_group, criterion_main,
};
use rand_xoshiro::Xoshiro256PlusPlus;
use rand_xoshiro::rand_core::{Rng, SeedableRng};
use std::hint::black_box;

const SIZES: &[usize] = &[
    0, 1, 2, 4, 7, 8, 16, 21, 22, 23, 31, 32, 33, 63, 64, 65, 128, 256, 384, 512, 768, 1024, 1536,
    2048, 3072, 4096, 6144, 8192, 12288, 32768, 36864, 65536, 131072, 147456, 1048576,
];

fn applicable(name: &str, size: usize) -> bool {
    match name {
        "batched" | "matrix" | "matrix-portable" => size <= 8192,
        _ => true,
    }
}

fn dispatch(c: &mut Criterion) {
    let mut group = c.benchmark_group("dispatch");
    group.sample_size(10).sampling_mode(SamplingMode::Flat);
    for &size in SIZES {
        let mut data = vec![0; size];
        Xoshiro256PlusPlus::seed_from_u64(size as u64 + 20261023).fill_bytes(&mut data);
        if let Some(first) = data.first_mut() {
            *first |= 1;
        }
        let mut encoded = vec![0; faster_base58::max_encoded_len(size)];
        let len = support::malachite::encode_malachite(&data, &mut encoded).unwrap();
        encoded.truncate(len);
        if size <= 4096 {
            assert_eq!(encoded, bs58::encode(&data).into_vec());
        }
        let text = std::str::from_utf8(&encoded).unwrap();
        group.throughput(Throughput::Bytes(size as u64));
        for &(name, f) in support::ENCODERS {
            if !applicable(name, size) {
                continue;
            }
            let mut out = vec![0; faster_base58::max_encoded_len(size)];
            assert_eq!(f(&data, &mut out), Ok(len), "{name} encode {size}");
            assert_eq!(&out[..len], encoded, "{name} encode {size}");
            group.bench_with_input(
                BenchmarkId::new(format!("encode/{name}"), size),
                &data,
                |b, d| {
                    b.iter(|| f(black_box(d), black_box(&mut out)).unwrap());
                },
            );
        }
        for &(name, f) in support::DECODERS {
            if !applicable(name, size) {
                continue;
            }
            let mut out = vec![0; size];
            assert_eq!(f(text, &mut out), Ok(size), "{name} decode {size}");
            assert_eq!(out, data, "{name} decode {size}");
            group.bench_with_input(
                BenchmarkId::new(format!("decode/{name}"), size),
                &text,
                |b, s| {
                    b.iter(|| f(black_box(s), black_box(&mut out)).unwrap());
                },
            );
        }
        if size == 32 || size == 64 {
            let mut out = vec![0; size];
            let f = |s: &str, out: &mut [u8]| match size {
                32 => five8::decode_32(s, out.try_into().unwrap()).unwrap(),
                64 => five8::decode_64(s, out.try_into().unwrap()).unwrap(),
                _ => unreachable!(),
            };
            f(text, &mut out);
            assert_eq!(out, data);
            group.bench_with_input(BenchmarkId::new("decode/five8", size), &text, |b, s| {
                b.iter(|| f(black_box(s), black_box(&mut out)));
            });
        }
    }
    group.finish();
}

criterion_group!(benches, dispatch);
criterion_main!(benches);
