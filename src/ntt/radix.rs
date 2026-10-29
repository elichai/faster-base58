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
const LEAF_DIGITS: usize = crate::max_encoded_len(LEAF_BYTES).div_ceil(3);
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
    )
))]
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

pub(super) fn encode_with(
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
    let mut arithmetic = Arithmetic {
        kernels,
        tw,
        work: vec![0; 3 * points],
        sums: Vec::with_capacity(2 * THRESHOLD),
    };
    let mut powers = Vec::with_capacity(levels);
    let power_capacity = |bytes| crate::max_encoded_len(bytes + 1).div_ceil(3);
    // Product slots need their untrimmed length, even when the power loses a digit.
    let storage_len = power_capacity(unit)
        + (1..levels)
            .map(|i| 2 * power_capacity(unit << (i - 1)))
            .sum::<usize>();
    let mut storage = vec![0; storage_len];
    let spectrum_len = (0..levels - 1)
        .map(|i| 2 * (2 * power_capacity(unit << i)).max(8).next_power_of_two())
        .sum();
    let mut spectra = vec![0; spectrum_len];
    let mut spectrum_storage = &mut spectra[..];
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
    let len = convert(x, &powers, &mut arithmetic, out, scratch);
    Some(write(zeros, &out[..len], output))
}

struct Power<'a> {
    bytes: usize,
    digits: &'a [u32],
    spectrum: Option<[&'a [i32]; 2]>,
}

struct Arithmetic<'a> {
    kernels: Kernels,
    tw: &'a [Twiddles],
    work: Vec<i32>,
    sums: Vec<u64>,
}

impl Arithmetic<'_> {
    fn prepare<'a>(&self, b: &[u32], storage: &'a mut [i32]) -> [&'a [i32]; 2] {
        let n = (2 * b.len()).max(8).next_power_of_two();
        assert_eq!(storage.len(), 2 * n);
        for (j, f) in storage.chunks_exact_mut(n).enumerate() {
            load(f, b);
            (self.kernels.forward)(f, PRIMES[j], &self.tw[j]);
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
            let coefficients = b.iter().map(|&d| d as u64).chain(core::iter::repeat(0));
            carry(out, coefficients, add_len);
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
            carry(out, self.sums.iter().copied(), add_len);
        } else {
            let n = len.max(8).next_power_of_two();
            let (r, tmp) = self.work.split_at_mut(2 * n);
            let tmp = &mut tmp[..n];
            let square = core::ptr::eq(a, b);
            for (j, r) in r.chunks_exact_mut(n).enumerate() {
                let p = PRIMES[j];
                let tw = &self.tw[j];
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
            carry(out, coefficients, add_len);
        }
        out.iter().rposition(|&d| d != 0).map_or(0, |i| i + 1)
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

fn carry(out: &mut [u32], coefficients: impl Iterator<Item = u64>, add_len: usize) {
    let mut carry = 0;
    for (i, (digit, coefficient)) in out.iter_mut().zip(coefficients).enumerate() {
        // Adding a digit raises the inductive carry bound to
        // MAX_COEFFICIENT / (BASE - 1) + 1; the complete sum still fits u64.
        let add = if i < add_len { *digit as u64 } else { 0 };
        let value = coefficient + carry + add;
        *digit = (value % BASE as u64) as u32;
        carry = value / BASE as u64;
    }
    debug_assert_eq!(carry, 0);
}

/// Horner in groups of five bytes. A digit times 2^40 plus its carry is below 2^58.
fn leaf(bytes: &[u8]) -> Vec<u32> {
    let (_, bytes) = crate::encode::split_zeros(bytes);
    if bytes.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0; crate::max_encoded_len(bytes.len()).div_ceil(3)];
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

fn capacity(powers: &[Power<'_>]) -> usize {
    powers.last().map_or(LEAF_DIGITS, |p| 2 * p.digits.len())
}

fn convert(
    x: &[u8],
    powers: &[Power<'_>],
    arithmetic: &mut Arithmetic<'_>,
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
        arithmetic: &mut Arithmetic<'_>,
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
                tw: &tables.tw,
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
