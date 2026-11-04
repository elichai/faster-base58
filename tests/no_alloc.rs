#[cfg(any(not(feature = "alloc"), feature = "check"))]
use faster_base58::{BufferTooSmall, Error};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
#[cfg(not(feature = "alloc"))]
use std::sync::atomic::AtomicBool;

struct Counting;

#[global_allocator]
static ALLOCATOR: Counting = Counting;
thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static CALLS: Cell<[usize; 4]> = const { Cell::new([0; 4]) };
    static LIVE: Cell<isize> = const { Cell::new(0) };
    static PEAK: Cell<isize> = const { Cell::new(0) };
}

fn record(kind: usize) {
    if ACTIVE.get() {
        let mut calls = CALLS.get();
        calls[kind] += 1;
        CALLS.set(calls);
    }
}

fn record_bytes(allocated: usize, deallocated: usize) {
    if ACTIVE.get() {
        let live = LIVE.get() - deallocated as isize + allocated as isize;
        LIVE.set(live);
        PEAK.set(PEAK.get().max(live));
    }
}

// SAFETY: each method delegates the unchanged allocation/layout to System.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(0);
        // SAFETY: the caller supplies the valid layout required by GlobalAlloc::alloc.
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            record_bytes(layout.size(), 0);
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(1);
        // SAFETY: the caller supplies the valid layout required by GlobalAlloc::alloc_zeroed.
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            record_bytes(layout.size(), 0);
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record(2);
        // SAFETY: forward the caller's live allocation, matching layout, and valid new size.
        let ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if !ptr.is_null() {
            record_bytes(new_size, layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        record(3);
        record_bytes(0, layout.size());
        // SAFETY: forward the caller's live System allocation and matching layout.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[cfg(any(
    not(feature = "alloc"),
    feature = "check",
    feature = "__internals",
    all(
        target_arch = "aarch64",
        target_endian = "little",
        target_feature = "neon"
    )
))]
fn observe<T>(call: impl FnOnce() -> T) -> (T, [usize; 4]) {
    assert!(!ACTIVE.get());
    CALLS.set([0; 4]);
    LIVE.set(0);
    PEAK.set(0);
    ACTIVE.set(true);
    let result = call();
    ACTIVE.set(false);
    (result, CALLS.get())
}

#[cfg(any(not(feature = "alloc"), feature = "check"))]
fn without_allocations<T>(call: impl FnOnce() -> T) -> T {
    let (result, calls) = observe(call);
    assert_eq!(
        calls, [0; 4],
        "allocator calls: alloc, zeroed, realloc, dealloc"
    );
    result
}

#[cfg(all(feature = "check", not(feature = "alloc")))]
#[test]
fn checked_conversion_without_allocating_backends() {
    use faster_base58::{Decoder, Encoder};
    for len in [0, 1, 21, 27, 28, 59, 60, 512, 8193] {
        for version in [None, Some(0), Some(42)] {
            let input = vec![0xa5; len];
            let encoder = version.map_or(Encoder::bitcoin().with_check(), |v| {
                Encoder::bitcoin().with_check_version(v)
            });
            let decoder = version.map_or(Decoder::bitcoin().with_check(), |v| {
                Decoder::bitcoin().with_check_version(v)
            });
            let expected = match version {
                Some(v) => bs58::encode(&input).with_check_version(v).into_string(),
                None => bs58::encode(&input).with_check().into_string(),
            };
            let payload = bs58::decode(&expected)
                .with_check(version)
                .into_vec()
                .unwrap();
            let mut text = vec![0xa5; expected.len() + 1];
            assert_eq!(
                without_allocations(|| encoder.encode(&input).onto(&mut text)).unwrap(),
                expected
            );
            assert_eq!(text[expected.len()], 0xa5);
            let mut output = vec![0xa5; payload.len() + 1];
            assert_eq!(
                without_allocations(|| decoder
                    .decode(&expected)
                    .onto(&mut output[..payload.len()]))
                .unwrap(),
                payload
            );
            assert_eq!(output[payload.len()], 0xa5);
            assert_eq!(
                without_allocations(|| decoder.decode("0").onto(&mut output)),
                Err(Error::InvalidCharacter { index: 0 })
            );
            if !payload.is_empty() {
                assert_eq!(
                    without_allocations(|| decoder
                        .decode(&expected)
                        .onto(&mut output[..payload.len() - 1])),
                    Err(Error::BufferTooSmall)
                );
            }
        }
    }
    assert_eq!(
        without_allocations(|| Decoder::bitcoin()
            .with_check()
            .decode("3QJmnh")
            .into_array::<0>()),
        Ok([])
    );
}

