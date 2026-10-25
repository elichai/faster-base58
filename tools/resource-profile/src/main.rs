//! One backend/operation/size per fresh process. Build twice: counted heap or timing.

#[cfg(feature = "count-allocations")]
mod allocator;
#[path = "../../../tests/support/malachite.rs"]
mod malachite;
#[path = "../../../tests/support/competitors.rs"]
mod competitors;

use base58_bench::{self as b58, internals};
use malachite_base::num::conversion::traits::{Digits, PowerOf2Digits};
use malachite_nz::natural::Natural;
use std::{hint::black_box, time::Instant};

const MAX_INPUT_BYTES: usize = 4_194_304;
const MIN_BATCH_NS: u128 = 3_000_000;
const MAX_ITERATIONS: usize = 10_000_000;
const TIMING_PROTOCOL: &str = "base58-timing-20261025-v2";

#[cfg(feature = "count-allocations")]
#[global_allocator]
static ALLOCATOR: allocator::Counting = allocator::Counting;

type Encode = fn(&[u8], &mut [u8]) -> Result<usize, b58::BufferTooSmall>;
type Decode = fn(&str, &mut [u8]) -> Result<usize, b58::Error>;

const BACKENDS: &[(&str, Encode, Decode)] = &[
    ("auto", b58::encode, b58::decode),
    ("no-alloc", b58::no_alloc::encode, b58::no_alloc::decode),
    ("batched", b58::internals::encode_batched, b58::internals::decode_batched),
    ("dc", b58::internals::encode_karatsuba, b58::internals::decode_karatsuba),
    ("toom3", b58::internals::encode_toom3, b58::internals::decode_toom3),
    ("ntt", b58::internals::encode_ntt, b58::internals::decode_ntt),
    ("ntt-cached", b58::internals::encode_ntt_cached, b58::internals::decode_ntt_cached),
    ("ntt-portable", b58::internals::encode_ntt_portable, b58::internals::decode_ntt_portable),
    ("malachite", malachite::encode_malachite, malachite::decode_malachite),
    ("matrix", b58::internals::encode_matrix, b58::internals::decode_matrix),
    ("matrix-portable", b58::internals::encode_matrix_portable, b58::internals::decode_matrix_portable),
    #[cfg(feature = "bs58")]
    ("bs58", competitors::encode_bs58, competitors::decode_bs58),
    #[cfg(feature = "five8")]
    ("five8", competitors::encode_five8, competitors::decode_five8),
    ("matrix-forced", internals::encode_matrix_whole, internals::decode_matrix_forced),
    ("ntt-same-owned", |s, o| internals::encode_ntt_same_leaves(s, o, false),
        |s, o| internals::decode_ntt_same_leaves(s, o, false)),
    ("ntt-same-cached", |s, o| internals::encode_ntt_same_leaves(s, o, true),
        |s, o| internals::decode_ntt_same_leaves(s, o, true)),
];

fn max_bytes(backend: &str, operation: &str) -> usize {
    match backend {
        "batched" | "no-alloc" | "bs58" => 65_536,
        "five8" => 64,
        "matrix-forced" if operation == "encode" => 8192,
        "matrix-forced" => 5632,
        _ => MAX_INPUT_BYTES,
    }
}

fn supports(backend: &str, operation: &str, size: usize) -> bool {
    size <= max_bytes(backend, operation)
        && (backend != "five8" || matches!(size, 32 | 64))
        && (backend != "matrix-forced" || size != 0)
}

#[cfg(all(target_pointer_width = "64", any(target_os = "macos", target_os = "linux")))]
fn cpu_ns() -> u128 {
    use std::ffi::{c_int, c_long};
    #[repr(C)]
    struct Timespec { seconds: c_long, nanos: c_long }
    unsafe extern "C" {
        fn clock_gettime(clock: c_int, time: *mut Timespec) -> c_int;
    }
    #[cfg(target_os = "macos")]
    const PROCESS_CPU: c_int = 12;
    #[cfg(target_os = "linux")]
    const PROCESS_CPU: c_int = 2;
    let mut time = Timespec { seconds: 0, nanos: 0 };
    // SAFETY: native 64-bit macOS/Linux timespec is two C longs; the pointer is valid.
    assert_eq!(unsafe { clock_gettime(PROCESS_CPU, &mut time) }, 0);
    assert!(time.seconds >= 0 && (0..1_000_000_000).contains(&time.nanos));
    time.seconds as u128 * 1_000_000_000 + time.nanos as u128
}

