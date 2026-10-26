//! The first `decode_matrix` calls in a process, from several threads at once.
//!
//! In its own file, so it runs in a fresh process where no table is built yet. Through the
//! public API it can check only the bytes, not that each table is built once.
#![cfg(feature = "__internals")]

use std::sync::Barrier;
use std::thread;

use faster_base58::Error;
use faster_base58::internals::{decode_batched, decode_matrix};
use rand_xoshiro::Xoshiro256PlusPlus;
use rand_xoshiro::rand_core::{Rng, SeedableRng};

const ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

fn decode(f: fn(&str, &mut [u8]) -> Result<usize, Error>, s: &str) -> Vec<u8> {
    let mut out = vec![0; s.len()];
    let n = f(s, &mut out).expect("valid");
    out.truncate(n);
    out
}

#[test]
fn cold_start_from_threads() {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(1);
    // One input per size class (up to 704, 1408, 2816 and 5632 characters), 'z' first so
    // that no leading '1' moves it to a smaller class.
    let inputs: Vec<String> = [600, 1000, 2000, 5632]
        .map(|n| {
            let random = (1..n).map(|_| ALPHABET[(rng.next_u32() % 58) as usize] as char);
            std::iter::once('z').chain(random).collect()
        })
        .into();
    let want: Vec<Vec<u8>> = inputs.iter().map(|s| decode(decode_batched, s)).collect();
    let threads = 8;
    let barrier = Barrier::new(threads);
    thread::scope(|scope| {
        for t in 0..threads {
            let (inputs, want, barrier) = (&inputs, &want, &barrier);
            scope.spawn(move || {
                barrier.wait();
                // Each thread starts in another class, so classes are built concurrently.
                for i in (0..inputs.len()).map(|i| (i + t) % inputs.len()) {
                    assert_eq!(
                        decode(decode_matrix, &inputs[i]),
                        want[i],
                        "thread {t}, input {i}"
                    );
                }
            });
        }
    });
}