#[cfg(not(feature = "alloc"))]
fn cold_paths() {
    let mut fixed = [0xA5; 96];
    assert_eq!(
        without_allocations(|| faster_base58::Encoder::bitcoin()
            .encode(&[0xFF; 32])
            .onto(&mut [])
            .map(str::len)),
        Err(BufferTooSmall)
    );
    assert_eq!(
        without_allocations(|| faster_base58::Encoder::bitcoin()
            .encode(&[0xFF; 64])
            .onto(&mut fixed)
            .map(str::len)),
        Ok(88)
    );
    assert_eq!(
        without_allocations(|| faster_base58::Decoder::bitcoin()
            .decode("é")
            .onto(&mut fixed)
            .map(|bytes| bytes.len())),
        Err(Error::InvalidCharacter { index: 0 })
    );

    // No API call warms the general conversion paths before these observations.
    // Fixture generation and caller-owned buffers are deliberately outside counting.
    let input = [0xA5; 8193];
    let mut text = [0xA5; 12_300];
    let mut decoded = [0xA5; 8193];
    let (encoded, encode_calls) = observe(|| {
        faster_base58::Encoder::bitcoin()
            .encode(&input)
            .onto(&mut text)
            .map(str::len)
    });
    let len = encoded.unwrap();
    let text = std::str::from_utf8(&text[..len]).unwrap();
    let (result, decode_calls) = observe(|| {
        faster_base58::Decoder::bitcoin()
            .decode(text)
            .onto(&mut decoded)
            .map(|bytes| bytes.len())
    });
    assert_eq!(result, Ok(input.len()));
    assert_eq!(decoded, input);
    assert_eq!(
        (encode_calls, decode_calls),
        ([0; 4], [0; 4]),
        "cold allocator calls"
    );
}

#[cfg(not(feature = "alloc"))]
fn check_buffers(input: &[u8]) {
    let expected = bs58::encode(input).into_string();
    for capacity in [
        expected.len().saturating_sub(1),
        expected.len(),
        expected.len() + 5,
    ] {
        for borrowed in [false, true] {
            let mut output = vec![0xA5; capacity + 2];
            let result = if borrowed {
                without_allocations(|| {
                    faster_base58::Encoder::bitcoin()
                        .encode(input)
                        .onto(&mut output[1..1 + capacity])
                })
                .map(|text| {
                    assert_eq!(text, expected);
                    text.len()
                })
            } else {
                without_allocations(|| {
                    faster_base58::Encoder::bitcoin()
                        .encode(input)
                        .onto(&mut output[1..1 + capacity])
                        .map(str::len)
                })
            };
            if capacity < expected.len() {
                assert_eq!(result, Err(BufferTooSmall));
            } else {
                assert_eq!(result, Ok(expected.len()));
                assert_eq!(&output[1..1 + expected.len()], expected.as_bytes());
                assert!(
                    output[1 + expected.len()..]
                        .iter()
                        .all(|&byte| byte == 0xA5)
                );
            }
            assert_eq!(output[0], 0xA5);
            assert_eq!(output[capacity + 1], 0xA5);
        }
    }

    for capacity in [input.len().saturating_sub(1), input.len(), input.len() + 5] {
        for borrowed in [false, true] {
            let mut output = vec![0xA5; capacity + 2];
            let result = if borrowed {
                without_allocations(|| {
                    faster_base58::Decoder::bitcoin()
                        .decode(&expected)
                        .onto(&mut output[1..1 + capacity])
                })
                .map(|bytes| {
                    assert_eq!(bytes, input);
                    bytes.len()
                })
            } else {
                without_allocations(|| {
                    faster_base58::Decoder::bitcoin()
                        .decode(&expected)
                        .onto(&mut output[1..1 + capacity])
                        .map(|bytes| bytes.len())
                })
            };
            if capacity < input.len() {
                assert_eq!(result, Err(Error::BufferTooSmall));
            } else {
                assert_eq!(result, Ok(input.len()));
                assert_eq!(&output[1..1 + input.len()], input);
                assert!(output[1 + input.len()..].iter().all(|&byte| byte == 0xA5));
            }
            assert_eq!(output[0], 0xA5);
            assert_eq!(output[capacity + 1], 0xA5);
        }
    }
}