#[cfg(not(all(target_pointer_width = "64", any(target_os = "macos", target_os = "linux"))))]
fn cpu_ns() -> u128 {
    panic!("process CPU clock is supported only on 64-bit macOS/Linux")
}

fn describe() {
    #[cfg(target_arch = "x86_64")]
    let [sse4_1, avx2, avx512ifma, avx512vbmi] = [
        std::is_x86_feature_detected!("sse4.1"),
        std::is_x86_feature_detected!("avx2"),
        std::is_x86_feature_detected!("avx512ifma"),
        std::is_x86_feature_detected!("avx512vbmi"),
    ];
    #[cfg(not(target_arch = "x86_64"))]
    let [sse4_1, avx2, avx512ifma, avx512vbmi] = [false; 4];
    print!("{{\"schema\":1,\"arch\":\"{}\",\"os\":\"{}\",\"pointer_bits\":{},\"count_allocations\":{},\"ifma_threshold\":{},\"source_sha256\":\"{}\"", std::env::consts::ARCH, std::env::consts::OS, usize::BITS, cfg!(feature = "count-allocations"), internals::ifma_threshold(), option_env!("RESOURCE_SOURCE_SHA256").unwrap_or("unbound"));
    print!(",\"timing_protocol\":\"{TIMING_PROTOCOL}\",\"max_input_bytes\":{MAX_INPUT_BYTES},\"primary_clock\":\"cpu_ns\",\"minimum_batch_ns\":{MIN_BATCH_NS},\"max_iterations\":{MAX_ITERATIONS},\"oracle\":\"malachite-natural-radix-conversion\"");
    print!(",\"runtime_features\":{{\"sse4_1\":{sse4_1},\"avx2\":{avx2},\"avx512ifma\":{avx512ifma},\"avx512vbmi\":{avx512vbmi}}}");
    println!(",\"compiled_features\":{{\"avx2\":{},\"neon\":{},\"little_endian\":{},\"cache\":{}}}}}", cfg!(target_feature = "avx2"), cfg!(target_feature = "neon"), cfg!(target_endian = "little"), cfg!(feature = "cache"));
}

fn data(size: usize) -> Vec<u8> {
    let mut result = vec![0; size];
    let mut x = size as u64 + 20_261_023;
    for byte in &mut result {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        *byte = x as u8;
    }
    if let Some(first) = result.first_mut() { *first |= 1; }
    result
}

// Noncryptographic fixture-consistency fingerprints; source and binary identity use SHA256.
fn fnv1a64(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf29ce484222325, |hash, &byte| {
        (hash ^ byte as u64).wrapping_mul(0x100000001b3)
    })
}

fn check_fixtures() {
    assert_eq!(data(1), [0xf1]);
    assert_eq!(data(8), [0x37, 0xa0, 0xbb, 0x4e, 0xe0, 0x7b, 0xcf, 0xec]);
    assert_eq!(data(32), [
        0x2f, 0xb4, 0x63, 0x57, 0x3f, 0xa3, 0x3c, 0x32,
        0xe0, 0x2b, 0xb3, 0x68, 0x4e, 0x7a, 0xd2, 0x8d,
        0xe2, 0x91, 0x7a, 0xda, 0x29, 0x61, 0x59, 0xb3,
        0x10, 0xca, 0x2d, 0x29, 0x9b, 0xf6, 0x4b, 0x8d,
    ]);
    assert_eq!(fnv1a64(b""), 0xcbf29ce484222325);
    assert_eq!(fnv1a64(b"a"), 0xaf63dc4c8601ec8c);
    assert_eq!(fnv1a64(b"foobar"), 0x85944171f73967e8);
}

fn fixture(data: &[u8]) -> Vec<u8> {
    let mut encoded = vec![0; b58::max_encoded_len(data.len())];
    let len = encode_fixture(data, &mut encoded);
    encoded.truncate(len);
    encoded
}

