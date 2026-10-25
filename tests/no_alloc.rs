use base58_bench::{BufferTooSmall, Error, no_alloc};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};

struct Counting;

#[global_allocator]
static ALLOCATOR: Counting = Counting;
static ACTIVE: AtomicBool = AtomicBool::new(false);
static CALLS: [AtomicUsize; 4] = [const { AtomicUsize::new(0) }; 4];

fn record(kind: usize) {
    if ACTIVE.load(Relaxed) {
        CALLS[kind].fetch_add(1, Relaxed);
    }
}

// SAFETY: each method delegates the unchanged allocation/layout to System.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(0);
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(1);
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record(2);
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        record(3);
        unsafe { System.dealloc(ptr, layout) }
    }
}

fn observe<T>(call: impl FnOnce() -> T) -> (T, [usize; 4]) {
    assert!(!ACTIVE.load(Relaxed));
    for count in &CALLS {
        count.store(0, Relaxed);
    }
    ACTIVE.store(true, Relaxed);
    let result = call();
    ACTIVE.store(false, Relaxed);
    (result, std::array::from_fn(|i| CALLS[i].load(Relaxed)))
}

fn without_allocations<T>(call: impl FnOnce() -> T) -> T {
    let (result, calls) = observe(call);
    assert_eq!(
        calls, [0; 4],
        "allocator calls: alloc, zeroed, realloc, dealloc"
    );
    result
}

fn cold_paths() {
    let mut fixed = [0xA5; 96];
    assert_eq!(
        without_allocations(|| no_alloc::encode(&[0xFF; 32], &mut [])),
        Err(BufferTooSmall)
    );
    assert_eq!(
        without_allocations(|| no_alloc::encode(&[0xFF; 64], &mut fixed)),
        Ok(88)
    );
    assert_eq!(
        without_allocations(|| no_alloc::decode("é", &mut fixed)),
        Err(Error::InvalidCharacter { index: 0 })
    );

    // No API call warms the general conversion paths before these observations.
    // Fixture generation and caller-owned buffers are deliberately outside counting.
    let input = [0xA5; 8193];
    let mut text = [0xA5; 12_300];
    let mut decoded = [0xA5; 8193];
    let (encoded, encode_calls) = observe(|| no_alloc::encode(&input, &mut text));
    let len = encoded.unwrap();
    let text = std::str::from_utf8(&text[..len]).unwrap();
    let (result, decode_calls) = observe(|| no_alloc::decode(text, &mut decoded));
    assert_eq!(result, Ok(input.len()));
    assert_eq!(decoded, input);
    assert_eq!(
        (encode_calls, decode_calls),
        ([0; 4], [0; 4]),
        "cold allocator calls"
    );
}

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
                without_allocations(|| no_alloc::encode_into(input, &mut output[1..1 + capacity]))
                    .map(|text| {
                        assert_eq!(text, expected);
                        text.len()
                    })
            } else {
                without_allocations(|| no_alloc::encode(input, &mut output[1..1 + capacity]))
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
                    no_alloc::decode_into(&expected, &mut output[1..1 + capacity])
                })
                .map(|bytes| {
                    assert_eq!(bytes, input);
                    bytes.len()
                })
            } else {
                without_allocations(|| no_alloc::decode(&expected, &mut output[1..1 + capacity]))
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
            without_allocations(|| no_alloc::decode(text, &mut output[1..31])),
            Err(Error::InvalidCharacter { index }),
        );
        assert_eq!(output[0], 0xA5);
        assert_eq!(output[31], 0xA5);
    }
    for index in [0, 1, 9, 10, 19, 20, 575, 576, 5631, 5632, 8192] {
        let mut text = vec![b'2'; 8193];
        text[index] = b'0';
        let text = std::str::from_utf8(&text).unwrap();
        let mut output = [0xA5; 8195];
        assert_eq!(
            without_allocations(|| no_alloc::decode_into(text, &mut output[1..8194])),
            Err(Error::InvalidCharacter { index }),
        );
        assert_eq!(output[0], 0xA5);
        assert_eq!(output[8194], 0xA5);
    }
}

fn borrowed_and_array_contracts() {
    let mut output = [0xA5; 8];
    let start = output.as_ptr();
    let text = without_allocations(|| no_alloc::encode_into(&[0, 0, 1], &mut output));
    let text = text.unwrap();
    assert_eq!(text, "112");
    assert_eq!(text.as_ptr(), start);
    let decoded = without_allocations(|| no_alloc::decode_into("112", &mut output)).unwrap();
    assert_eq!(decoded, [0, 0, 1]);
    assert_eq!(decoded.as_ptr(), start);
    decoded[0] = 9;
    assert_eq!(output[0], 9);

    let input = vec![0, 0, 1];
    let text = without_allocations(|| no_alloc::encode_into(&input, &mut output)).unwrap();
    drop(input);
    assert_eq!(text, "112");

    assert_eq!(
        without_allocations(|| no_alloc::decode_array::<0>("")),
        Ok([])
    );
    assert_eq!(
        without_allocations(|| no_alloc::decode_array::<1>("1")),
        Ok([0])
    );
    assert_eq!(
        without_allocations(|| no_alloc::decode_array::<2>("11")),
        Ok([0, 0])
    );
    assert_eq!(
        without_allocations(|| no_alloc::decode_array::<0>("1")),
        Err(Error::BufferTooSmall)
    );
    assert_eq!(
        without_allocations(|| no_alloc::decode_array::<1>("11")),
        Err(Error::BufferTooSmall)
    );
    assert_eq!(
        without_allocations(|| no_alloc::decode_array::<2>("2")),
        Err(Error::InvalidLength {
            expected: 2,
            actual: 1
        }),
    );
    assert_eq!(
        without_allocations(|| no_alloc::decode_array::<1>("")),
        Err(Error::InvalidLength {
            expected: 1,
            actual: 0
        }),
    );
    assert_eq!(
        without_allocations(|| no_alloc::decode_array::<8>("1é")),
        Err(Error::InvalidCharacter { index: 1 }),
    );
    let text = bs58::encode([0xFF; 32]).into_string();
    assert_eq!(
        without_allocations(|| no_alloc::decode_array::<32>(&text)),
        Ok([0xFF; 32])
    );
    assert_eq!(
        without_allocations(|| no_alloc::decode_array::<31>(&text)),
        Err(Error::BufferTooSmall)
    );
    assert_eq!(
        without_allocations(|| no_alloc::decode_array::<33>(&text)),
        Err(Error::InvalidLength {
            expected: 33,
            actual: 32
        }),
    );
}

// One test in a separate integration process keeps first use cold and prevents
// overlapping counting regions. Assertions/oracle/caller-buffer setup stay outside.
#[test]
fn no_allocator_calls_on_cold_success_or_error_paths() {
    cold_paths();
    differential_boundaries();
    invalid_byte_offsets();
    borrowed_and_array_contracts();
}