#[cfg(not(feature = "alloc"))]
fn differential_boundaries() {
    for size in [
        0, 1, 2, 6, 7, 8, 9, 10, 13, 14, 15, 16, 21, 22, 23, 31, 32, 33, 63, 64, 65, 255, 256, 257,
        511, 512, 513, 575, 576, 577, 1023, 1024, 1025, 4095, 4096, 4097, 8191, 8192, 8193,
    ] {
        let mut input: Vec<u8> = (0..size).map(|index| (index * 131 + 17) as u8).collect();
        check_buffers(&input);
        input.fill(0xFF);
        check_buffers(&input);
        input.fill(0);
        check_buffers(&input);
    }
    // Every leading-zero count, including all-zero fixed-size inputs.
    for size in [32, 64] {
        let mut input = vec![0xFF; size];
        for zeros in 0..=size {
            input[..zeros].fill(0);
            check_buffers(&input);
        }
    }
    for byte in 0..=255 {
        check_buffers(&[byte]);
    }
}

#[cfg(not(feature = "alloc"))]
fn invalid_byte_offsets() {
    for (text, index) in [
        ("0", 0),
        ("O", 0),
        ("1I", 1),
        ("l1", 0),
        ("é", 0),
        ("1é", 1),
        ("1é0", 1),
        ("1111🦀", 4),
        ("\0", 0),
        ("11111111111é", 11),
    ] {
        let mut output = [0xA5; 32];
        assert_eq!(
            without_allocations(|| faster_base58::Decoder::bitcoin()
                .decode(text)
                .onto(&mut output[1..31])
                .map(|bytes| bytes.len())),
            Err(Error::InvalidCharacter { index }),
        );
        assert_eq!(output[0], 0xA5);
        assert_eq!(output[31], 0xA5);
    }
    // Raw bytes need not be UTF-8.
    for (text, index) in [(&b"\x80"[..], 0), (b"2\xC3", 1), (b"1112\xFF", 4)] {
        assert_eq!(
            without_allocations(|| faster_base58::Decoder::bitcoin()
                .decode(text)
                .onto(&mut [0; 8])
                .map(|bytes| bytes.len())),
            Err(Error::InvalidCharacter { index }),
        );
    }
    for index in [0, 1, 9, 10, 19, 20, 575, 576, 5631, 5632, 8192] {
        let mut text = vec![b'2'; 8193];
        text[index] = b'0';
        let text = std::str::from_utf8(&text).unwrap();
        let mut output = [0xA5; 8195];
        assert_eq!(
            without_allocations(|| faster_base58::Decoder::bitcoin()
                .decode(text)
                .onto(&mut output[1..8194])),
            Err(Error::InvalidCharacter { index }),
        );
        assert_eq!(output[0], 0xA5);
        assert_eq!(output[8194], 0xA5);
    }
}

