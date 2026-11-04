#![cfg(feature = "alloc")]

use faster_base58::{BufferTooSmall, Decoder, Encoder, Error};

const ENCODER: Encoder = Encoder::bitcoin();
const DECODER: Decoder = Decoder::bitcoin();
const INPUT: faster_base58::EncodeInput<&[u8]> = ENCODER.encode(&[0, 1, 2, 3]);

#[test]
fn plain_configuration_is_const_reusable_and_matches_direct_api() {
    let want = faster_base58::Encoder::bitcoin()
        .encode([0, 1, 2, 3])
        .into_string();
    assert_eq!(INPUT.into_string(), want);
    for data in [vec![], vec![0; 32], vec![0xff; 64], vec![42; 512]] {
        let want = faster_base58::Encoder::bitcoin()
            .encode(&data)
            .into_string();
        assert_eq!(ENCODER.encode(&data).into_string(), want);
        assert_eq!(ENCODER.encode(data.clone()).into_vec(), want.as_bytes());
        assert_eq!(DECODER.decode(want.clone()).into_vec(), Ok(data.clone()));
        let mut encoded = vec![0xa5; ENCODER.max_encoded_len(data.len()) + 1];
        let text = ENCODER.encode(&data).onto(&mut encoded).unwrap();
        assert_eq!(text, want);
        let mut decoded = vec![0xa5; data.len() + 1];
        assert_eq!(DECODER.decode(text).onto(&mut decoded).unwrap(), data);
        assert_eq!(encoded[want.len()], 0xa5);
        assert_eq!(decoded[data.len()], 0xa5);
    }
    assert_eq!(DECODER.decode("112").into_array::<3>(), Ok([0, 0, 1]));
    assert_eq!(DECODER.decode("").into_array::<0>(), Ok([]));
    assert_eq!(
        DECODER.decode("112").into_array::<4>(),
        Err(Error::InvalidLength {
            expected: 4,
            actual: 3
        })
    );
    assert_eq!(
        DECODER.decode("112").into_array::<2>(),
        Err(Error::BufferTooSmall)
    );
    assert_eq!(
        DECODER.decode("1é").into_vec(),
        Err(Error::InvalidCharacter { index: 1 })
    );
    assert_eq!(ENCODER.encode([42]).onto(&mut []), Err(BufferTooSmall));
    assert_eq!(ENCODER.max_encoded_len(usize::MAX), usize::MAX);
}
