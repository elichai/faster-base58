//! Binary-to-base58 conversion by multiplication in the destination radix. A binary
//! split is free: recursively convert `hi` and `lo`, then form `hi * 256^k + lo` in
//! base 58^3. The powers and their transforms live for this conversion only.
//!
//! This is the multiplication-based radix conversion used, for example, by FLINT's
//! `radix_set_mpn`; it needs no reciprocals or approximate quotient corrections.

use alloc::vec::Vec;

use super::Kernels;
use super::tables::{MAX_LOG, PRIMES, Tables, Twiddles};
use crate::BufferTooSmall;
use crate::common::ALPHABET;

const BASE: u32 = 58u32.pow(3);
const LEAF_BYTES: usize = 32;
const LEAF_DIGITS: usize = crate::encode::max_encoded_len(LEAF_BYTES).div_ceil(3);
const THRESHOLD: usize = 64;

// A transform has at most 2^23 coefficients. Each ordinary product coefficient
// sums at most 2^22 digit products; two primes therefore determine it exactly.
const MAX_COEFFICIENT: u64 = (1u64 << (MAX_LOG - 1)) * (BASE as u64 - 1).pow(2);
const _: () = assert!(MAX_COEFFICIENT < PRIMES[0].q as u64 * PRIMES[1].q as u64);
const _: () =
    assert!(MAX_COEFFICIENT + MAX_COEFFICIENT / (BASE as u64 - 1) + (BASE as u64) < u64::MAX);

/// Returns `None` before allocating when the division tree is preferable or this
/// backend's transform limit is exceeded.
/// `x` has already had its `zeros` leading zero bytes removed.
#[cfg(any(
    test,
    all(
        target_arch = "aarch64",
        target_endian = "little",
        target_feature = "neon"
    ),
    all(target_arch = "x86_64", target_feature = "sse2")
))]
pub(super) fn try_encode(
    zeros: usize,
    input: &[u8],
    output: &mut [u8],
) -> Option<Result<usize, BufferTooSmall>> {
    if !(supported() && preferred(input.len())) {
        return None;
    }
    encode_with(zeros, input, output, Kernels::best())
}

#[cfg(all(
    feature = "check",
    feature = "alloc",
    any(
        all(
            target_arch = "aarch64",
            target_endian = "little",
            target_feature = "neon"
        ),
        all(target_arch = "x86_64", target_feature = "sse2")
    )
))]
pub(super) fn try_encode_staged(
    zeros: usize,
    end: usize,
    output: &mut [u8],
) -> Option<Result<usize, BufferTooSmall>> {
    if !(supported() && preferred(end - zeros)) {
        return None;
    }
    let digits = encode_value(zeros, &output[zeros..end], output.len(), Kernels::best())?;
    Some(digits.and_then(|digits| write(zeros, &digits, output)))
}

/// Whether this CPU runs the route. x86 needs the AVX-512 transforms; AVX2-only hosts
/// keep the division tree because no such host has been measured.
#[cfg(any(
    test,
    all(
        target_arch = "aarch64",
        target_endian = "little",
        target_feature = "neon"
    ),
    all(target_arch = "x86_64", target_feature = "sse2")
))]
fn supported() -> bool {
    core::cfg_select! {
        all(target_arch = "x86_64", target_feature = "sse2") => crate::cpu::avx512f(),
        _ => true,
    }
}

#[cfg(any(
    test,
    all(
        target_arch = "aarch64",
        target_endian = "little",
        target_feature = "neon"
    ),
    all(target_arch = "x86_64", target_feature = "sse2")
))]
fn preferred(bytes: usize) -> bool {
    let Some((_, _, points)) = layout(bytes) else {
        return false;
    };
    // Full transforms waste work when heavily padded, and the balanced tree repeats
    // roughly this padding ratio at each level. The used fraction of the root transform
    // must reach 5/8, or 3/4 with `cache`, whose cached Matrix leaves speed up only the
    // division tree. Same-binary sweeps (`benches/radix_tune.rs`, generic flags) put
    // the crossover near 0.55 (M5) and 0.62 (Zen 5) without `cache`, and near 0.7 on
    // both with it. Not adopted: 7/8 forgoes 6-22% at 0.68-0.81 used; 9/16 loses 4-6%
    // at 5 KiB (M5) and 4-25% on Zen 5; 5/8 with `cache` loses up to 13% (M5, 12 KiB).
    // With `cache`, 3/4 still costs up to 3% at 13-14 KiB on M5.
    let eighths = if cfg!(feature = "cache") { 6 } else { 5 };
    bytes > MIN_BYTES && crate::encode::max_encoded_len(bytes).div_ceil(3) * 8 >= points * eighths
}