#[cfg(not(feature = "alloc"))]
fn borrowed_and_array_contracts() {
    let mut output = [0xA5; 8];
    let start = output.as_ptr();
    let text = without_allocations(|| {
        faster_base58::Encoder::bitcoin()
            .encode(&[0, 0, 1])
            .onto(&mut output)
    });
    let text = text.unwrap();
    assert_eq!(text, "112");
    assert_eq!(text.as_ptr(), start);
    let decoded = without_allocations(|| {
        faster_base58::Decoder::bitcoin()
            .decode("112")
            .onto(&mut output)
    })
    .unwrap();
    assert_eq!(decoded, [0, 0, 1]);
    assert_eq!(decoded.as_ptr(), start);
    decoded[0] = 9;
    assert_eq!(output[0], 9);

    let input = vec![0, 0, 1];
    let text = without_allocations(|| {
        faster_base58::Encoder::bitcoin()
            .encode(&input)
            .onto(&mut output)
    })
    .unwrap();
    drop(input);
    assert_eq!(text, "112");

    assert_eq!(
        without_allocations(|| faster_base58::Decoder::bitcoin()
            .decode("")
            .into_array::<0>()),
        Ok([])
    );
    assert_eq!(
        without_allocations(|| faster_base58::Decoder::bitcoin()
            .decode("1")
            .into_array::<1>()),
        Ok([0])
    );
    assert_eq!(
        without_allocations(|| faster_base58::Decoder::bitcoin()
            .decode("11")
            .into_array::<2>()),
        Ok([0, 0])
    );
    assert_eq!(
        without_allocations(|| faster_base58::Decoder::bitcoin()
            .decode("1")
            .into_array::<0>()),
        Err(Error::BufferTooSmall)
    );
    assert_eq!(
        without_allocations(|| faster_base58::Decoder::bitcoin()
            .decode("11")
            .into_array::<1>()),
        Err(Error::BufferTooSmall)
    );
    assert_eq!(
        without_allocations(|| faster_base58::Decoder::bitcoin()
            .decode("2")
            .into_array::<2>()),
        Err(Error::InvalidLength {
            expected: 2,
            actual: 1
        }),
    );
    assert_eq!(
        without_allocations(|| faster_base58::Decoder::bitcoin()
            .decode("")
            .into_array::<1>()),
        Err(Error::InvalidLength {
            expected: 1,
            actual: 0
        }),
    );
    assert_eq!(
        without_allocations(|| faster_base58::Decoder::bitcoin()
            .decode("1é")
            .into_array::<8>()),
        Err(Error::InvalidCharacter { index: 1 }),
    );
    let text = bs58::encode([0xFF; 32]).into_string();
    assert_eq!(
        without_allocations(|| faster_base58::Decoder::bitcoin()
            .decode(&text)
            .into_array::<32>()),
        Ok([0xFF; 32])
    );
    assert_eq!(
        without_allocations(|| faster_base58::Decoder::bitcoin()
            .decode(&text)
            .into_array::<31>()),
        Err(Error::BufferTooSmall)
    );
    assert_eq!(
        without_allocations(|| faster_base58::Decoder::bitcoin()
            .decode(&text)
            .into_array::<33>()),
        Err(Error::InvalidLength {
            expected: 33,
            actual: 32
        }),
    );
}

#[cfg(not(feature = "alloc"))]
fn allocation_counter_is_thread_local() {
    use std::sync::atomic::Ordering::{Acquire, Release};

    let started = AtomicBool::new(false);
    let finished = AtomicBool::new(false);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            while !started.load(Acquire) {
                std::hint::spin_loop();
            }
            std::hint::black_box(vec![0xA5; 128]);
            finished.store(true, Release);
        });
        without_allocations(|| {
            started.store(true, Release);
            while !finished.load(Acquire) {
                std::hint::spin_loop();
            }
        });
    });
    let (value, calls) = observe(|| std::hint::black_box(vec![0xA5u8; 128]));
    assert_eq!(PEAK.get(), 128);
    assert!(
        calls[0] + calls[1] > 0,
        "the observed thread must still count"
    );
    drop(value);
}

