//! The three NTT primes, their scalar arithmetic, and twiddle tables.
//!
//! Residues are `i32` values, not always reduced. [`Prime::mont`] and [`Prime::reduce`]
//! state the inputs they accept and the outputs they guarantee. The transforms keep
//! every stored value in `(-q, q)`.

use alloc::vec::Vec;

#[cfg(feature = "cache")]
pub(super) mod cache;

/// Every prime is `1 mod 2^MAX_LOG`, so transforms have at most `2^MAX_LOG` points.
pub(crate) const MAX_LOG: u32 = 23;

/// Barrett shift: [`Prime::reduce`] estimates `x / q` as
/// `round(floor(x * v / 2^31) / 2^S)`.
pub(crate) const S: i32 = 26;

/// Primes below `2^30` that are `1 mod 2^23`, each with a root of unity of order exactly
/// `2^23`. Their product is about `2^89.35`.
const PRIMES_AND_ROOTS: [(u32, u32); 3] = [
    (998_244_353, 15_311_432),
    (897_581_057, 872_686_320),
    (880_803_841, 273_508_579),
];

pub(crate) const PRIMES: [Prime; 3] = [
    Prime::new(PRIMES_AND_ROOTS[0].0),
    Prime::new(PRIMES_AND_ROOTS[1].0),
    Prime::new(PRIMES_AND_ROOTS[2].0),
];

#[derive(Clone, Copy, Debug)]
pub(crate) struct Prime {
    pub q: i32,
    /// `q^-1 mod 2^32`.
    pub qinv: i32,
    /// `round(2^(31 + S) / q)`.
    pub v: i32,
    /// `2^32 mod q`, the Montgomery form of 1.
    pub r32: i32,
}

impl Prime {
    const fn new(q: u32) -> Self {
        // `q * q = 1 mod 8` for odd `q`, and each Newton step doubles the correct bits.
        let mut qinv = q;
        let mut i = 0;
        while i < 4 {
            qinv = qinv.wrapping_mul(2u32.wrapping_sub(q.wrapping_mul(qinv)));
            i += 1;
        }
        let q64 = q as u64;
        Prime {
            q: q as i32,
            qinv: qinv as i32,
            v: (((1u64 << (31 + S)) + q64 / 2) / q64) as i32,
            r32: ((1u64 << 32) % q64) as i32,
        }
    }

    /// `a * w / 2^32 mod q`: exactly `(a * w - m * q) / 2^32` for `m = a * wq mod 2^32`,
    /// with `wq = self.wq(w)`.
    ///
    /// `m * q = a * w mod 2^32`, so the division is exact. For `|a| < 2^31` and
    /// `|w| <= q / 2`, or for `|a|, |w| < q`, the magnitude is below
    /// `(2^30 q + 2^31 q) / 2^32 = 0.75 q`.
    #[inline(always)]
    pub fn mont(self, a: i32, w: i32, wq: i32) -> i32 {
        let m = a.wrapping_mul(wq) as i64;
        ((a as i64 * w as i64 - m * self.q as i64) >> 32) as i32
    }

    /// `w * q^-1 mod 2^32`, the companion [`Prime::mont`] needs for `w`.
    #[inline(always)]
    pub const fn wq(self, w: i32) -> i32 {
        w.wrapping_mul(self.qinv)
    }

    /// `x mod q`, with magnitude at most `q / 2 + 32`, for any `x`.
    ///
    /// `floor(x * v / 2^31) = x * 2^S / q + d` with `-1.5 < d <= 0.5`, so the estimate
    /// is within `1/2 + 2^-25` of `x / q`.
    #[inline(always)]
    pub fn reduce(self, x: i32) -> i32 {
        let t = ((x as i64 * self.v as i64) >> 31) as i32;
        let t = (t + (1 << (S - 1))) >> S;
        x.wrapping_sub(t.wrapping_mul(self.q))
    }

