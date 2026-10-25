//! Fresh-process table initialization versus the next call; every result is checked.

use std::{hint::black_box, process::Command, time::Instant};

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.is_empty() {
        println!("operation,bytes,first_ns,second_ns");
        let exe = std::env::current_exe().unwrap();
        for operation in ["encode", "decode"] {
            for size in [0, 1, 31, 32, 64, 512, 4096, 8192] {
                for _ in 0..3 {
                    assert!(Command::new(&exe).args([operation, &size.to_string()]).status().unwrap().success());
                }
            }
        }
        return;
    }
    assert_eq!(args.len(), 2, "usage: cold [encode|decode BYTES]");
    let size: usize = args[1].parse().unwrap();
    let data: Vec<u8> = (0..size).map(|i| (i.wrapping_mul(73) + 1) as u8).collect();
    let encoded = bs58::encode(&data).into_string();
    let mut times = [0; 2];
    match args[0].as_str() {
        "encode" => {
            let mut out = vec![0; base58_bench::max_encoded_len(size)];
            for elapsed in &mut times {
                let start = Instant::now();
                let len = base58_bench::encode(black_box(&data), black_box(&mut out)).unwrap();
                *elapsed = start.elapsed().as_nanos();
                assert_eq!(&out[..len], encoded.as_bytes());
            }
        }
        "decode" => {
            let mut out = vec![0; size];
            for elapsed in &mut times {
                let start = Instant::now();
                let len = base58_bench::decode(black_box(&encoded), black_box(&mut out)).unwrap();
                *elapsed = start.elapsed().as_nanos();
                assert_eq!(len, size);
                assert_eq!(out, data);
            }
        }
        _ => panic!("usage: cold [encode|decode BYTES]"),
    }
    println!("{},{size},{},{}", args[0], times[0], times[1]);
}