#[cfg(all(feature = "check", feature = "alloc"))]
#[test]
fn checked_encoding_uses_only_output_and_normal_codec_workspace() {
    use faster_base58::Encoder;
    for version in [None, Some(42)] {
        let encoder = version.map_or(Encoder::bitcoin().with_check(), |v| {
            Encoder::bitcoin().with_check_version(v)
        });
        for len in [0, 1, 16, 17, 27, 28, 59, 60, 4096, 8193, 16379] {
            let input = vec![0xff; len];
            let expected = match version {
                Some(v) => bs58::encode(&input).with_check_version(v).into_string(),
                None => bs58::encode(&input).with_check().into_string(),
            };
            let frame = faster_base58::Decoder::bitcoin()
                .decode(&expected)
                .into_vec()
                .unwrap();
            let mut output = vec![0xa5; expected.len() + 7];
            // Warm optional caches before comparing per-call allocation counts.
            faster_base58::Encoder::bitcoin()
                .encode(&frame)
                .onto(&mut output)
                .map(str::len)
                .unwrap();
            let (plain, baseline) = observe(|| {
                faster_base58::Encoder::bitcoin()
                    .encode(&frame)
                    .onto(&mut output)
                    .map(str::len)
            });
            assert_eq!(plain, Ok(expected.len()));
            output.fill(0xa5);
            let (checked, calls) = observe(|| encoder.encode(&input).onto(&mut output));
            assert_eq!(checked.unwrap(), expected);
            assert_eq!(
                calls, baseline,
                "extra allocator calls for check at {len}, {version:?}"
            );
            assert!(output[expected.len()..].iter().all(|&b| b == 0xa5));
        }
        let mut output = [0; 1];
        assert_eq!(
            without_allocations(|| encoder.encode([42; 32]).onto(&mut output)),
            Err(BufferTooSmall)
        );
        let (text, calls) = observe(|| encoder.encode([0; 21]).into_string());
        assert_eq!(
            calls[0] + calls[1],
            1,
            "only the returned string needs allocation"
        );
        assert_eq!(calls[2..], [0, 0]);
        assert!(!text.is_empty());
    }
}

#[cfg(all(feature = "check", feature = "alloc"))]
#[test]
fn checked_arrays_use_stack_checksum_storage() {
    use faster_base58::{Decoder, Encoder};
    let input = [42; 16];
    let encoder = Encoder::bitcoin().with_check();
    let decoder = Decoder::bitcoin().with_check();
    let text = encoder.encode(input).into_string();
    assert_eq!(
        without_allocations(|| decoder.decode(&text).into_array::<16>()),
        Ok(input)
    );
    assert_eq!(
        without_allocations(|| decoder.decode("3QJmnh").into_array::<0>()),
        Ok([])
    );
    assert_eq!(
        without_allocations(|| decoder.decode("0").into_array::<16>()),
        Err(Error::InvalidCharacter { index: 0 })
    );
    let text = encoder.with_check_version(0).encode(input).into_string();
    let mut expected = [42; 17];
    expected[0] = 0;
    assert_eq!(
        without_allocations(|| decoder
            .with_check_version(0)
            .decode(&text)
            .into_array::<17>()),
        Ok(expected)
    );
    assert_eq!(
        without_allocations(|| decoder
            .with_check_version(1)
            .decode(&text)
            .into_array::<17>()),
        Err(Error::InvalidVersion {
            expected: 1,
            actual: Some(0)
        })
    );
}

