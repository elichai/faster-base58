"""Build an exhaustive-half and boundary check from the extracted formatter sources."""
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parent
source = (root / 'base58_pr1_ten_codegen_top.rs').read_text()
source += r'''
fn main() {
    const HALF: u64 = 58u64.pow(5);
    let check = |value| assert_eq!(format_candidate(value), format_pr(value), "value {value}");
    for half in 0..HALF {
        check(half);
        check(half * HALF);
    }
    check(BASE - 1);
    for position in 0..5 {
        for pair in 1..58u64.pow(2) {
            let boundary = pair * 58u64.pow(2 * position);
            for value in [boundary - 1, boundary, boundary + 1] {
                if value < BASE { check(value); }
            }
        }
    }
    let mut state = 0x123456789abcdefu64;
    for _ in 0..1_000_000 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        check(state % BASE);
    }
    println!("Both {}-value five-digit halves, all pair boundaries, and 1,000,000 seeded whole-limb values match PR1", HALF);
}
'''
target = root / 'base58_pr1_ten_verify.rs'
assert not target.exists()
target.write_text(source)
subprocess.run(['rustc', '--edition=2024', '-O', '-C', 'overflow-checks=yes', '-A', 'unused_imports',
                '-A', 'dead_code', str(target), '-o', str(root / 'base58_pr1_ten_verify')], check=True)