// Independent radix conversion: no base58-bench decoder, encoder, or multiplication adapter.
fn encode_fixture(data: &[u8], output: &mut [u8]) -> usize {
    const ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let zeros = data.iter().take_while(|&&byte| byte == 0).count();
    let value = Natural::from_power_of_2_digits_desc(8, data[zeros..].iter().copied()).unwrap();
    let digits: Vec<u8> = value.to_digits_desc(&58);
    output[..zeros].fill(b'1');
    for (position, &digit) in digits.iter().enumerate() {
        output[zeros + position] = ALPHABET[digit as usize];
    }
    zeros + digits.len()
}

fn check_competitors() {
    let mut backends: Vec<(&str, Encode, Decode)> = Vec::new();
    #[cfg(feature = "bs58")]
    backends.push(("bs58", competitors::encode_bs58, competitors::decode_bs58));
    #[cfg(feature = "five8")]
    backends.push(("five8", competitors::encode_five8, competitors::decode_five8));
    for size in [0, 1, 31, 32, 33, 63, 64, 65, 513] {
        for data in [data(size), vec![0; size], vec![0xff; size]] {
            let encoded = fixture(&data);
            let text = std::str::from_utf8(&encoded).unwrap();
            for &(name, encode, decode) in &backends {
                if name == "five8" && !matches!(size, 32 | 64) { continue; }
                let mut output = vec![0; b58::max_encoded_len(size)];
                let len = encode(&data, &mut output).unwrap();
                assert_eq!(len, encoded.len(), "{name} encode length");
                assert_eq!(&output[..len], encoded, "{name} encode bytes");
                if len != 0 { assert!(encode(&data, &mut output[..len - 1]).is_err()); }
                let mut output = vec![0; size];
                let len = decode(text, &mut output).unwrap();
                assert_eq!(len, size, "{name} decode length");
                assert_eq!(output, data, "{name} decode bytes");
                if len != 0 { assert!(decode(text, &mut output[..len - 1]).is_err()); }
            }
        }
    }
    #[cfg(feature = "five8")]
    for size in [0, 1, 31, 33, 63, 65, 513] {
        assert!(competitors::encode_five8(&data(size), &mut [0; 88]).is_err());
        assert!(competitors::decode_five8("1", &mut vec![0; size]).is_err());
    }
}

#[cfg(test)]
#[test]
fn competitors_match_malachite_and_reject_short_buffers() {
    check_competitors();
}

#[cfg(test)]
#[test]
fn fixture_vectors_and_fixed_width_support() {
    check_fixtures();
    for operation in ["encode", "decode"] {
        for size in [32, 64] { assert!(supports("five8", operation, size)); }
        for size in [0, 1, 31, 33, 63, 65] { assert!(!supports("five8", operation, size)); }
        for name in ["bs58", "batched", "no-alloc"] {
            assert!(supports(name, operation, 65_536));
            assert!(!supports(name, operation, 1_048_576));
        }
    }
}

#[cfg(test)]
#[test]
fn four_mib_and_forced_matrix_domains() {
    for operation in ["encode", "decode"] {
        for name in ["auto", "dc", "ntt-same-owned", "ntt-same-cached"] {
            assert!(supports(name, operation, 4_194_304));
            assert!(!supports(name, operation, 4_194_305));
        }
        assert!(!supports("matrix-forced", operation, 0));
        assert!(supports("matrix-forced", operation, 1));
        for name in ["batched", "no-alloc", "bs58", "five8"] {
            assert!(!supports(name, operation, 4_194_304));
        }
    }
}

fn cache_bytes() -> [usize; 3] {
    let encode = internals::encode_matrix_cache_memory().1.iter().map(|r| r.3 + r.5).sum();
    let decode = internals::decode_matrix_cache_memory().1.iter().map(|r| r.3 + r.5).sum();
    let ntt = internals::ntt_cache_memory().1.iter().map(|r| r.3).sum();
    [encode, decode, ntt]
}

