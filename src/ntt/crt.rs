//! Chinese remaindering for the three primes (Garner), and the carry pass into limbs.
//!
//! A product coefficient is at most `min(na, nb) * (2^32 - 1)^2`. Products have at most
//! `2^23` coefficients (`2 (a.len() + b.len())` is at most the transform length `L`,
//! which the tables bound), so `min(na, nb) <= 2^22` and the coefficient is below
//! `2^86`, less than `q1 q2 q3 ~ 2^89.35`: its residues `r1, r2, r3` determine it. With
//! `v2 = (r2 - r1) / q1 mod q2` and `v3 = ((r3 - r1) / q1 - v2) / q2 mod q3`, it is
//! `r1 + q1 v2 + q1 q2 v3`. The kernels compute `v2` and `v3`; [`carry`] the rest.

use super::tables::PRIMES;

/// Garner's inverses in Montgomery form, with their [`super::tables::Prime::wq`]
/// companions: `c12 = 1 / q1 mod q2`, `c13 = 1 / q1 mod q3`, `c23 = 1 / q2 mod q3`.
#[derive(Clone, Copy)]
pub(crate) struct Garner {
    pub c12: i32,
    pub c12q: i32,
    pub c13: i32,
    pub c13q: i32,
    pub c23: i32,
    pub c23q: i32,
}

pub(crate) const GARNER: Garner = {
    let [p1, p2, p3] = PRIMES;
    let (q1, q2, q3) = (p1.q as u64, p2.q as u64, p3.q as u64);
    let c12 = p2.to_mont(inv(q1 % q2, q2));
    let c13 = p3.to_mont(inv(q1 % q3, q3));
    let c23 = p3.to_mont(inv(q2 % q3, q3));
    Garner { c12, c12q: p2.wq(c12), c13, c13q: p3.wq(c13), c23, c23q: p3.wq(c23) }
};

/// `1 / a mod q` for prime `q < 2^32`, as `a^(q - 2)`.
const fn inv(a: u64, q: u64) -> u32 {
    let (mut r, mut b, mut e) = (1, a, q - 2);
    while e > 0 {
        if e & 1 == 1 {
            r = r * b % q;
        }
        b = b * b % q;
        e >>= 1;
    }
    r as u32
}

/// Writes `sum_i (r1[i] + q1 v2[i] + q1 q2 v3[i]) 2^(32 i)` to `out`. The inputs are
/// the kernels' Garner output, all in `[0, q)`, and at least `2 * out.len()` long; the
/// sum must fit in `out`.
pub(crate) fn carry(out: &mut [u64], r: [&[i32]; 3]) {
    let c = carry_out(out, r);
    debug_assert_eq!(c, 0, "product does not fit");
}

/// [`carry`] of `2 L` coefficients modulo `β^L - 1`, for `L = out.len()`: the carry out of
/// the top limb wraps to the bottom. Writes some value in `[0, β^L - 1]`.
pub(crate) fn carry_wrapped(out: &mut [u64], r: [&[i32]; 3]) {
    let c = carry_out(out, r);
    // `c β^L ≡ c`. A carry out of that addition wraps once more, onto a value that is now
    // at most `β^L - 2`, so it stops there.
    let c = u64::try_from(c).expect("the carry is below 2^57");
    if crate::mul::add_assign(out, &[c]) {
        let carry = crate::mul::add_assign(out, &[1]);
        debug_assert!(!carry);
    }
}

/// [`carry`], returning the carry out of the top limb.
fn carry_out(out: &mut [u64], r: [&[i32]; 3]) -> u128 {
    let [p1, p2, _] = PRIMES;
    let q1 = p1.q as u128;
    let q12 = q1 * p2.q as u128;
    // Garner output is non-negative, so going through `u32` zero-extends exactly.
    let coef = |r: i32, v2: i32, v3: i32| {
        r as u32 as u128 + q1 * v2 as u32 as u128 + q12 * v3 as u32 as u128
    };
    // Two coefficients per limb.
    let n = 2 * out.len();
    let [r1, v2, v3] = r.map(|x| x[..n].as_chunks::<2>().0);
    let mut c = 0u128;
    for (((o, x), y), z) in out.iter_mut().zip(r1).zip(v2).zip(v3) {
        // Coefficients are below 2^87 (2^86 for full products), so `c` stays below 2^120.
        c += coef(x[0], y[0], z[0]) + (coef(x[1], y[1], z[1]) << 32);
        *o = c as u64;
        c >>= 64;
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ntt::portable;
    use crate::ntt::tables::tests::modq;
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    #[test]
    fn garner_constants() {
        let [p1, p2, p3] = PRIMES;
        let r = |p: crate::ntt::tables::Prime| modq(1 << 32, p.q);
        // Montgomery form of 1/x times x is 2^32.
        assert_eq!(modq(GARNER.c12 as i128 * p1.q as i128, p2.q), r(p2));
        assert_eq!(modq(GARNER.c13 as i128 * p1.q as i128, p3.q), r(p3));
        assert_eq!(modq(GARNER.c23 as i128 * p2.q as i128, p3.q), r(p3));
        assert_eq!(GARNER.c12q, p2.wq(GARNER.c12));
        assert_eq!(GARNER.c13q, p3.wq(GARNER.c13));
        assert_eq!(GARNER.c23q, p3.wq(GARNER.c23));
    }

    /// Residues of `coefs` in `(-q, q)`, both signs, through Garner and the carry,
    /// against adding the coefficients directly.
    #[test]
    fn reconstructs_coefficients() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(12);
        let max = (1u128 << 86) - 1;
        let mut coefs: Vec<u128> = vec![0, 1, max, max - 1, 1 << 64, (1 << 85) + 12345];
        coefs.extend((0..2000).map(|_| (rng.next_u64() as u128) << 22 ^ rng.next_u64() as u128));
        coefs.extend((0..2000).map(|_| max - rng.next_u64() as u128));
        // An even count, plus a zero top coefficient so the product fits `out`.
        coefs.truncate(coefs.len() / 4 * 4 - 2);
        coefs.extend([0, 0]);
        let mut r: [Vec<i32>; 3] = std::array::from_fn(|i| {
            let q = PRIMES[i].q;
            // Alternate representatives in (0, q) and (-q, 0].
            let residue = |(j, &c): (usize, &u128)| (c % q as u128) as i32 - (j % 2) as i32 * q;
            coefs.iter().enumerate().map(residue).collect()
        });
        let [r1, r2, r3] = &mut r;
        portable::garner([r1, r2, r3]);

        let mut want = vec![0u64; coefs.len() / 2 + 2];
        let mut c = 0u128;
        for (k, w) in want.iter_mut().enumerate() {
            let at = |i: usize| coefs.get(i).copied().unwrap_or(0);
            c += at(2 * k) + (at(2 * k + 1) << 32);
            *w = c as u64;
            c >>= 64;
        }
        assert_eq!(c, 0);
        let mut got = vec![0u64; coefs.len() / 2];
        carry(&mut got, [&r[0], &r[1], &r[2]]);
        assert_eq!(got[..], want[..got.len()]);
        assert!(want[got.len()..].iter().all(|&w| w == 0));
    }
}