/// The smallest input is larger on x86. On Zen 5 (IFMA Karatsuba in the division
/// tree) this route took 1.23-2.54x the division tree's time at 4-16 KiB and
/// 0.95-1.06x around 32 KiB. AVX-512 CPUs without IFMA are not measured.
#[cfg(any(
    test,
    all(
        target_arch = "aarch64",
        target_endian = "little",
        target_feature = "neon"
    ),
    all(target_arch = "x86_64", target_feature = "sse2")
))]
const MIN_BYTES: usize = if cfg!(target_arch = "x86_64") {
    48 << 10
} else {
    4096
};

fn layout(bytes: usize) -> Option<(usize, usize, usize)> {
    // This conservative byte limit also makes every sizing expression below fit
    // usize on 32-bit targets. The exact transform-length check follows it.
    if bytes > (1usize << MAX_LOG) * 2 {
        return None;
    }
    let (unit, levels) = crate::dc::split_unit_leaf(bytes, LEAF_BYTES);
    let points = if levels == 0 {
        8
    } else {
        let power_bytes = unit << (levels - 1);
        let digits = crate::encode::max_encoded_len(power_bytes).div_ceil(3);
        (2 * digits).max(8).next_power_of_two()
    };
    (points <= 1 << MAX_LOG).then_some((unit, levels, points))
}

pub(super) fn encode_with(
    zeros: usize,
    input: &[u8],
    output: &mut [u8],
    kernels: Kernels,
) -> Option<Result<usize, BufferTooSmall>> {
    let digits = encode_value(zeros, input, output.len(), kernels)?;
    Some(digits.and_then(|digits| write(zeros, &digits, output)))
}