#[cfg(all(feature = "check", feature = "alloc"))]
#[test]
fn checked_decoding_uses_exact_payload_storage_without_extra_heap() {
    use faster_base58::{Decoder, Encoder};
    for version in [None, Some(0), Some(42)] {
        let encoder = version.map_or(Encoder::bitcoin().with_check(), |v| {
            Encoder::bitcoin().with_check_version(v)
        });
        let decoder = version.map_or(Decoder::bitcoin().with_check(), |v| {
            Decoder::bitcoin().with_check_version(v)
        });
        for len in [0, 1, 16, 21, 32, 512, 4096, 8193] {
            let input = vec![42; len];
            let text = encoder.encode(&input).into_string();
            let expected = bs58::decode(&text).with_check(version).into_vec().unwrap();
            let mut framed = vec![0; expected.len() + 4];
            faster_base58::Decoder::bitcoin()
                .decode(&text)
                .onto(&mut framed)
                .map(|bytes| bytes.len())
                .unwrap();
            let (_, baseline) = observe(|| {
                faster_base58::Decoder::bitcoin()
                    .decode(&text)
                    .onto(&mut framed)
                    .map(|bytes| bytes.len())
            });
            let mut output = vec![0xa5; expected.len() + 7];
            let (result, calls) =
                observe(|| decoder.decode(&text).onto(&mut output[..expected.len()]));
            assert_eq!(result.unwrap(), expected);
            assert_eq!(
                calls, baseline,
                "extra check allocator calls at {len}, {version:?}"
            );
            assert_eq!(&output[expected.len()..], &[0xa5; 7]);
        }
    }
}

// One process keeps first use cold; thread-local counters exclude libtest activity.
// Assertions/oracle/caller-buffer setup stay outside counting regions.
#[test]
#[cfg(not(feature = "alloc"))]
fn no_allocator_calls_on_cold_success_or_error_paths() {
    cold_paths();
    differential_boundaries();
    invalid_byte_offsets();
    borrowed_and_array_contracts();
    allocation_counter_is_thread_local();
}

#[cfg(all(
    feature = "alloc",
    target_arch = "aarch64",
    target_endian = "little",
    target_feature = "neon"
))]
#[test]
fn large_encoding_does_not_allocate_per_leaf() {
    for bytes in [1 << 20, 1 << 22] {
        let input = vec![255; bytes];
        let mut output =
            vec![0xA5; faster_base58::Encoder::bitcoin().max_encoded_len(input.len()) + 8];
        let (result, calls) = observe(|| {
            faster_base58::Encoder::bitcoin()
                .encode(&input)
                .onto(&mut output[1..])
                .map(str::len)
        });
        let peak = PEAK.get();
        let len = result.unwrap();
        let mib = if bytes == 1 << 20 { 18 } else { 67 };
        assert!(
            peak <= mib * 1024 * 1024,
            "large encoding peak: {peak} bytes"
        );
        assert!(
            calls[0] + calls[1] + calls[2] <= 7,
            "large encoding allocated per leaf: {calls:?}"
        );
        assert_eq!(output[0], 0xA5);
        assert!(output[1 + len..].iter().all(|&c| c == 0xA5));
        let mut decoded = vec![0; input.len()];
        let text = std::str::from_utf8(&output[1..1 + len]).unwrap();
        assert_eq!(
            faster_base58::Decoder::bitcoin()
                .decode(text)
                .onto(&mut decoded)
                .map(|bytes| bytes.len()),
            Ok(input.len())
        );
        assert_eq!(decoded, input);
    }
}

#[cfg(feature = "__internals")]
#[test]
fn owned_ntt_tables_use_one_allocation_per_prime() {
    let input = [0xA5; 4096];
    let text = bs58::encode(input).into_string();
    let mut output = [0xA5; 4098];
    let (result, calls) = observe(|| {
        faster_base58::internals::decode_ntt_with(&text, &mut output[1..4097], false, 1, usize::MAX)
    });
    assert_eq!(result, Ok(input.len()));
    assert_eq!(output[1..4097], input);
    assert_eq!((output[0], output[4097]), (0xA5, 0xA5));
    assert!(
        calls[0] + calls[1] <= 4,
        "three prime tables and one conversion workspace: {calls:?}"
    );
    assert_eq!(calls[2], 0, "fixed table storage must not reallocate");
}