    /// Maps `(-q, q)` to `[0, q)`.
    #[cfg(any(
        test,
        feature = "__internals",
        kani,
        not(all(
            target_arch = "aarch64",
            target_endian = "little",
            target_feature = "neon"
        ))
    ))]
    #[inline(always)]
    pub fn normalize(self, x: i32) -> i32 {
        x + ((x >> 31) & self.q)
    }

    /// `x * 2^32 mod q` for `x < q`, balanced: magnitude at most `(q - 1) / 2`.
    pub const fn to_mont(self, x: u32) -> i32 {
        let q = self.q as u64;
        balance(((x as u64) << 32) % q, q)
    }

    /// `k = 2^64 / n mod q` and its companion: `mont(mont(a, b, bq), k, kq) = a b / n`.
    /// Requires `n` to be a power of two dividing `2^23`.
    pub fn scale(self, n: usize) -> (i32, i32) {
        debug_assert!(n.is_power_of_two() && n <= 1usize << MAX_LOG);
        let q = self.q as u64;
        // `n` divides `q - 1`, so `n * (q - (q - 1) / n) = 1 mod q`.
        let inv = q - (q - 1) / n as u64;
        let k = balance((((inv << 32) % q) << 32) % q, q);
        (k, self.wq(k))
    }

    /// Maps `(-q, q)` to `[-(q - 1) / 2, (q - 1) / 2]`.
    fn balance(self, x: i32) -> i32 {
        let h = self.q / 2;
        if x > h {
            x - self.q
        } else if x < -h {
            x + self.q
        } else {
            x
        }
    }
}

/// `x < q` as a balanced `i32`.
const fn balance(x: u64, q: u64) -> i32 {
    if x > q / 2 {
        (x as i64 - q as i64) as i32
    } else {
        x as i32
    }
}

/// Montgomery twiddles for one prime and transforms of up to `2^log` points.
///
/// `fw[h + j]` is `w_2h^j` in Montgomery form, balanced, for `j < h` and
/// `h = 1, 2, 4, ..`. Here `w_2h` is a primitive `2h`-th root of unity and
/// `w_2h = w_4h^2`. `iw` holds `w_2h^-j` likewise. Montgomery companions are derived
/// with [`Prime::wq`] when loaded. Index 0 is unused. Stage `h` needs only `h` values,
/// so the table serves every transform of up to `2^log` points.
pub(crate) struct Twiddles {
    pub fw: Vec<i32>,
    pub iw: Vec<i32>,
}

impl Twiddles {
    fn new(p: Prime, root: u32, log: u32) -> Self {
        let (n, q) = (1usize << log, p.q as u64);
        // `root^(2^(23 - log))` has order exactly `n`.
        let w = (log..MAX_LOG).fold(root as u64, |w, _| w * w % q) as u32;
        let mut fw = vec![0; n];
        // The top stage, h = n / 2, holds powers of `w`. After `lead` serial steps,
        // `lead` chains a stride apart are independent, so the loop vectorizes.
        let h = n / 2;
        let lead = h.min(8);
        let wm = p.to_mont(w);
        let mut x = p.balance(p.r32);
        for j in 0..lead {
            fw[h + j] = x;
            x = p.balance(p.mont(x, wm, p.wq(wm)));
        }
        let (s, sq) = (x, p.wq(x));
        for j in h + lead..n {
            fw[j] = p.balance(p.mont(fw[j - lead], s, sq));
        }
        // Lower stages: w_2h^j = w_4h^2j.
        let mut h = n / 4;
        while h >= 1 {
            for j in 0..h {
                fw[h + j] = fw[2 * h + 2 * j];
            }
            h /= 2;
        }
        // w_2h^-j = -w_2h^(h - j), since w_2h^h = -1.
        let mut iw = vec![0; n];
        let mut h = 1;
        while h < n {
            iw[h] = fw[h];
            for j in 1..h {
                iw[h + j] = -fw[2 * h - j];
            }
            h *= 2;
        }
        Twiddles { fw, iw }
    }
}

/// Twiddles for all three primes.
pub(crate) struct Tables {
    /// The tables serve transforms of up to `2^log` points.
    pub log: u32,
    pub tw: [Twiddles; 3],
}

impl Tables {
    /// Requires `1 <= log <= MAX_LOG`.
    pub fn new(log: u32) -> Self {
        Tables {
            log,
            tw: Self::twiddles(log),
        }
    }