// The digit workspace owns the result before the destination is written.
fn encode_value(
    zeros: usize,
    x: &[u8],
    output_capacity: usize,
    kernels: Kernels,
) -> Option<Result<Vec<u32>, BufferTooSmall>> {
    let (unit, levels, points) = layout(x.len())?;
    let minimum = if x.is_empty() {
        0
    } else {
        crate::encode::min_digits(crate::encode::bits(x))
    };
    if zeros > output_capacity || output_capacity - zeros < minimum {
        return Some(Err(BufferTooSmall));
    }
    if levels == 0 {
        return Some(Ok(leaf(x)));
    }
    let log = points.ilog2().max(5);
    #[cfg(feature = "cache")]
    let shared = (log <= super::tables::cache::CACHE_MAX_LOG).then(|| Tables::cached(log));
    #[cfg(not(feature = "cache"))]
    let shared: Option<&Tables> = None;
    let owned;
    let tw = match shared {
        Some(tables) => &tables.tw[..2],
        None => {
            owned = Tables::twiddles::<2>(log);
            &owned
        }
    };
    let power_capacity = |bytes| crate::encode::max_encoded_len(bytes + 1).div_ceil(3);
    // Product slots need their untrimmed length, even when the power loses a digit.
    let storage_len = power_capacity(unit)
        + (1..levels)
            .map(|i| 2 * power_capacity(unit << (i - 1)))
            .sum::<usize>();
    let spectrum_len = (0..levels - 1)
        .map(|i| 2 * (2 * power_capacity(unit << i)).max(8).next_power_of_two())
        .sum::<usize>();
    // Children and power squares need at most points/2 transform points. After
    // both children finish, their spectra become the root's residue storage.
    let mut arena = vec![0; (spectrum_len + 3 * (points / 2)).max(3 * points)];
    let (spectra, work) = arena.split_at_mut(spectrum_len);
    let (residues, tmp) = work.split_at_mut(points);
    let mut arithmetic = Arithmetic {
        kernels,
        tw,
        residues,
        tmp,
        sums: Vec::with_capacity(2 * THRESHOLD),
    };
    let mut powers = Vec::with_capacity(levels);
    let mut storage = vec![0; storage_len];
    let mut spectrum_storage = spectra;
    let (seed, mut remaining) = storage.split_at_mut(power_capacity(unit));
    let mut bytes = [0; LEAF_BYTES + 1];
    bytes[0] = 1;
    let len = leaf_into(&bytes[..unit + 1], seed);
    let mut digits = &seed[..len];
    for i in 0..levels {
        // The top power serves only the root; transforming it in work avoids storing it.
        let spectrum = if i + 1 < levels && digits.len() >= THRESHOLD {
            let len = 2 * (2 * digits.len()).max(8).next_power_of_two();
            let slot;
            (slot, spectrum_storage) = spectrum_storage.split_at_mut(len);
            Some(arithmetic.prepare(digits, slot))
        } else {
            None
        };
        powers.push(Power {
            bytes: unit << i,
            digits,
            spectrum,
        });
        if i + 1 < levels {
            let p = &powers[i];
            let slot;
            (slot, remaining) = remaining.split_at_mut(2 * p.digits.len());
            let len = arithmetic.multiply_add(p.digits, p.digits, slot, 0, p.spectrum);
            digits = &slot[..len];
        } else {
            break;
        }
    }
    let out_capacity = capacity(&powers);
    let scratch: usize = (0..powers.len()).map(|n| capacity(&powers[..n])).sum();
    let mut digits = vec![0; out_capacity + scratch];
    let (out, scratch) = digits.split_at_mut(out_capacity);
    let (root, smaller) = powers.split_last().expect("levels is nonzero");
    let power = root.digits;
    let (hi, lo) = x.split_at(x.len() - root.bytes);
    let (high, scratch) = scratch.split_at_mut(capacity(smaller));
    let hi_len = convert(hi, smaller, &mut arithmetic, high, scratch);
    let lo_len = convert(lo, smaller, &mut arithmetic, out, scratch);
    drop(powers);
    let Arithmetic {
        kernels, tw, sums, ..
    } = arithmetic;
    let (residues, tmp) = arena.split_at_mut(2 * points);
    let mut arithmetic = Arithmetic {
        kernels,
        tw,
        residues,
        tmp: &mut tmp[..points],
        sums,
    };
    let len = arithmetic.multiply_add(&high[..hi_len], power, out, lo_len, None);
    digits.truncate(len);
    Some(Ok(digits))
}

struct Power<'a, 's> {
    bytes: usize,
    digits: &'a [u32],
    spectrum: Option<[&'s [i32]; 2]>,
}

struct Arithmetic<'a, 'w> {
    kernels: Kernels,
    tw: &'a [Twiddles],
    residues: &'w mut [i32],
    tmp: &'w mut [i32],
    sums: Vec<u64>,
}

impl Arithmetic<'_, '_> {
    fn prepare<'a>(&self, b: &[u32], storage: &'a mut [i32]) -> [&'a [i32]; 2] {
        let n = (2 * b.len()).max(8).next_power_of_two();
        assert_eq!(storage.len(), 2 * n);
        for (j, f) in storage.chunks_exact_mut(n).enumerate() {
            load(f, b);
            (self.kernels.forward)(f, PRIMES[j], self.tw[j].fw());
        }
        let (a, b) = storage.split_at(n);
        [a, b]
    }

    #[cfg(test)]
    fn multiply(&mut self, a: &[u32], b: &[u32], fb: Option<[&[i32]; 2]>) -> Vec<u32> {
        let mut out = vec![0; a.len() + b.len()];
        let len = self.multiply_add(a, b, &mut out, 0, fb);
        out.truncate(len);
        out
    }

