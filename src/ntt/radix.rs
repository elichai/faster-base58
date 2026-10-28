//! Binary-to-base58 conversion by multiplication in the destination radix. A binary
//! split is free: recursively convert `hi` and `lo`, then form `hi * 256^k + lo` in
//! base 58^3. The powers and their transforms live for this conversion only.
//!
//! This is the multiplication-based radix conversion used, for example, by FLINT's
//! `radix_set_mpn`; it needs no reciprocals or approximate quotient corrections.

use alloc::vec::Vec;

use super::tables::{MAX_LOG, PRIMES, Tables};
use super::{Kernels, Ntt};
use crate::BufferTooSmall;
use crate::common::ALPHABET;
use crate::dc::Multiplier;

const BASE: u32 = 58u32.pow(3);
const LEAF_BYTES: usize = 32;
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
pub(super) fn try_encode(
    zeros: usize,
    x: &[u8],
    output: &mut [u8],
) -> Option<Result<usize, BufferTooSmall>> {
    let (_, _, points) = layout(x.len())?;
    // Full transforms waste work when heavily padded. The balanced tree repeats
    // roughly this padding ratio at each level. Cached division also wins just
    // below binary power-of-two boundaries, so keep it until two input bytes
    // fill each transform point. See the large-value measurements.
    let preferred = if cfg!(feature = "cache") {
        x.len() >= 262_144 && x.len() >= 2 * points
    } else {
        x.len() >= 8192 && crate::max_encoded_len(x.len()).div_ceil(3) * 8 >= points * 7
    };
    if !preferred {
        return None;
    }
    encode_with(zeros, x, output, Kernels::best())
}

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
        let digits = crate::max_encoded_len(power_bytes).div_ceil(3);
        (2 * digits).max(8).next_power_of_two()
    };
    (points <= 1 << MAX_LOG).then_some((unit, levels, points))
}

fn encode_with(
    zeros: usize,
    x: &[u8],
    output: &mut [u8],
    kernels: Kernels,
) -> Option<Result<usize, BufferTooSmall>> {
    let (unit, levels, points) = layout(x.len())?;
    let minimum = if x.is_empty() {
        0
    } else {
        crate::encode::min_digits(crate::encode::bits(x))
    };
    if zeros > output.len() || output.len() - zeros < minimum {
        return Some(Err(BufferTooSmall));
    }
    if levels == 0 {
        return Some(write(zeros, &leaf(x), output));
    }
    let mut ntt = Ntt::new(kernels);
    ntt.prepare(points / 2);
    ntt.ensure_tables();
    let mut arithmetic = Arithmetic {
        kernels,
        tables: ntt.initialized_tables(),
        work: vec![0; 3 * points],
        sums: Vec::new(),
    };
    let mut powers = Vec::with_capacity(levels);
    let mut bytes = vec![0; unit + 1];
    bytes[0] = 1;
    let mut digits = leaf(&bytes);
    for i in 0..levels {
        let spectrum = (digits.len() >= THRESHOLD).then(|| arithmetic.prepare(&digits));
        powers.push(Power {
            bytes: unit << i,
            digits,
            spectrum,
        });
        if i + 1 < levels {
            let p = &powers[i];
            digits = arithmetic.multiply(&p.digits, &p.digits, p.spectrum.as_ref());
        } else {
            break;
        }
    }
    let digits = convert(x, &powers, &mut arithmetic);
    Some(write(zeros, &digits, output))
}

struct Power {
    bytes: usize,
    digits: Vec<u32>,
    spectrum: Option<[Vec<i32>; 2]>,
}

struct Arithmetic<'a> {
    kernels: Kernels,
    tables: &'a Tables,
    work: Vec<i32>,
    sums: Vec<u64>,
}