fn caches(phase: &str) {
    let (headers, rows) = internals::encode_matrix_cache_memory();
    println!("{{\"kind\":\"cache-header\",\"phase\":\"{phase}\",\"family\":\"encode-matrix\",\"header_bytes\":{headers}}}");
    for (class, initialized, panels, panel_capacity_bytes, data_len, data_capacity_bytes) in rows {
        println!("{{\"kind\":\"cache\",\"phase\":\"{phase}\",\"family\":\"encode-matrix\",\"class\":{class},\"initialized\":{initialized},\"panels\":{panels},\"panel_capacity_bytes\":{panel_capacity_bytes},\"data_len\":{data_len},\"data_capacity_bytes\":{data_capacity_bytes}}}");
    }
    let (headers, rows) = internals::decode_matrix_cache_memory();
    println!("{{\"kind\":\"cache-header\",\"phase\":\"{phase}\",\"family\":\"decode-matrix\",\"header_bytes\":{headers}}}");
    for (class, initialized, panels, panel_capacity_bytes, data_len, data_capacity_bytes) in rows {
        println!("{{\"kind\":\"cache\",\"phase\":\"{phase}\",\"family\":\"decode-matrix\",\"class\":{class},\"initialized\":{initialized},\"panels\":{panels},\"panel_capacity_bytes\":{panel_capacity_bytes},\"data_len\":{data_len},\"data_capacity_bytes\":{data_capacity_bytes}}}");
    }
    let (headers, rows) = internals::ntt_cache_memory();
    println!("{{\"kind\":\"cache-header\",\"phase\":\"{phase}\",\"family\":\"ntt\",\"header_bytes\":{headers}}}");
    for (class, initialized, data_len, data_capacity_bytes) in rows {
        if initialized {
            println!("{{\"kind\":\"cache\",\"phase\":\"{phase}\",\"family\":\"ntt\",\"class\":{class},\"initialized\":true,\"data_len_bytes\":{data_len},\"data_capacity_bytes\":{data_capacity_bytes}}}");
        }
    }
    let [small, large] = internals::fixed_matrix_table_bytes();
    println!("{{\"kind\":\"cache-header\",\"phase\":\"{phase}\",\"family\":\"fixed-32-layout\",\"header_bytes\":{small}}}");
    println!("{{\"kind\":\"cache-header\",\"phase\":\"{phase}\",\"family\":\"fixed-64-layout\",\"header_bytes\":{large}}}");
}

#[cfg(feature = "count-allocations")]
fn allocations(phase: &str, stats: allocator::Stats) {
    print!("{{\"kind\":\"allocations\",\"phase\":\"{phase}\"");
    for (name, value) in allocator::NAMES.iter().zip(stats.counters) {
        print!(",\"{name}\":{value}");
    }
    let before = stats.live_before;
    let peak = stats.peak;
    let after = stats.live_after;
    let extra = peak.checked_sub(before).unwrap();
    let retained = after as i128 - before as i128;
    let [encode, decode, ntt] = cache_bytes();
    println!(",\"live_before_bytes\":{before},\"peak_live_bytes\":{peak},\"live_after_bytes\":{after},\"peak_extra_bytes\":{extra},\"retained_delta_bytes\":{retained},\"encode_cache_bytes\":{encode},\"decode_cache_bytes\":{decode},\"ntt_cache_bytes\":{ntt}}}");
}

fn timed(mut call: impl FnMut() -> usize, iterations: usize) -> (usize, u128, u128) {
    let cpu = cpu_ns();
    let start = Instant::now();
    let mut len = 0;
    for _ in 0..iterations { len = black_box(call()); }
    let elapsed = start.elapsed().as_nanos();
    (len, elapsed, cpu_ns() - cpu)
}

fn more_iterations(iterations: usize, primary_ns: u128) -> usize {
    assert!(iterations < MAX_ITERATIONS, "minimum batch duration unreachable at iteration cap");
    let factor = (MIN_BATCH_NS * 5).div_ceil(primary_ns.max(1).saturating_mul(4)).clamp(2, 16) as usize;
    iterations.saturating_mul(factor).min(MAX_ITERATIONS)
}

fn initial_iterations(second_cpu: u128) -> usize {
    // Single-call CPU clocks can read zero; calibrate before estimating a large batch.
    if second_cpu >= MIN_BATCH_NS {
        1
    } else {
        (MIN_BATCH_NS * 5).div_ceil(second_cpu.max(1) * 4).clamp(1, 16) as usize
    }
}

#[cfg(test)]
#[test]
fn calibration_growth_is_bounded_and_cap_failure_is_explicit() {
    assert_eq!(initial_iterations(0), 16);
    assert_eq!(initial_iterations(1), 16);
    assert_eq!(initial_iterations(MIN_BATCH_NS - 1), 2);
    assert_eq!(initial_iterations(MIN_BATCH_NS), 1);
    assert_eq!(more_iterations(1, 0), 16);
    assert_eq!(more_iterations(100, 1_000_000), 400);
    assert_eq!(more_iterations(MAX_ITERATIONS - 1, 1), MAX_ITERATIONS);
    assert!(std::panic::catch_unwind(|| more_iterations(MAX_ITERATIONS, 1)).is_err());
}