    /// `a * b + out[..add_len]`, with the addend below `b`. The result is below `(a + 1) * b`
    /// and fits the same `a.len() + b.len()` digits as the product alone.
    fn multiply_add(
        &mut self,
        a: &[u32],
        b: &[u32],
        out: &mut [u32],
        add_len: usize,
        fb: Option<[&[i32]; 2]>,
    ) -> usize {
        if a.is_empty() || b.is_empty() {
            return add_len;
        }
        if b == [1] && add_len == 0 {
            out[..a.len()].copy_from_slice(a);
            return a.len();
        }
        let len = a.len() + b.len();
        let out = &mut out[..len];
        if a == [1] {
            carry(out, |i| b.get(i).map_or(0, |&d| d as u64), add_len);
        } else if a.len().min(b.len()) < THRESHOLD {
            self.sums.clear();
            self.sums.resize(len, 0);
            // Accumulate before carrying: all partial sums are nonnegative and no
            // larger than the full coefficient bound above.
            for (i, &a) in a.iter().enumerate() {
                for (s, &b) in self.sums[i..].iter_mut().zip(b) {
                    *s += a as u64 * b as u64;
                }
            }
            let sums = &self.sums;
            carry(out, |i| sums[i], add_len);
        } else {
            let n = len.max(8).next_power_of_two();
            let r = &mut self.residues[..2 * n];
            let tmp = &mut self.tmp[..n];
            let square = core::ptr::eq(a, b);
            for (j, r) in r.chunks_exact_mut(n).enumerate() {
                let p = PRIMES[j];
                let tw = &self.tw[j];
                if square && let Some(f) = fb {
                    r.copy_from_slice(&f[j][..n]);
                } else {
                    load(r, a);
                    (self.kernels.forward)(r, p, tw.fw());
                }
                if square {
                    (self.kernels.square)(r, p);
                } else if let Some(f) = fb {
                    // The bit-reversed spectrum's first n entries are the smaller
                    // cyclic transform. Here b.len() <= n, so no input folds.
                    (self.kernels.pointwise)(r, &f[j][..n], p);
                } else {
                    load(tmp, b);
                    (self.kernels.forward)(tmp, p, tw.fw());
                    (self.kernels.pointwise)(r, tmp, p);
                }
                (self.kernels.inverse)(r, p, tw.inverse(tmp));
            }
            let (r1, r2) = r.split_at_mut(n);
            garner_two(
                &mut r1[..len.next_multiple_of(4)],
                &mut r2[..len.next_multiple_of(4)],
            );
            let q1 = PRIMES[0].q as u64;
            carry(out, |i| r1[i] as u64 + q1 * r2[i] as u64, add_len);
        }
        out.iter().rposition(|&d| d != 0).map_or(0, |i| i + 1)
    }
}

/// `r1` to `[0, q1)` and `r2` to `(r2 - r1) / q1 mod q2` in `[0, q2)`, from residues in
/// `(-q, q)`. The lengths are equal and a multiple of 4.
fn garner_two(r1: &mut [i32], r2: &mut [i32]) {
    core::cfg_select! {
        all(target_arch = "aarch64", target_endian = "little", target_feature = "neon") => {
            super::neon::garner_two(r1, r2)
        }
        all(target_arch = "x86_64", target_feature = "sse2") => {
            if crate::cpu::avx512f() {
                // SAFETY: the CPU has AVX-512F.
                unsafe { super::avx512::garner_two(r1, r2) }
            } else {
                garner_two_portable(r1, r2)
            }
        }
        _ => garner_two_portable(r1, r2),
    }
}

/// [`garner_two`] for any length.
#[cfg(any(
    test,
    not(all(
        target_arch = "aarch64",
        target_endian = "little",
        target_feature = "neon"
    ))
))]
pub(super) fn garner_two_portable(r1: &mut [i32], r2: &mut [i32]) {
    use super::crt::GARNER;
    let [p1, p2, _] = PRIMES;
    assert_eq!(r1.len(), r2.len());
    for (r1, r2) in r1.iter_mut().zip(r2) {
        *r1 += (*r1 >> 31) & p1.q;
        // The difference is in (-q1-q2, q2), hence within i32. c12 is
        // balanced, so mont returns a value of magnitude below 0.75*q2.
        let v2 = p2.mont(*r2 - *r1, GARNER.c12, GARNER.c12q);
        *r2 = v2 + ((v2 >> 31) & p2.q);
    }
}

fn load(out: &mut [i32], digits: &[u32]) {
    let (head, tail) = out.split_at_mut(digits.len());
    for (to, &from) in head.iter_mut().zip(digits) {
        *to = from as i32; // BASE is below both primes.
    }
    tail.fill(0);
}

