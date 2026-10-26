//! Full-width carry invariants for the production matrix decoder.

use super::{carry32, carry64};

#[kani::proof]
#[kani::unwind(4)]
fn carry32_preserves_value() {
    let a: u64 = kani::any();
    let b: u64 = kani::any();
    kani::assume(a <= u64::MAX - u32::MAX as u64);
    kani::assume(b <= u64::MAX - u32::MAX as u64);
    let mut cols = [a, b, 0];
    carry32(&mut cols);
    assert!(cols[0] <= u32::MAX as u64);
    assert!(cols[1] <= u32::MAX as u64);
    assert!(cols[2] <= u32::MAX as u64);
    let before = a as u128 + ((b as u128) << 32);
    let after = cols[0] as u128 + ((cols[1] as u128) << 32) + ((cols[2] as u128) << 64);
    assert_eq!(before, after);
}

#[kani::proof]
#[kani::unwind(6)]
fn carry64_preserves_value_and_length() {
    let a: u64 = kani::any();
    let b: u64 = kani::any();
    let mut cols = [a, b, 0, 0];
    let len = carry64(&mut cols);
    let value = a as u128 + ((b as u128) << 32);
    assert_eq!(cols[0], value as u64);
    assert_eq!(cols[1], (value >> 64) as u64);
    assert_eq!(
        len,
        if cols[1] != 0 {
            2
        } else {
            usize::from(cols[0] != 0)
        }
    );
}