impl Arithmetic<'_> {
    fn prepare(&self, b: &[u32]) -> [Vec<i32>; 2] {
        let n = (2 * b.len()).max(8).next_power_of_two();
        core::array::from_fn(|j| {
            let mut f = vec![0; n];
            load(&mut f, b);
            (self.kernels.forward)(&mut f, PRIMES[j], &self.tables.tw[j]);
            f
        })
    }

    fn multiply(&mut self, a: &[u32], b: &[u32], fb: Option<&[Vec<i32>; 2]>) -> Vec<u32> {
        self.multiply_add(a, b, &[], fb)
    }

    /// `a * b + add`, with `add < b`. Thus the result is below `(a + 1) * b`
    /// and fits the same `a.len() + b.len()` digits as the product alone.
    fn multiply_add(
        &mut self,
        a: &[u32],
        b: &[u32],
        add: &[u32],
        fb: Option<&[Vec<i32>; 2]>,
    ) -> Vec<u32> {
        if a.is_empty() || b.is_empty() {
            return add.to_vec();
        }
        if b == [1] && add.is_empty() {
            return a.to_vec();
        }
        let len = a.len() + b.len();
        let mut out = vec![0; len];
        if a == [1] {
            let coefficients = b.iter().map(|&d| d as u64).chain(core::iter::repeat(0));
            carry(&mut out, coefficients, add);
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
            carry(&mut out, self.sums.iter().copied(), add);
        } else {
            let n = len.max(8).next_power_of_two();
            let (r, tmp) = self.work.split_at_mut(2 * n);
            let tmp = &mut tmp[..n];
            let square = core::ptr::eq(a, b);
            for (j, r) in r.chunks_exact_mut(n).enumerate() {
                let p = PRIMES[j];
                let tw = &self.tables.tw[j];
                if square && let Some(f) = fb {
                    r.copy_from_slice(&f[j][..n]);
                } else {
                    load(r, a);
                    (self.kernels.forward)(r, p, tw);
                }
                if square {
                    (self.kernels.square)(r, p);
                } else if let Some(f) = fb {
                    // The bit-reversed spectrum's first n entries are the smaller
                    // cyclic transform. Here b.len() <= n, so no input folds.
                    (self.kernels.pointwise)(r, &f[j][..n], p);
                } else {
                    load(tmp, b);
                    (self.kernels.forward)(tmp, p, tw);
                    (self.kernels.pointwise)(r, tmp, p);
                }
                (self.kernels.inverse)(r, p, tw);
            }
            let (r1, r2) = r.split_at_mut(n);
            garner_two(
                &mut r1[..len.next_multiple_of(4)],
                &mut r2[..len.next_multiple_of(4)],
            );
            let coefficients = r1
                .iter()
                .zip(r2.iter())
                .take(len)
                .map(|(&r1, &v2)| r1 as u64 + PRIMES[0].q as u64 * v2 as u64);
            carry(&mut out, coefficients, add);
        }
        trim(&mut out);
        out
    }
}

fn garner_two(r1: &mut [i32], r2: &mut [i32]) {
    #[cfg(all(
        target_arch = "aarch64",
        target_endian = "little",
        target_feature = "neon"
    ))]
    super::neon::garner_two(r1, r2);
    #[cfg(not(all(
        target_arch = "aarch64",
        target_endian = "little",
        target_feature = "neon"
    )))]
    {
        use super::crt::GARNER;
        let [p1, p2, _] = PRIMES;
        for (r1, r2) in r1.iter_mut().zip(r2) {
            *r1 += (*r1 >> 31) & p1.q;
            // The difference is in (-q1-q2, q2), hence within i32. c12 is
            // balanced, so mont returns a value of magnitude below 0.75*q2.
            let v2 = p2.mont(*r2 - *r1, GARNER.c12, GARNER.c12q);
            *r2 = v2 + ((v2 >> 31) & p2.q);
        }
    }
}

fn load(out: &mut [i32], digits: &[u32]) {
    let (head, tail) = out.split_at_mut(digits.len());
    for (to, &from) in head.iter_mut().zip(digits) {
        *to = from as i32; // BASE is below both primes.
    }
    tail.fill(0);
}

fn carry(out: &mut [u32], coefficients: impl Iterator<Item = u64>, add: &[u32]) {
    let mut carry = 0;
    for (i, (digit, coefficient)) in out.iter_mut().zip(coefficients).enumerate() {
        // Adding a digit raises the inductive carry bound to
        // MAX_COEFFICIENT / (BASE - 1) + 1; the complete sum still fits u64.
        let value = coefficient + carry + add.get(i).copied().unwrap_or(0) as u64;
        *digit = (value % BASE as u64) as u32;
        carry = value / BASE as u64;
    }
    debug_assert_eq!(carry, 0);
}