/// `out` = the coefficients `coef(i)` plus the first `add_len` digits of `out`, in base
/// `BASE`. The value must fit `out`.
///
/// One carry chain waits on a division per digit, so four chains run interleaved over
/// quarters of `out`, each starting without a carry. Each quarter's carry is then added
/// at the start of the next, which rarely changes more than three digits. Eight chains
/// measure no faster than four (M5).
fn carry(out: &mut [u32], coef: impl Fn(usize) -> u64, add_len: usize) {
    const K: usize = 4;
    let step = |digit: &mut u32, i: usize, carry: u64| {
        // Adding a digit raises the inductive carry bound to
        // MAX_COEFFICIENT / (BASE - 1) + 1; the complete sum still fits u64.
        let add = if i < add_len { *digit as u64 } else { 0 };
        let value = coef(i) + carry + add;
        *digit = (value % BASE as u64) as u32;
        value / BASE as u64
    };
    let s = out.len() / K;
    let mut carries = [0; K];
    for j in 0..s {
        for (k, c) in carries.iter_mut().enumerate() {
            *c = step(&mut out[k * s + j], k * s + j, *c);
        }
    }
    for (i, digit) in (K * s..).zip(&mut out[K * s..]) {
        carries[K - 1] = step(digit, i, carries[K - 1]);
    }
    debug_assert_eq!(carries[K - 1], 0);
    for (k, &c) in carries[..K - 1].iter().enumerate() {
        let mut carry = c;
        for digit in &mut out[(k + 1) * s..] {
            if carry == 0 {
                break;
            }
            let value = *digit as u64 + carry;
            *digit = (value % BASE as u64) as u32;
            carry = value / BASE as u64;
        }
        debug_assert_eq!(carry, 0);
    }
}

/// Horner in groups of five bytes. A digit times 2^40 plus its carry is below 2^58.
fn leaf(bytes: &[u8]) -> Vec<u32> {
    let (_, bytes) = crate::encode::split_zeros(bytes);
    if bytes.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0; crate::encode::max_encoded_len(bytes.len()).div_ceil(3)];
    let len = leaf_into(bytes, &mut out);
    out.truncate(len);
    out
}

fn leaf_into(bytes: &[u8], out: &mut [u32]) -> usize {
    let (_, bytes) = crate::encode::split_zeros(bytes);
    if bytes.is_empty() {
        return 0;
    }
    let mut len = 0;
    for chunk in bytes.chunks(5) {
        let bits = 8 * chunk.len();
        let mut carry = chunk.iter().fold(0u64, |c, &b| (c << 8) | b as u64);
        for d in &mut out[..len] {
            let value = ((*d as u64) << bits) + carry;
            *d = (value % BASE as u64) as u32;
            carry = value / BASE as u64;
        }
        while carry != 0 {
            out[len] = (carry % BASE as u64) as u32;
            len += 1;
            carry /= BASE as u64;
        }
    }
    len
}

fn capacity(powers: &[Power<'_, '_>]) -> usize {
    powers.last().map_or(LEAF_DIGITS, |p| 2 * p.digits.len())
}

fn convert(
    x: &[u8],
    powers: &[Power<'_, '_>],
    arithmetic: &mut Arithmetic<'_, '_>,
    out: &mut [u32],
    scratch: &mut [u32],
) -> usize {
    let Some((p, smaller)) = powers.split_last() else {
        return leaf_into(x, out);
    };
    if x.len() <= p.bytes {
        return convert(x, smaller, arithmetic, out, scratch);
    }
    let (hi, lo) = x.split_at(x.len() - p.bytes);
    // Each level holds its high child while the low child reuses the remaining scratch.
    let (high, scratch) = scratch.split_at_mut(capacity(smaller));
    let hi_len = convert(hi, smaller, arithmetic, high, scratch);
    let lo_len = convert(lo, smaller, arithmetic, out, scratch);
    arithmetic.multiply_add(&high[..hi_len], p.digits, out, lo_len, p.spectrum)
}