fn timing_row(phase: &str, sample: usize, iterations: usize, wall: u128, cpu: u128) {
    println!("{{\"kind\":\"timing\",\"phase\":\"{phase}\",\"sample\":{sample},\"iterations\":{iterations},\"wall_ns\":{wall},\"cpu_ns\":{cpu}}}");
}

fn worker(mode: &str, operation: &str, backend: &str, size: usize, samples: usize) {
    match (mode, cfg!(feature = "count-allocations")) {
        ("timing", true) => panic!("timing requires a build without count-allocations"),
        ("allocations", false) => panic!("allocations requires --features count-allocations"),
        _ => {},
    }
    assert!(size <= MAX_INPUT_BYTES, "maximum input is 4 MiB");
    assert!(samples > 0);
    assert!(matches!(operation, "encode" | "decode"));
    let &(_, encode, decode) = BACKENDS.iter().find(|b| b.0 == backend).expect("unknown backend");
    assert!(supports(backend, operation, size), "unsupported backend byte count");
    assert_eq!(cache_bytes(), [0; 3], "worker must start without cache initialization");
    println!("{{\"kind\":\"context\",\"operation\":\"{operation}\",\"backend\":\"{backend}\",\"bytes\":{size}}}");
    #[cfg(feature = "count-allocations")]
    let (input, input_stats) = allocator::measure(|| data(size));
    #[cfg(not(feature = "count-allocations"))]
    let input = data(size);
    #[cfg(feature = "count-allocations")]
    let (mut encoded, encoded_stats) = allocator::measure(|| vec![0; b58::max_encoded_len(size)]);
    #[cfg(not(feature = "count-allocations"))]
    let mut encoded = vec![0; b58::max_encoded_len(size)];
    let encoded_len = encode_fixture(&input, &mut encoded);
    encoded.truncate(encoded_len);
    assert_eq!(cache_bytes(), [0; 3], "fixture initialized a cache");
    let text = std::str::from_utf8(&encoded).unwrap();
    if backend == "matrix-forced" {
        assert!(if operation == "encode" { (1..=8192).contains(&size) } else { (1..=5632).contains(&text.len()) }, "outside forced matrix kernel domain");
    }
    let output_size = if operation == "encode" { b58::max_encoded_len(size) } else { size };
    #[cfg(feature = "count-allocations")]
    let (mut output, output_stats) = allocator::measure(|| vec![0u8; output_size]);
    #[cfg(not(feature = "count-allocations"))]
    let mut output = vec![0u8; output_size];
    let (input_capacity, fixture_capacity) = if operation == "encode" {
        (input.capacity(), encoded.capacity())
    } else {
        (encoded.capacity(), input.capacity())
    };
    println!("{{\"kind\":\"io\",\"input_capacity_bytes\":{input_capacity},\"output_capacity_bytes\":{},\"fixture_capacity_bytes\":{fixture_capacity},\"encoded_chars\":{},\"data_fnv1a64\":\"{:016x}\",\"encoded_fnv1a64\":\"{:016x}\"}}", output.capacity(), encoded.len(), fnv1a64(&input), fnv1a64(&encoded));
    let expected = if operation == "encode" { &encoded[..] } else { &input[..] };
    let call = |out: &mut [u8]| {
        if operation == "encode" { encode(black_box(&input), black_box(out)).unwrap() }
        else { decode(black_box(text), black_box(out)).unwrap() }
    };
    if mode == "allocations" {
        #[cfg(feature = "count-allocations")]
        {
            allocations("io-data", input_stats);
            allocations("io-encoded", encoded_stats);
            allocations("io-output", output_stats);
            for phase in ["cold", "warm"] {
                let (len, stats) = allocator::measure(|| call(&mut output));
                assert_eq!(&output[..len], expected);
                assert_eq!(len, expected.len());
                allocations(phase, stats);
                caches(phase);
            }
        }
        #[cfg(not(feature = "count-allocations"))]
        panic!("allocations requires --features count-allocations");
    } else {
        assert_eq!(mode, "timing");
        let mut second_cpu = 0;
        for phase in ["cold", "second"] {
            let (len, wall, cpu) = timed(|| call(&mut output), 1);
            assert_eq!(&output[..len], expected);
            assert_eq!(len, expected.len());
            timing_row(phase, 0, 1, wall, cpu);
            second_cpu = cpu;
        }
        let mut iterations = initial_iterations(second_cpu);
        let mut sample = 0;
        while sample < samples {
            let (len, wall, cpu) = timed(|| call(&mut output), iterations);
            assert_eq!(&output[..len], expected);
            assert_eq!(len, expected.len());
            let phase = if cpu >= MIN_BATCH_NS { "warm" } else { "calibration" };
            timing_row(phase, sample, iterations, wall, cpu);
            if phase == "warm" { sample += 1; }
            else { iterations = more_iterations(iterations, cpu); }
        }
        caches("warm");
    }
}