fn trim(digits: &mut Vec<u32>) {
    while digits.last() == Some(&0) {
        digits.pop();
    }
}

/// Horner in groups of five bytes. A digit times 2^40 plus its carry is below 2^58.
fn leaf(bytes: &[u8]) -> Vec<u32> {
    let (_, bytes) = crate::encode::split_zeros(bytes);
    if bytes.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(crate::max_encoded_len(bytes.len()).div_ceil(3));
    for chunk in bytes.chunks(5) {
        let bits = 8 * chunk.len();
        let mut carry = chunk.iter().fold(0u64, |c, &b| (c << 8) | b as u64);
        for d in &mut out {
            let value = ((*d as u64) << bits) + carry;
            *d = (value % BASE as u64) as u32;
            carry = value / BASE as u64;
        }
        while carry != 0 {
            out.push((carry % BASE as u64) as u32);
            carry /= BASE as u64;
        }
    }
    out
}

fn convert(x: &[u8], powers: &[Power], arithmetic: &mut Arithmetic<'_>) -> Vec<u32> {
    let Some((p, smaller)) = powers.split_last() else {
        return leaf(x);
    };
    if x.len() <= p.bytes {
        return convert(x, smaller, arithmetic);
    }
    let (hi, lo) = x.split_at(x.len() - p.bytes);
    let hi = convert(hi, smaller, arithmetic);
    let lo = convert(lo, smaller, arithmetic);
    arithmetic.multiply_add(&hi, &p.digits, &lo, p.spectrum.as_ref())
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
        for bytes in [4097, 6144, 12_288, 49_152, 98_304, 786_432] {
            assert_eq!(try_encode(0, &vec![255; bytes], &mut []), None);
        }
        let minimum = if cfg!(feature = "cache") {
            262_144
        } else {
            8192
        };
        for bytes in [minimum, 1_048_576] {
            assert_eq!(
                try_encode(0, &vec![255; bytes], &mut []),
                Some(Err(BufferTooSmall))
            );
        }
        if cfg!(feature = "cache") {
            assert_eq!(try_encode(0, &vec![255; 131_072], &mut []), None);
            assert_eq!(try_encode(0, &vec![255; 1_007_758], &mut []), None);
            assert_eq!(try_encode(0, &vec![255; 1_048_575], &mut []), None);
        }
    }

    #[test]
    fn products_match_natural() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(5803);
        for kernels in [Kernels::best(), Kernels::portable()] {
            let tables = Tables::new(11);
            let mut arithmetic = Arithmetic {
                kernels,
                tables: &tables,
                work: vec![0; 3 << 11],
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
                    let fb = arithmetic.prepare(&b);
                    assert_eq!(
                        arithmetic.multiply(&a, &b, Some(&fb)),
                        want,
                        "prepared length {n}"
                    );
                    let square: Vec<u32> = (&value(&b) * &value(&b)).to_digits_asc(&BASE);
                    assert_eq!(
                        arithmetic.multiply(&b, &b, Some(&fb)),
                        square,
                        "square length {n}"
                    );
                    let add: Vec<u32> = (value(&b) - Natural::from(1u8)).to_digits_asc(&BASE);
                    let sum: Vec<u32> =
                        (&value(&a) * &value(&b) + value(&add)).to_digits_asc(&BASE);
                    assert_eq!(arithmetic.multiply_add(&a, &b, &add, Some(&fb)), sum);
                    let one_sum: Vec<u32> = (value(&b) + value(&add)).to_digits_asc(&BASE);
                    assert_eq!(arithmetic.multiply_add(&[1], &b, &add, Some(&fb)), one_sum);
                    assert_eq!(arithmetic.multiply_add(&[], &b, &add, Some(&fb)), add);
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
        for len in [32_769, 262_144, 1_048_576] {
            let mut data = vec![0; len];
            rng.fill_bytes(&mut data);
            data[..7].fill(0);
            let want = oracle(&data);
            check(&data, &want, Kernels::best(), len == 32_769);
            if len == 32_769 {
                check(&data, &want, Kernels::portable(), false);
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