    pub(super) fn twiddles<const N: usize>(log: u32) -> [Twiddles; N] {
        assert!((1..=MAX_LOG).contains(&log), "log = {log}");
        core::array::from_fn(|i| Twiddles::new(PRIMES[i], PRIMES_AND_ROOTS[i].1, log))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    pub(crate) fn pow(mut b: u64, mut e: u64, q: u64) -> u64 {
        let mut r = 1;
        b %= q;
        while e > 0 {
            if e & 1 == 1 {
                r = r * b % q;
            }
            b = b * b % q;
            e >>= 1;
        }
        r
    }

    /// `x mod q` in `[0, q)`.
    pub(crate) fn modq(x: i128, q: i32) -> u64 {
        x.rem_euclid(q as i128) as u64
    }

    #[test]
    fn primes_and_roots() {
        let mut product = 1u128;
        for (q, root) in PRIMES_AND_ROOTS {
            let q = q as u64;
            assert!(q < 1 << 30 && (q - 1).is_multiple_of(1 << 23), "{q}");
            let prime = (2..)
                .take_while(|d| d * d <= q)
                .all(|d| !q.is_multiple_of(d));
            assert!(prime, "{q} is not prime");
            // root^(2^22) = -1, so the order is exactly 2^23.
            assert_eq!(pow(root as u64, 1 << 22, q), q - 1, "{q}");
            product *= q as u128;
        }
        assert!(product > 1 << 89);
    }

    #[test]
    fn constants() {
        for p in PRIMES {
            let q = p.q as u64;
            assert_eq!(p.q.wrapping_mul(p.qinv), 1);
            assert_eq!(p.r32 as u64, (1 << 32) % q);
            assert_eq!(p.v as u64, ((1u64 << (31 + S)) + q / 2) / q);
            for log in 0..=MAX_LOG {
                let n = 1usize << log;
                let (k, kq) = p.scale(n);
                assert_eq!(kq, p.wq(k));
                assert_eq!(
                    modq(k as i128 * n as i128, p.q),
                    modq(1 << 64, p.q),
                    "n = {n}"
                );
            }
        }
    }

    /// Covers the complete legal length domain, including 1 and `2^MAX_LOG`.
    #[test]
    fn scale_matches_integer_division() {
        for p in PRIMES {
            let q = p.q as u128;
            for log in 0..=MAX_LOG {
                let n = 1usize << log;
                let (k, kq) = p.scale(n);
                let residue = ((1u128 << 64) / n as u128) % q;
                let want = if 2 * residue > q {
                    residue as i128 - q as i128
                } else {
                    residue as i128
                };
                assert_eq!(k as i128, want, "q {q}, log {log}");
                assert!((k as i64).abs() <= (p.q / 2) as i64);
                assert_eq!((kq as u32).wrapping_mul(p.q as u32), k as u32);

                let edges = [0, 1, -1, p.q - 1, 1 - p.q, p.q / 2, -(p.q / 2)];
                for a in edges {
                    for b in edges {
                        let product = p.mont(p.mont(a, b, p.wq(b)), k, kq);
                        assert_eq!(
                            modq(product as i128 * n as i128, p.q),
                            modq(a as i128 * b as i128, p.q),
                            "q {q}, log {log}, a {a}, b {b}"
                        );
                        assert!((product as i64).abs() < p.q as i64);
                    }
                }
            }
        }
    }

    #[test]
    #[cfg_attr(debug_assertions, should_panic(expected = "n.is_power_of_two()"))]
    #[cfg_attr(not(debug_assertions), should_panic(expected = "divide by zero"))]
    fn scale_rejects_zero() {
        PRIMES[0].scale(0);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "n.is_power_of_two()")]
    fn scale_rejects_non_power_of_two() {
        PRIMES[0].scale(3);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "n <= 1usize << MAX_LOG")]
    fn scale_rejects_length_above_max_log() {
        PRIMES[0].scale(1usize << (MAX_LOG + 1));
    }

    /// A smaller table is a prefix of a larger one, which [`Tables::cached`] relies on.
    #[test]
    fn smaller_tables_are_prefixes() {
        let big = Tables::new(16);
        for log in [1, 5, 10, 15] {
            let small = Tables::new(log);
            for (s, b) in small.tw.iter().zip(&big.tw) {
                for (x, y) in [(&s.fw, &b.fw), (&s.iw, &b.iw)] {
                    assert_eq!(x[..], y[..x.len()], "log {log}");
                }
            }
        }
    }

    /// Values of magnitude below `bound`: the edges, then `count` random ones.
    pub(crate) fn samples(rng: &mut Xoshiro256PlusPlus, bound: i32, count: usize) -> Vec<i32> {
        let edges = [0, 1, -1, bound - 1, 1 - bound, bound / 2, -bound / 2];
        let span = 2 * bound as u64 - 1;
        let offset = bound as i64 - 1;
        let random = (0..count).map(|_| ((rng.next_u64() % span) as i64 - offset) as i32);
        edges.into_iter().chain(random).collect()
    }

    #[test]
    fn mont_is_exact_and_bounded() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(10);
        for p in PRIMES {
            let wide = samples(&mut rng, i32::MAX, 2000);
            let half = samples(&mut rng, p.q / 2 + 1, 200);
            let small = samples(&mut rng, p.q, 2000);
            let pairs = wide.iter().flat_map(|&a| half.iter().map(move |&w| (a, w)));
            for (a, w) in pairs.chain(small.iter().zip(small.iter().rev()).map(|(&a, &w)| (a, w))) {
                let r = p.mont(a, w, p.wq(w));
                assert!(4 * (r as i64).abs() < 3 * p.q as i64, "{a} * {w}: {r}");
                assert_eq!(
                    modq((r as i128) << 32, p.q),
                    modq(a as i128 * w as i128, p.q)
                );
            }
        }
    }

    #[test]
    fn reduce_is_bounded() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(11);
        for p in PRIMES {
            let edges = [
                i32::MIN,
                i32::MAX,
                p.q / 2,
                p.q / 2 + 1,
                -(p.q / 2) - 1,
                3 * (p.q / 2),
            ];
            for x in edges
                .into_iter()
                .chain(samples(&mut rng, i32::MAX, 100_000))
            {
                let r = p.reduce(x);
                assert!(r.abs() <= p.q / 2 + 32, "{x}: {r}");
                assert_eq!(modq(r as i128, p.q), modq(x as i128, p.q));
            }
        }
    }

    #[test]
    fn twiddle_heap_budget() {
        assert_eq!(size_of::<Twiddles>(), 2 * size_of::<Vec<i32>>());
        let t = Tables::new(16);
        let bytes =
            t.tw.iter()
                .flat_map(|tw| [&tw.fw, &tw.iw])
                .map(|v| v.capacity() * size_of::<i32>())
                .sum::<usize>();
        assert!(bytes <= 1536 * 1024, "twiddle heap: {bytes} bytes");
    }

    #[test]
    fn twiddles() {
        for log in (1..=12).chain([16]) {
            let t = Tables::new(log);
            for ((p, (_, root)), tw) in PRIMES.iter().zip(PRIMES_AND_ROOTS).zip(&t.tw) {
                let q = p.q as u64;
                let r = (1u64 << 32) % q;
                let mut h = 1;
                while h < 1 << log {
                    // w_2h = root^(2^23 / 2h).
                    let w = pow(root as u64, (1 << 23) / (2 * h as u64), q);
                    for j in 0..h {
                        let want = pow(w, j as u64, q) * r % q;
                        let got = modq(tw.fw[h + j] as i128, p.q);
                        assert_eq!(got, want, "log {log}, h {h}, j {j}");
                        // Montgomery forms of x and 1/x multiply to 2^64.
                        let prod = tw.fw[h + j] as i128 * tw.iw[h + j] as i128;
                        assert_eq!(modq(prod, p.q), r * r % q);
                        assert!(tw.fw[h + j].abs() <= p.q / 2 && tw.iw[h + j].abs() <= p.q / 2);
                    }
                    h *= 2;
                }
            }
        }
    }

    /// A cached table serves at least the size asked for, and stays: asking for its own
    /// size returns it. Other tests may have cached larger tables first, so the sizes
    /// returned depend on test order; these properties do not.
    #[cfg(feature = "cache")]
    #[test]
    fn cache_builds_once() {
        for log in [3, 12, 6, 9, 12] {
            let a = Tables::cached(log);
            assert!(a.log >= log, "log {log}");
            assert!(core::ptr::eq(a, Tables::cached(a.log)), "log {log}");
        }
    }
}