fn write(zeros: usize, digits: &[u32], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    let head = digits.last().copied().unwrap_or(0);
    let head_len = if head == 0 {
        0
    } else if head < 58 {
        1
    } else if head < 58 * 58 {
        2
    } else {
        3
    };
    let len = zeros + digits.len().saturating_sub(1) * 3 + head_len;
    if output.len() < len {
        return Err(BufferTooSmall);
    }
    output[..zeros].fill(b'1');
    let mut at = zeros;
    for (i, &digit) in digits.iter().rev().enumerate() {
        let chars = [
            ALPHABET[(digit / (58 * 58)) as usize],
            ALPHABET[(digit / 58 % 58) as usize],
            ALPHABET[(digit % 58) as usize],
        ];
        let chars = &chars[if i == 0 { 3 - head_len } else { 0 }..];
        output[at..at + chars.len()].copy_from_slice(chars);
        at += chars.len();
    }
    Ok(len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use malachite_base::num::conversion::traits::{Digits, PowerOf2Digits};
    use malachite_nz::natural::Natural;
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    fn multiply_add(
        arithmetic: &mut Arithmetic<'_, '_>,
        a: &[u32],
        b: &[u32],
        add: &[u32],
        fb: Option<[&[i32]; 2]>,
    ) -> Vec<u32> {
        let mut out = vec![u32::MAX; a.len() + b.len()];
        out[..add.len()].copy_from_slice(add);
        let len = arithmetic.multiply_add(a, b, &mut out, add.len(), fb);
        out.truncate(len);
        out
    }

    fn oracle(input: &[u8]) -> Vec<u8> {
        let (zeros, x) = crate::encode::split_zeros(input);
        let n = Natural::from_power_of_2_digits_desc(8, x.iter().copied()).unwrap();
        let digits: Vec<u8> = n.to_digits_desc(&58);
        core::iter::repeat_n(b'1', zeros)
            .chain(digits.iter().map(|&d| ALPHABET[d as usize]))
            .collect()
    }

    fn check(input: &[u8], want: &[u8], kernels: Kernels, buffers: bool) {
        let (zeros, x) = crate::encode::split_zeros(input);
        let mut out = vec![0xa5; want.len() + 32];
        let n = encode_with(zeros, x, &mut out[8..], kernels)
            .unwrap()
            .unwrap();
        assert_eq!(n, want.len());
        assert_eq!(&out[8..8 + n], want);
        assert!(out[..8].iter().chain(&out[8 + n..]).all(|&c| c == 0xa5));
        if buffers {
            let mut exact = vec![0xa5; want.len()];
            assert_eq!(
                encode_with(zeros, x, &mut exact, kernels),
                Some(Ok(want.len()))
            );
            assert_eq!(exact, want);
            if !want.is_empty() {
                assert_eq!(
                    encode_with(zeros, x, &mut exact[..want.len() - 1], kernels),
                    Some(Err(BufferTooSmall))
                );
            }
        }
    }

    #[test]
    fn sizes_and_coefficient_bound() {
        assert_eq!(try_encode(0, &[], &mut []), None);
        assert_eq!(encode_with(0, &[], &mut [], Kernels::best()), Some(Ok(0)));
        assert!(layout(usize::MAX).is_none());
        assert!(layout((1 << MAX_LOG) * 2 + 1).is_none());
        for bytes in [0, 1, 31, 32, 33, 4096, 1 << 20, 1 << 24] {
            if let Some((unit, levels, points)) = layout(bytes) {
                assert!(unit <= LEAF_BYTES);
                assert!(points.is_power_of_two() && points <= 1 << MAX_LOG);
                assert!(levels == 0 || unit << levels >= bytes);
            }
        }
        assert!(MAX_COEFFICIENT < PRIMES[0].q as u64 * PRIMES[1].q as u64);
    }

    #[test]
    fn selection_preserves_fallback_and_short_buffers() {
        // Below 5/8 of the root transform used, and below 4097 bytes.
        for bytes in [4096, 4353, 8705, 17_409, 69_633, 89_976, 557_057] {
            assert_eq!(try_encode(0, &vec![255; bytes], &mut []), None);
        }
        // From 5/8 to below 3/4 used: only without `cache`; then also with it.
        let default_only =
            [6144, 12_288, 89_977, 98_304, 107_972, 786_432].map(|b| (b, !cfg!(feature = "cache")));
        let always = [
            4097, 8192, 32_768, 65_536, 107_973, 262_143, 1_048_575, 1_048_576,
        ]
        .map(|b| (b, true));
        for (bytes, used) in default_only.into_iter().chain(always) {
            let selected = supported() && bytes > MIN_BYTES && used;
            assert_eq!(
                try_encode(0, &vec![255; bytes], &mut []),
                selected.then_some(Err(BufferTooSmall)),
                "{bytes} bytes"
            );
        }
    }

    #[test]
    fn products_match_natural() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(5803);
        for kernels in [Kernels::best(), Kernels::portable()] {
            let tables = Tables::new(11);
            let mut work = vec![0; 3 << 11];
            let (residues, tmp) = work.split_at_mut(2 << 11);
            let mut arithmetic = Arithmetic {
                kernels,
                tw: &tables.tw,
                residues,
                tmp,
                sums: Vec::new(),
            };
            for (n, m) in [1, 63, 64, 65, 127, 128, 129, 255, 256, 257, 511]
                .map(|n| (n, n + 1))
                .into_iter()
                .chain([(64, 129), (96, 257)])
            {
                for maximum in [false, true] {
                    let a: Vec<u32> = (0..n)
                        .map(|_| {
                            if maximum {
                                BASE - 1
                            } else {
                                rng.next_u32() % BASE
                            }
                        })
                        .collect();
                    let b: Vec<u32> = (0..m)
                        .map(|_| {
                            if maximum {
                                BASE - 1
                            } else {
                                (rng.next_u32() % BASE) | 1
                            }
                        })
                        .collect();
                    let value =
                        |d: &[u32]| Natural::from_digits_asc(&BASE, d.iter().copied()).unwrap();
                    let want: Vec<u32> = (&value(&a) * &value(&b)).to_digits_asc(&BASE);
                    assert_eq!(arithmetic.multiply(&a, &b, None), want, "length {n}");
                    let mut spectrum = vec![0; 2 * (2 * b.len()).max(8).next_power_of_two()];
                    let fb = arithmetic.prepare(&b, &mut spectrum);
                    assert_eq!(
                        arithmetic.multiply(&a, &b, Some(fb)),
                        want,
                        "prepared length {n}"
                    );
                    let square: Vec<u32> = (&value(&b) * &value(&b)).to_digits_asc(&BASE);
                    assert_eq!(
                        arithmetic.multiply(&b, &b, Some(fb)),
                        square,
                        "square length {n}"
                    );
                    let add: Vec<u32> = (value(&b) - Natural::from(1u8)).to_digits_asc(&BASE);
                    let sum: Vec<u32> =
                        (&value(&a) * &value(&b) + value(&add)).to_digits_asc(&BASE);
                    assert_eq!(multiply_add(&mut arithmetic, &a, &b, &add, Some(fb)), sum);
                    let one_sum: Vec<u32> = (value(&b) + value(&add)).to_digits_asc(&BASE);
                    assert_eq!(
                        multiply_add(&mut arithmetic, &[1], &b, &add, Some(fb)),
                        one_sum
                    );
                    assert_eq!(multiply_add(&mut arithmetic, &[], &b, &add, Some(fb)), add);
                }
            }
        }
    }

    #[test]
    fn small_inputs_match_bs58() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(5804);
        for len in (0..100).chain([
            127, 128, 129, 255, 256, 257, 511, 512, 513, 1023, 1024, 2049, 4096, 8193,
        ]) {
            let mut random = vec![0; len];
            rng.fill_bytes(&mut random);
            let mut sparse = vec![0; len];
            if len != 0 {
                sparse[0] = 1;
                sparse[len - 1] = 1;
            }
            for data in [random, sparse, vec![255; len], vec![0; len]] {
                let want = bs58::encode(&data).into_vec();
                check(&data, &want, Kernels::best(), true);
            }
        }
    }

    #[test]
    fn large_inputs_match_malachite() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(5805);
        for len in [
            16_383, 16_384, 16_385, 32_767, 32_768, 32_769, 65_535, 65_536, 65_537, 262_144,
            1_048_576,
        ] {
            let mut data = vec![0; len + 7];
            rng.fill_bytes(&mut data[7..]);
            data[7] |= 1;
            let want = oracle(&data);
            check(&data, &want, Kernels::best(), len <= 65_537);
            if len <= 65_537 {
                check(&data, &want, Kernels::portable(), true);
            }
        }
        for pattern in 0..3 {
            let mut data = vec![255; 1_048_576];
            if pattern == 1 {
                data.fill(0);
                data[0] = 1;
                data[1_048_575] = 1;
            } else if pattern == 2 {
                data[100_000..400_000].fill(0);
                data[600_000..900_000].fill(1);
            }
            check(&data, &oracle(&data), Kernels::best(), false);
        }
    }
}