fn inventory(operation: &str, size: usize) {
    assert_eq!(cache_bytes(), [0; 3]);
    match operation {
        "encode" => {
            assert!((1..=8192).contains(&size));
            let input = data(size);
            let expected = fixture(&input);
            let mut out = vec![0; b58::max_encoded_len(size)];
            let len = internals::encode_matrix_whole(&input, &mut out).unwrap();
            assert_eq!(&out[..len], expected);
        }
        "decode" => {
            assert!((1..=5632).contains(&size));
            let input = "z".repeat(size);
            let mut expected = vec![0; size];
            let len = malachite::decode_malachite(&input, &mut expected).unwrap();
            expected.truncate(len);
            let mut out = vec![0; size];
            let len = internals::decode_matrix_forced(&input, &mut out).unwrap();
            assert_eq!(&out[..len], expected);
        }
        _ => panic!("inventory expects encode BYTES or decode CHARACTERS"),
    }
    caches("inventory");
}

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("describe") => describe(),
        Some("fixture-metadata") if args.len() > 1 => {
            for argument in &args[1..] {
                let size: usize = argument.parse().unwrap();
                assert!(size <= MAX_INPUT_BYTES);
                let encoded = fixture(&data(size));
                println!("{{\"bytes\":{size},\"encoded_chars\":{}}}", encoded.len());
            }
        }
        Some("list") => {
            for &(name, _, _) in BACKENDS {
                let encode_max = max_bytes(name, "encode");
                let decode_max = max_bytes(name, "decode");
                let exact = match name {
                    "five8" => ",\"supported_encode_bytes\":[32,64],\"supported_decode_bytes\":[32,64]",
                    "matrix-forced" => ",\"min_encode_bytes\":1,\"min_decode_bytes\":1,\"min_decode_chars\":1,\"max_decode_chars\":5632",
                    _ => "",
                };
                println!("{{\"backend\":\"{name}\",\"max_encode_bytes\":{encode_max},\"max_decode_bytes\":{decode_max}{exact}}}");
            }
        }
        Some("self-check") => {
            #[cfg(feature = "count-allocations")]
            allocator::self_check();
            check_fixtures();
            check_competitors();
            for size in [0, 1, 32, 64, 513] {
                let data = data(size);
                let encoded = fixture(&data);
                let text = std::str::from_utf8(&encoded).unwrap();
                for &(name, encode, decode) in BACKENDS {
                    if !supports(name, "encode", size) || !supports(name, "decode", size) { continue; }
                    let mut out = vec![0; b58::max_encoded_len(size)];
                    let len = encode(&data, &mut out).unwrap();
                    assert_eq!(&out[..len], encoded);
                    let mut out = vec![0; size];
                    let len = decode(text, &mut out).unwrap();
                    assert_eq!(&out[..len], data);
                }
            }
            let before = cpu_ns();
            assert!(cpu_ns() >= before);
            println!("self-check passed");
        }
        Some("inventory") if args.len() == 3 => inventory(&args[1], args[2].parse().unwrap()),
        Some("allocations" | "timing") if args.len() == 5 => worker(&args[0], &args[1], &args[2], args[3].parse().unwrap(), args[4].parse().unwrap()),
        _ => panic!("usage: resource-profile describe|list|self-check|fixture-metadata BYTES...|inventory encode|decode SIZE|allocations|timing encode|decode BACKEND BYTES SAMPLES"),
    }
}
