//! A separate test binary keeps both matrix caches cold before rejection checks.
#![cfg(feature = "__internals")]

use base58_bench::{self as b58, BufferTooSmall, Error, internals};

type Encode = fn(&[u8], &mut [u8]) -> Result<usize, BufferTooSmall>;
type Decode = fn(&str, &mut [u8]) -> Result<usize, Error>;

const ENCODERS: [(&str, Encode); 5] = [
    ("auto", b58::encode),
    ("matrix", b58::internals::encode_matrix),
    ("portable", b58::internals::encode_matrix_portable),
    ("forced", |input, output| internals::encode_matrix_with(input, output, 0)),
    ("whole", internals::encode_matrix_whole),
];
const DECODERS: [(&str, Decode); 4] = [
    ("auto", b58::decode),
    ("matrix", b58::internals::decode_matrix),
    ("portable", b58::internals::decode_matrix_portable),
    ("forced", internals::decode_matrix_forced),
];

#[test]
fn impossible_encode_buffers_build_no_table() {
    let retained = || internals::encode_matrix_cache_memory().1.iter().any(|row| row.1);
    assert!(!retained());
    let mut power = vec![0; 512];
    power[0] = 1;
    // 2^4088 needs 698 base58 digits. A bound based only on byte count misses this.
    let cases = [
        (vec![0; 512], 511),
        (vec![0xFF; 512], 0),
        ([vec![0; 100], vec![0xFF; 512]].concat(), 99),
        (power, 697),
    ];
    for (input, capacity) in &cases {
        for (name, encode) in ENCODERS {
            assert_eq!(encode(input, &mut vec![0xA5; *capacity]), Err(BufferTooSmall), "{name}");
            assert!(!retained(), "{name}: rejected {} bytes retained a table", input.len());
        }
    }
    for (input, _) in cases {
        let want = bs58::encode(&input).into_vec();
        for (name, encode) in ENCODERS {
            for capacity in [want.len(), want.len() + 17] {
                let mut output = vec![0xA5; capacity];
                assert_eq!(encode(&input, &mut output), Ok(want.len()), "{name}");
                assert_eq!(&output[..want.len()], want, "{name}");
                assert!(output[want.len()..].iter().all(|&b| b == 0xA5), "{name}");
            }
        }
    }
    assert_eq!(retained(), cfg!(feature = "cache"), "matrix table retention follows the cache feature");
}

#[test]
fn impossible_decode_buffers_build_no_table() {
    let retained = || internals::decode_matrix_cache_memory().1.iter().any(|row| row.1);
    assert!(!retained());
    let cases = [
        ("1".repeat(4096), 4095),
        ("z".repeat(4096), 32),
        (format!("{}{}", "1".repeat(100), "z".repeat(704)), 99),
        // 58^703 needs 515 bytes; this exercises a nonempty short output.
        (format!("2{}", "1".repeat(703)), 514),
    ];
    for (input, capacity) in &cases {
        for (name, decode) in DECODERS {
            assert_eq!(decode(input, &mut vec![0xA5; *capacity]), Err(Error::BufferTooSmall), "{name}");
            assert!(!retained(), "{name}: rejected {} characters retained a table", input.len());
        }
    }
    assert_eq!(b58::decode_array::<32>(&"z".repeat(4096)), Err(Error::BufferTooSmall));
    assert!(!retained());
    let invalid = format!("11{}é0", "z".repeat(704));
    for (name, decode) in DECODERS {
        assert_eq!(
            decode(&invalid, &mut vec![0xA5; invalid.len()]),
            Err(Error::InvalidCharacter { index: 706 }),
            "{name}",
        );
        assert!(!retained(), "{name}: invalid input retained a table");
    }
    for (input, _) in cases {
        let want = bs58::decode(&input).into_vec().unwrap();
        for (name, decode) in DECODERS {
            for capacity in [want.len(), want.len() + 17] {
                let mut output = vec![0xA5; capacity];
                assert_eq!(decode(&input, &mut output), Ok(want.len()), "{name}");
                assert_eq!(&output[..want.len()], want, "{name}");
                assert!(output[want.len()..].iter().all(|&b| b == 0xA5), "{name}");
            }
        }
    }
    assert_eq!(retained(), cfg!(feature = "cache"), "matrix table retention follows the cache feature");
}
