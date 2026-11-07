//! Transforms modulo two primes below `2^50` with AVX-512 IFMA, eight `u64` lanes per
//! vector, for CPUs that have it.
//!
//! Butterflies are Harvey's lazy ones (arXiv 1205.2926, Algorithms 3 and 4) with Shoup
//! twiddles `(w, floor(w 2^52 / p))`, as in Intel HEXL (arXiv 2103.16400): `p < 2^52 / 4`
//! keeps every value below `4 p < 2^52`, the width `vpmadd52*` reads. A forward
//! transform takes and returns `[0, 2p)`; an inverse takes `[0, 2p)` and returns
//! `[0, 4p)`, which the polynomial CRT helpers and [`garner_v`] accept.
//!
//! Only forward roots are stored: the inverse root `w^-j` of a stage is `-w^(h-j)`, so
//! the inverse transform and [`untwist_v`] read the forward roots backwards and swap
//! the sum and difference.
//!
//! The kernels are written against [`L8`]: `__m512i` on x86-64, and an emulation in tests,
//! so tests run them on every target.

use alloc::boxed::Box;
use alloc::vec;

use super::tables::MAX_LOG;

/// `2^52 - 1`.
pub(crate) const MASK: u64 = (1 << 52) - 1;

/// Primes `c 2^26 + 1` below `2^50`, each with a root of unity of order exactly `2^23`.
/// Their product exceeds `2^99.99`.
const PRIMES_AND_ROOTS: [(u64, u64); 2] = [
    (1_125_899_437_080_577, 246_330_078_034_315),
    (1_125_899_302_862_849, 289_465_001_164_053),
];

pub(crate) const PRIMES: [Prime52; 2] = [
    Prime52::new(PRIMES_AND_ROOTS[0].0),
    Prime52::new(PRIMES_AND_ROOTS[1].0),
];

/// A prime below `2^50` and its constants.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Prime52 {
    pub p: u64,
    /// `2^52 - p`: `madd52lo` adds `-q p mod 2^52` with it.
    negp: u64,
    /// `-1 / p mod 2^52`, for Montgomery reduction by `2^52`.
    j: u64,
    /// `floor(2^101 / p)`, below `2^52` since `p > 2^49`.
    r101: u64,
    /// `p - 1` (that is, `-1`) and `1 / 2`, with their Shoup companions.
    m1: (u64, u64),
    half: (u64, u64),
}

/// `floor(w 2^52 / p)`, the Shoup companion of `w < p`.
pub(crate) const fn shoup(w: u64, p: u64) -> u64 {
    (((w as u128) << 52) / p as u128) as u64
}

const fn mulmod(a: u64, b: u64, p: u64) -> u64 {
    (a as u128 * b as u128 % p as u128) as u64
}

const fn powmod(mut b: u64, mut e: u64, p: u64) -> u64 {
    let mut r = 1;
    while e > 0 {
        if e & 1 == 1 {
            r = mulmod(r, b, p);
        }
        b = mulmod(b, b, p);
        e >>= 1;
    }
    r
}

impl Prime52 {
    const fn new(p: u64) -> Self {
        assert!(p < 1 << 50 && p > 1 << 49 && (p - 1).is_multiple_of(1 << MAX_LOG));
        // `p p = 1 mod 8`, and each Newton step doubles the correct bits.
        let mut inv = p;
        let mut i = 0;
        while i < 6 {
            inv = inv.wrapping_mul(2u64.wrapping_sub(p.wrapping_mul(inv)));
            i += 1;
        }
        let half = p.div_ceil(2);
        Prime52 {
            p,
            negp: (1 << 52) - p,
            j: inv.wrapping_neg() & MASK,
            r101: ((1u128 << 101) / p as u128) as u64,
            m1: (p - 1, shoup(p - 1, p)),
            half: (half, shoup(half, p)),
        }
    }

    /// `2^52 / n mod p` and its companion: [`pointwise_v`]'s Montgomery product then
    /// divides by `n`. Requires `n` to be a power of two dividing `2^MAX_LOG`.
    fn scale(self, n: usize) -> (u64, u64) {
        debug_assert!(n.is_power_of_two() && n <= 1 << MAX_LOG);
        // `n` divides `p - 1`, so `n (p - (p - 1) / n) = 1 mod p`.
        let inv = self.p - (self.p - 1) / n as u64;
        let k = mulmod(inv, (1 << 52) % self.p, self.p);
        (k, shoup(k, self.p))
    }

    /// `w t mod p` in `[0, 2p)` for `t < 2^52` (Shoup).
    #[inline(always)]
    fn mulw(self, t: u64, w: (u64, u64)) -> u64 {
        let q = ((w.1 as u128 * t as u128) >> 52) as u64;
        w.0.wrapping_mul(t).wrapping_sub(q.wrapping_mul(self.p)) & MASK
    }
}

/// `x - m` if `x >= m`.
#[inline(always)]
fn csub(x: u64, m: u64) -> u64 {
    x.min(x.wrapping_sub(m))
}

/// Forward roots of one prime: `w[h + j] = w_2h^j` in `[0, p)` for `j < h`, with
/// `w_2h = w_4h^2` primitive, and Shoup companions in `wp`, as in
/// [`super::tables::Twiddles`]. Eight more entries follow the last stage: the inverse
/// transform reads one past a stage (and replaces it). One boxed buffer holds `w`, `wp`
/// and [`Tw52::small`].
pub(crate) struct Tw52 {
    /// The prime's index in [`PRIMES`].
    j: usize,
    roots: Box<[u64]>,
}

/// The `u64`s of [`Tw52::small`].
const SMALL: usize = 32;

impl Tw52 {
    pub(crate) fn prime(&self) -> Prime52 {
        PRIMES[self.j]
    }

    /// Points the roots serve.
    pub(crate) fn len(&self) -> usize {
        self.stage_len() - 8
    }

    fn stage_len(&self) -> usize {
        (self.roots.len() - SMALL) / 2
    }

    fn w(&self) -> &[u64] {
        &self.roots[..self.stage_len()]
    }

    fn wp(&self) -> &[u64] {
        &self.roots[self.stage_len()..2 * self.stage_len()]
    }

    /// Inverse roots of the stages `h = 2, 4` in their register layouts, each followed by
    /// its companions.
    /// Whether both hold the same roots, companions and inverse stages.
    #[cfg(test)]
    pub(crate) fn roots_eq(&self, other: &Tw52) -> bool {
        self.j == other.j && self.roots == other.roots
    }

    fn small(&self) -> &[[u64; 8]] {
        self.roots[2 * self.stage_len()..].as_chunks().0
    }

    fn new(j: usize, log: u32, k: &Kernels52) -> Self {
        let (p, root) = PRIMES_AND_ROOTS[j];
        let n = 1usize << log;
        // `root^(2^(23 - log))` has order exactly `n`.
        let w = (log..MAX_LOG).fold(root, |w, _| mulmod(w, w, p));
        let mut roots = vec![0; 2 * (n + 8) + SMALL].into_boxed_slice();
        let (ws, rest) = roots.split_at_mut(n + 8);
        let (wp, small) = rest.split_at_mut(n + 8);
        (k.roots)(ws, wp, PRIMES[j], w);
        let small = small.as_chunks_mut::<8>().0;
        for (i, h) in [2usize, 4].into_iter().enumerate() {
            if 2 * h > n {
                break;
            }
            let iw: [u64; 8] = core::array::from_fn(|l| match l & (h - 1) {
                0 => 1,
                jj => p - ws[2 * h - jj],
            });
            small[2 * i] = iw;
            small[2 * i + 1] = iw.map(|w| shoup(w, p));
        }
        Tw52 { j, roots }
    }
}

/// Twiddles for both primes and transforms of up to `2^log` points.
pub(crate) struct Tables52 {
    pub log: u32,
    pub tw: [Tw52; 2],
}

impl Tables52 {
    /// Requires `1 <= log <= MAX_LOG`.
    pub(crate) fn new(log: u32, k: &Kernels52) -> Self {
        assert!((1..=MAX_LOG).contains(&log), "log = {log}");
        Tables52 {
            log,
            tw: [Tw52::new(0, log, k), Tw52::new(1, log, k)],
        }
    }
}

#[cfg(feature = "cache")]
pub(super) mod cache {
    use super::{Kernels52, Tables52};
    use crate::ntt::tables::cache::CACHE_MAX_LOG;
    use std::sync::OnceLock;

    static CACHE: [OnceLock<Tables52>; CACHE_MAX_LOG as usize + 1] =
        [const { OnceLock::new() }; CACHE_MAX_LOG as usize + 1];

    impl Tables52 {
        /// As [`crate::ntt::tables::Tables::cached`]: a larger cached table serves too.
        /// Each log retains `32 2^log + 768` data bytes.
        pub(crate) fn cached(log: u32, k: &Kernels52) -> &'static Tables52 {
            assert!(
                (1..=CACHE_MAX_LOG).contains(&log),
                "uncacheable log = {log}"
            );
            let log = log as usize;
            CACHE[log..]
                .iter()
                .find_map(OnceLock::get)
                .unwrap_or_else(|| CACHE[log].get_or_init(|| Tables52::new(log as u32, k)))
        }
    }

    /// The static header bytes, and the payload bytes of the table cached for `log`.
    #[cfg(feature = "__internals")]
    pub(crate) fn memory(log: usize) -> (usize, Option<usize>) {
        let bytes = CACHE.get(log).and_then(OnceLock::get).map(|t| {
            t.tw.iter()
                .map(|tw| tw.roots.len() * size_of::<u64>())
                .sum()
        });
        (size_of_val(&CACHE), bytes)
    }
}

/// The kernels of the two-prime backend; each works on one prime's residues.
#[derive(Clone, Copy)]
pub(crate) struct Kernels52 {
    /// Fills `w` and `wp` (each `n + 8` long) for transforms of `n` points, from a
    /// root of order exactly `n`.
    pub roots: fn(w: &mut [u64], wp: &mut [u64], p: Prime52, root: u64),
    /// The 32-bit halves of `limbs` into `dst`, the rest zero.
    pub load: fn(dst: &mut [u64], limbs: &[u64]),
    /// [`Kernels52::load`], coefficient `i` times `w_2m^i` (`m = dst.len()`).
    pub load_twist: fn(dst: &mut [u64], limbs: &[u64], tw: &Tw52),
    /// Adds (or with `sub` subtracts) the 32-bit halves of `limbs` to the start of `dst`.
    pub load_add: fn(dst: &mut [u64], limbs: &[u64], p: Prime52, sub: bool),
    /// `a[i] w_2m^i` for `m = a.len()`.
    pub twist: fn(a: &mut [u64], tw: &Tw52),
    /// `a[i] w_2m^-(start + i)`; `a.len() + start <= m`.
    pub untwist: fn(a: &mut [u64], m: usize, start: usize, tw: &Tw52),
    /// `dst[i] = s[i] mod p` for `s[i] < 2^63`.
    pub narrow: fn(dst: &mut [u64], s: &[u64], p: Prime52),
    pub forward: fn(a: &mut [u64], tw: &Tw52),
    pub inverse: fn(a: &mut [u64], tw: &Tw52),
    /// `a[i] = a[i] b[i] / a.len()`.
    pub pointwise: fn(a: &mut [u64], b: &[u64], p: Prime52),
    pub square: fn(a: &mut [u64], p: Prime52),
    /// As [`super::Kernels::fold_sub`].
    pub fold_sub: fn(dst: &mut [u64], src: &[u64], start: usize, neg: bool, p: Prime52),
    /// As [`super::Kernels::halves`].
    pub halves: fn(lo: &mut [u64], hi: &mut [u64], p: Prime52),
    /// `r1` to `[0, p1)`, `r2` to `(r2 - r1) / p1 mod p2` in `[0, p2)`.
    pub garner: fn(r1: &mut [u64], r2: &mut [u64]),
}

impl Kernels52 {
    /// The AVX-512 IFMA kernels when this CPU has them.
    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
    pub(crate) fn best() -> Option<Self> {
        crate::cpu::ifma().then(z::kernels)
    }

    #[cfg(not(all(target_arch = "x86_64", target_feature = "sse2")))]
    pub(crate) fn best() -> Option<Self> {
        None
    }
}

/// [`Kernels52`] from the generic kernels on `$v`, for tests.
#[cfg(test)]
macro_rules! kernels {
    ($v:ty) => {
        Kernels52 {
            // SAFETY (each): the caller of the macro guarantees `$v` may be created.
            roots: |w, wp, p, r| unsafe { super::roots_v::<$v>(w, wp, p, r) },
            load: |d, l| unsafe { super::load_v::<$v>(d, l) },
            load_twist: |d, l, t| unsafe { super::load_twist_v::<$v>(d, l, t) },
            load_add: |d, l, p, s| unsafe { super::load_add_v::<$v>(d, l, p, s) },
            twist: |a, t| unsafe { super::twist_v::<$v>(a, t) },
            untwist: |a, m, s, t| unsafe { super::untwist_v::<$v>(a, m, s, t) },
            narrow: |d, s, p| unsafe { super::narrow_v::<$v>(d, s, p) },
            forward: |a, t| unsafe { super::forward_v::<$v>(a, t) },
            inverse: |a, t| unsafe { super::inverse_v::<$v>(a, t) },
            pointwise: |a, b, p| unsafe { super::pointwise_v::<$v>(a, b, p) },
            square: |a, p| unsafe { super::square_v::<$v>(a, p) },
            fold_sub: |d, s, st, n, p| unsafe { super::fold_sub_v::<$v>(d, s, st, n, p) },
            halves: |l, h, p| unsafe { super::halves_v::<$v>(l, h, p) },
            garner: |a, b| unsafe { super::garner_v::<$v>(a, b) },
        }
    };
}

/// Eight `u64` lanes. For `__m512i` the CPU must have AVX-512F and IFMA: creating a value is
/// `unsafe` with that contract, so holding one proves it and the other methods are safe.
pub(crate) trait L8: Copy {
    unsafe fn splat(x: u64) -> Self;
    unsafe fn from_array(x: [u64; 8]) -> Self;
    /// Requires 8 readable lanes at `p`.
    unsafe fn load(p: *const u64) -> Self;
    /// Eight `u32` at `p`, zero-extended; requires them readable.
    unsafe fn load_u32(p: *const u32) -> Self;
    /// Requires 8 writable lanes at `p`.
    unsafe fn store(self, p: *mut u64);
    fn add(self, b: Self) -> Self;
    fn sub(self, b: Self) -> Self;
    /// Unsigned lane minima.
    fn min(self, b: Self) -> Self;
    fn and(self, b: Self) -> Self;
    fn or(self, b: Self) -> Self;
    fn shl<const N: u32>(self) -> Self;
    fn shr<const N: u32>(self) -> Self;
    /// `self` plus the low (high) 52 bits of the product of the low 52 bits of `x`, `y`.
    fn madd52lo(self, x: Self, y: Self) -> Self;
    fn madd52hi(self, x: Self, y: Self) -> Self;
    /// Lane `i` is lane `idx[i] % 8` of `self`.
    fn permute(self, idx: Self) -> Self;
    /// Lane `i` is lane `idx[i] % 16` of `self` followed by `b`.
    fn permute2(self, idx: Self, b: Self) -> Self;
    /// Lane 0 of `b`, the others of `self`.
    fn set0(self, b: Self) -> Self;
    /// `self + 1` in the lanes where `x != 0`.
    fn inc_nonzero(self, x: Self) -> Self;
    /// `self + 1` in the lanes where `r >= m`.
    fn inc_ge(self, r: Self, m: Self) -> Self;
}

/// `L` vectors as one, each operation applied to each in turn, so that code on them lists
/// the steps of `L` independent computations together (see `avx512.rs`).
impl<V: L8, const L: usize> L8 for [V; L] {
    #[inline(always)]
    unsafe fn splat(x: u64) -> Self {
        // SAFETY: the caller permits V creation.
        [unsafe { V::splat(x) }; L]
    }
    #[inline(always)]
    unsafe fn from_array(x: [u64; 8]) -> Self {
        // SAFETY: the caller permits V creation.
        [unsafe { V::from_array(x) }; L]
    }
    #[inline(always)]
    unsafe fn load(p: *const u64) -> Self {
        // SAFETY: the caller provides 8 L readable lanes and permits V creation.
        core::array::from_fn(|i| unsafe { V::load(p.add(8 * i)) })
    }
    #[inline(always)]
    unsafe fn load_u32(p: *const u32) -> Self {
        // SAFETY: as `load`.
        core::array::from_fn(|i| unsafe { V::load_u32(p.add(8 * i)) })
    }
    #[inline(always)]
    unsafe fn store(self, p: *mut u64) {
        for (i, v) in self.into_iter().enumerate() {
            // SAFETY: the caller provides 8 L writable lanes.
            unsafe { v.store(p.add(8 * i)) }
        }
    }
    #[inline(always)]
    fn add(self, b: Self) -> Self {
        core::array::from_fn(|i| self[i].add(b[i]))
    }
    #[inline(always)]
    fn sub(self, b: Self) -> Self {
        core::array::from_fn(|i| self[i].sub(b[i]))
    }
    #[inline(always)]
    fn min(self, b: Self) -> Self {
        core::array::from_fn(|i| self[i].min(b[i]))
    }
    #[inline(always)]
    fn and(self, b: Self) -> Self {
        core::array::from_fn(|i| self[i].and(b[i]))
    }
    #[inline(always)]
    fn or(self, b: Self) -> Self {
        core::array::from_fn(|i| self[i].or(b[i]))
    }
    #[inline(always)]
    fn shl<const N: u32>(self) -> Self {
        self.map(V::shl::<N>)
    }
    #[inline(always)]
    fn shr<const N: u32>(self) -> Self {
        self.map(V::shr::<N>)
    }
    #[inline(always)]
    fn madd52lo(self, x: Self, y: Self) -> Self {
        core::array::from_fn(|i| self[i].madd52lo(x[i], y[i]))
    }
    #[inline(always)]
    fn madd52hi(self, x: Self, y: Self) -> Self {
        core::array::from_fn(|i| self[i].madd52hi(x[i], y[i]))
    }
    #[inline(always)]
    fn permute(self, idx: Self) -> Self {
        core::array::from_fn(|i| self[i].permute(idx[i]))
    }
    #[inline(always)]
    fn permute2(self, idx: Self, b: Self) -> Self {
        core::array::from_fn(|i| self[i].permute2(idx[i], b[i]))
    }
    #[inline(always)]
    fn set0(self, b: Self) -> Self {
        core::array::from_fn(|i| self[i].set0(b[i]))
    }
    #[inline(always)]
    fn inc_nonzero(self, x: Self) -> Self {
        core::array::from_fn(|i| self[i].inc_nonzero(x[i]))
    }
    #[inline(always)]
    fn inc_ge(self, r: Self, m: Self) -> Self {
        core::array::from_fn(|i| self[i].inc_ge(r[i], m[i]))
    }
}

/// The constants of one prime.
#[derive(Clone, Copy)]
struct C<V> {
    p: V,
    p2: V,
    p4: V,
    negp: V,
    mask: V,
    zero: V,
}

impl<V: L8> C<V> {
    /// Safety: `V` values may be created.
    #[inline(always)]
    unsafe fn new(p: Prime52) -> Self {
        // SAFETY: the caller permits V creation.
        unsafe {
            C {
                p: V::splat(p.p),
                p2: V::splat(2 * p.p),
                p4: V::splat(4 * p.p),
                negp: V::splat(p.negp),
                mask: V::splat(MASK),
                zero: V::splat(0),
            }
        }
    }
}

/// [`Prime52::mulw`]: `(w t - floor(wp t / 2^52) p) mod 2^52`, in `[0, 2p)` for `t < 2^52`.
#[inline(always)]
fn mulw<V: L8>(t: V, (w, wp): (V, V), c: &C<V>) -> V {
    let q = c.zero.madd52hi(wp, t);
    c.zero.madd52lo(w, t).madd52lo(q, c.negp).and(c.mask)
}

#[inline(always)]
fn vcsub<V: L8>(x: V, m: V) -> V {
    x.min(x.sub(m))
}

/// Harvey's forward (DIF) butterfly, `[0, 2p)` in and out.
#[inline(always)]
fn dif<V: L8>(x: V, y: V, w: (V, V), c: &C<V>) -> (V, V) {
    (vcsub(x.add(y), c.p2), mulw(x.add(c.p2.sub(y)), w, c))
}

/// Harvey's inverse (DIT) butterfly `(x + w y, x - w y)`, `[0, 4p)` in and out.
#[inline(always)]
fn dit<V: L8>(x: V, y: V, w: (V, V), c: &C<V>) -> (V, V) {
    let x = vcsub(x, c.p2);
    let t = mulw(y, w, c);
    (x.add(t), x.add(c.p2.sub(t)))
}

/// [`dit`] with `-w` given: `(x - (-w) y, x + (-w) y)`.
#[inline(always)]
fn dit_neg<V: L8>(x: V, y: V, w: (V, V), c: &C<V>) -> (V, V) {
    let (s, d) = dit(x, y, w, c);
    (d, s)
}

const REVERSE: [u64; 8] = [7, 6, 5, 4, 3, 2, 1, 0];

/// `-w^-i` for `i` in `[pos, pos + 8)` of stage `h` (`pos + 8 <= h`): the forward roots
/// `w[2h - pos - 7..=2h - pos]` reversed, and for `pos = 0`, `-1` in lane 0.
///
/// Safety: `V` values may be created; `w`, `wp` hold at least `2h + 1` entries.
#[inline(always)]
unsafe fn neg_inverse<V: L8>(tw: &Tw52, h: usize, pos: usize, rev: V, m1: (V, V)) -> (V, V) {
    let at = 2 * h - pos - 7;
    // SAFETY: `at + 8 <= 2h + 1 <= tw.w().len()`; the caller permits V creation.
    let (w, wp) = unsafe {
        (
            V::load(tw.w().as_ptr().add(at)).permute(rev),
            V::load(tw.wp().as_ptr().add(at)).permute(rev),
        )
    };
    if pos == 0 {
        (w.set0(m1.0), wp.set0(m1.1))
    } else {
        (w, wp)
    }
}

// A block of 16 values sits in two registers in "layout h", for h = 1, 2, 4, 8: the first
// holds the positions whose bit h is clear, in order, the second their partners in a
// stage of distance h. Layout 8 is the natural order (as in avx512.rs, with 8 lanes).

const fn first(i: usize, h: usize) -> usize {
    (i & (h - 1)) | ((i & !(h - 1)) << 1)
}

const fn lane(pos: usize, h: usize) -> usize {
    8 * (pos & h != 0) as usize + ((pos & (h - 1)) | ((pos >> 1) & !(h - 1)))
}

const fn regroup(from: usize, to: usize) -> [[u64; 8]; 2] {
    let mut idx = [[0; 8]; 2];
    let mut i = 0;
    while i < 8 {
        idx[0][i] = lane(first(i, to), from) as u64;
        idx[1][i] = lane(first(i, to) + to, from) as u64;
        i += 1;
    }
    idx
}

const FORWARD: [[[u64; 8]; 2]; 4] = [regroup(8, 4), regroup(4, 2), regroup(2, 1), regroup(1, 8)];
const INVERSE: [[[u64; 8]; 2]; 4] = [regroup(8, 1), regroup(1, 2), regroup(2, 4), regroup(4, 8)];

#[inline(always)]
fn regroup2<V: L8>(x: V, y: V, (i, j): (V, V)) -> (V, V) {
    (x.permute2(i, y), x.permute2(j, y))
}

/// Safety: `V` values may be created.
#[inline(always)]
unsafe fn indices<V: L8>(t: &[[[u64; 8]; 2]; 4]) -> [(V, V); 4] {
    // SAFETY: the caller permits V creation.
    t.map(|[a, b]| unsafe { (V::from_array(a), V::from_array(b)) })
}

/// Scalar forward transform, for fewer than 32 points.
fn forward_s(a: &mut [u64], tw: &Tw52) {
    let (n, p) = (a.len(), tw.prime());
    let mut h = n / 2;
    while h >= 1 {
        for blk in a.chunks_exact_mut(2 * h) {
            let (x, y) = blk.split_at_mut(h);
            for (j, (x, y)) in x.iter_mut().zip(y).enumerate() {
                let t = *x + 2 * p.p - *y;
                *x = csub(*x + *y, 2 * p.p);
                *y = p.mulw(t, (tw.w()[h + j], tw.wp()[h + j]));
            }
        }
        h /= 2;
    }
}

/// Scalar inverse transform, for fewer than 32 points.
fn inverse_s(a: &mut [u64], tw: &Tw52) {
    let (n, p) = (a.len(), tw.prime());
    let mut h = 1;
    while h < n {
        for blk in a.chunks_exact_mut(2 * h) {
            let (x, y) = blk.split_at_mut(h);
            for (j, (x, y)) in x.iter_mut().zip(y).enumerate() {
                let w = if j == 0 {
                    p.m1
                } else {
                    (tw.w()[2 * h - j], tw.wp()[2 * h - j])
                };
                let s = csub(*x, 2 * p.p);
                let t = p.mulw(*y, w);
                (*x, *y) = (s + 2 * p.p - t, s + t);
            }
        }
        h *= 2;
    }
}

/// Decimation in frequency: natural order in, bit-reversed out, `[0, 2p)` in and out.
/// `a.len()` is a power of two, at least 2, at most `tw.len()`.
///
/// Safety: `V` values may be created.
#[inline(always)]
unsafe fn forward_v<V: L8>(a: &mut [u64], tw: &Tw52) {
    let n = a.len();
    assert!(n.is_power_of_two() && n >= 2 && n <= tw.len());
    if n < 32 {
        return forward_s(a, tw);
    }
    // SAFETY: the caller permits V creation.
    let c = unsafe { C::<[V; 2]>::new(tw.prime()) };
    let pa = a.as_mut_ptr();
    let ld = |i: usize| {
        // SAFETY: callers use i + 8 <= 2 h <= n <= tw.len().
        unsafe {
            (
                V::load(tw.w().as_ptr().add(i)),
                V::load(tw.wp().as_ptr().add(i)),
            )
        }
    };
    let both = |a: (V, V), b: (V, V)| ([a.0, b.0], [a.1, b.1]);
    let mut h = n / 2;
    while h >= 8 {
        if h >= 16 {
            // Stages `h` and `g = h / 2` on four values `g` apart, as in avx512.rs.
            let g = h / 2;
            for s in (0..n).step_by(2 * h) {
                for j in (0..g).step_by(8) {
                    // SAFETY: s + 2 h <= n and j + 8 <= g: all four vectors lie in `a`.
                    unsafe {
                        let ps = [0, g, h, h + g].map(|o| pa.add(s + j + o));
                        let [x0, x1, x2, x3] = ps.map(|p| V::load(p));
                        let ([x0, x1], [x2, x3]) =
                            dif([x0, x1], [x2, x3], both(ld(h + j), ld(h + g + j)), &c);
                        let w = ld(g + j);
                        let ([x0, x2], [x1, x3]) = dif([x0, x2], [x1, x3], both(w, w), &c);
                        for (p, x) in ps.into_iter().zip([x0, x1, x2, x3]) {
                            x.store(p);
                        }
                    }
                }
            }
            h /= 4;
        } else {
            let w = ld(8);
            for s in (0..n).step_by(32) {
                // SAFETY: s + 32 <= n.
                unsafe {
                    let px = [pa.add(s), pa.add(s + 16)];
                    let (x, y) = (px.map(|p| V::load(p)), px.map(|p| V::load(p.add(8))));
                    let (x, y) = dif(x, y, both(w, w), &c);
                    for (p, (x, y)) in px.into_iter().zip(x.into_iter().zip(y)) {
                        x.store(p);
                        y.store(p.add(8));
                    }
                }
            }
            h /= 2;
        }
    }
    debug_assert_eq!(h, 4);
    // Stages 4, 2, 1 in registers, two blocks of 16 at a time.
    let small = |h: usize| {
        let at = |t: &[u64]| core::array::from_fn(|i| t[h + (i & (h - 1))]);
        // SAFETY: the caller permits V creation.
        unsafe {
            (
                [V::from_array(at(tw.w())); 2],
                [V::from_array(at(tw.wp())); 2],
            )
        }
    };
    let (w4, w2) = (small(4), small(2));
    // SAFETY: the caller permits V creation.
    let idx = unsafe { indices::<[V; 2]>(&FORWARD) };
    for i in (0..n).step_by(32) {
        // SAFETY: i + 32 <= n.
        unsafe {
            let ps = [pa.add(i), pa.add(i + 16)];
            let (x, y) = (ps.map(|p| V::load(p)), ps.map(|p| V::load(p.add(8))));
            let (x, y) = regroup2(x, y, idx[0]);
            let (x, y) = dif(x, y, w4, &c);
            let (x, y) = regroup2(x, y, idx[1]);
            let (x, y) = dif(x, y, w2, &c);
            let (x, y) = regroup2(x, y, idx[2]);
            // h = 1: the twiddle is 1.
            let (s, d) = (vcsub(x.add(y), c.p2), vcsub(x.add(c.p2.sub(y)), c.p2));
            let (x, y) = regroup2(s, d, idx[3]);
            for (p, (x, y)) in ps.into_iter().zip(x.into_iter().zip(y)) {
                x.store(p);
                y.store(p.add(8));
            }
        }
    }
}

/// Decimation in time: bit-reversed order in `[0, 2p)`, natural order out in `[0, 4p)`,
/// times `a.len()`. `a.len()` is a power of two, at least 2, at most `tw.len()`.
///
/// Safety: `V` values may be created.
#[inline(always)]
unsafe fn inverse_v<V: L8>(a: &mut [u64], tw: &Tw52) {
    let n = a.len();
    assert!(n.is_power_of_two() && n >= 2 && n <= tw.len());
    if n < 32 {
        return inverse_s(a, tw);
    }
    let pa = a.as_mut_ptr();
    {
        // SAFETY: the caller permits V creation.
        let (c, idx, w2, w4) = unsafe {
            let s = |i: usize| [V::from_array(tw.small()[i]); 2];
            (
                C::<[V; 2]>::new(tw.prime()),
                indices::<[V; 2]>(&INVERSE),
                (s(0), s(1)),
                (s(2), s(3)),
            )
        };
        for i in (0..n).step_by(32) {
            // SAFETY: i + 32 <= n.
            unsafe {
                let ps = [pa.add(i), pa.add(i + 16)];
                let (x, y) = (ps.map(|p| V::load(p)), ps.map(|p| V::load(p.add(8))));
                let (x, y) = regroup2(x, y, idx[0]);
                // h = 1, twiddle 1, inputs below 2p: no reduction.
                let (x, y) = regroup2(x.add(y), x.add(c.p2.sub(y)), idx[1]);
                let (x, y) = dit(x, y, w2, &c);
                let (x, y) = regroup2(x, y, idx[2]);
                let (x, y) = dit(x, y, w4, &c);
                let (x, y) = regroup2(x, y, idx[3]);
                for (p, (x, y)) in ps.into_iter().zip(x.into_iter().zip(y)) {
                    x.store(p);
                    y.store(p.add(8));
                }
            }
        }
    }
    // SAFETY: the caller permits V creation.
    let (c, rev, m1) = unsafe {
        (
            C::<[V; 2]>::new(tw.prime()),
            V::from_array(REVERSE),
            (V::splat(tw.prime().m1.0), V::splat(tw.prime().m1.1)),
        )
    };
    // SAFETY: stages up to `n / 2` read at most `n + 1 <= tw.w().len()` entries.
    let nw = |h: usize, pos: usize| unsafe { neg_inverse(tw, h, pos, rev, m1) };
    let both = |a: (V, V), b: (V, V)| ([a.0, b.0], [a.1, b.1]);
    let mut h = 8;
    while h < n {
        if 4 * h <= n {
            for s in (0..n).step_by(4 * h) {
                for j in (0..h).step_by(8) {
                    // SAFETY: s + 4 h <= n and j + 8 <= h.
                    unsafe {
                        let ps = [0, h, 2 * h, 3 * h].map(|o| pa.add(s + j + o));
                        let [x0, x1, x2, x3] = ps.map(|p| V::load(p));
                        let w = nw(h, j);
                        let ([x0, x2], [x1, x3]) = dit_neg([x0, x2], [x1, x3], both(w, w), &c);
                        let w = both(nw(2 * h, j), nw(2 * h, h + j));
                        let ([x0, x1], [x2, x3]) = dit_neg([x0, x1], [x2, x3], w, &c);
                        for (p, x) in ps.into_iter().zip([x0, x1, x2, x3]) {
                            x.store(p);
                        }
                    }
                }
            }
            h *= 4;
        } else {
            // The last stage, `h = n / 2 >= 16`.
            for j in (0..h).step_by(16) {
                // SAFETY: j + 16 <= h, and `2 h = n`.
                unsafe {
                    let px = [pa.add(j), pa.add(j + 8)];
                    let (x, y) = (px.map(|p| V::load(p)), px.map(|p| V::load(p.add(h))));
                    let (x, y) = dit_neg(x, y, both(nw(h, j), nw(h, j + 8)), &c);
                    for (p, (x, y)) in px.into_iter().zip(x.into_iter().zip(y)) {
                        x.store(p);
                        y.store(p.add(h));
                    }
                }
            }
            h *= 2;
        }
    }
}

/// `x` mod `p` by Montgomery reduction of `x y`, in `[0, 2p)` for `x, y < 2p`: `x y 2^-52`.
#[inline(always)]
fn mont<V: L8>(x: V, y: V, j: V, c: &C<V>) -> V {
    let lo = c.zero.madd52lo(x, y);
    let hi = c.zero.madd52hi(x, y);
    let m = c.zero.madd52lo(lo, j);
    // `lo + (m p mod 2^52)` is `2^52` unless `lo` is 0.
    hi.madd52hi(m, c.p).inc_nonzero(lo)
}

/// Scalar [`mont`].
fn mont_s(x: u64, y: u64, p: Prime52) -> u64 {
    let xy = x as u128 * y as u128;
    let (lo, hi) = (xy as u64 & MASK, (xy >> 52) as u64);
    let m = lo.wrapping_mul(p.j) & MASK;
    hi + ((m as u128 * p.p as u128) >> 52) as u64 + (lo != 0) as u64
}

/// `a[i] = a[i] b[i] / n` in `[0, 2p)`, for inputs in `[0, 2p)`. With `b = None`, squares.
///
/// `a[i] k` (Shoup, `k = 2^52 / n`) is below `2p`, and the Montgomery product of two
/// values below `2p < 2^51` is `(x y + m p) / 2^52 < (4 p^2 + 2^52 p) / 2^52 < 2p`.
#[inline(always)]
unsafe fn product_v<V: L8>(a: &mut [u64], b: Option<&[u64]>, p: Prime52) {
    let n = a.len();
    let k = p.scale(n);
    let full = n / 8 * 8;
    // SAFETY: the caller permits V creation.
    let (c, kv, j) = unsafe {
        (
            C::<V>::new(p),
            (V::splat(k.0), V::splat(k.1)),
            V::splat(p.j),
        )
    };
    for i in (0..full).step_by(8) {
        // SAFETY: i + 8 <= n for both slices.
        unsafe {
            let x = V::load(a.as_ptr().add(i));
            let y = b.map_or(x, |b| V::load(b.as_ptr().add(i)));
            mont(mulw(x, kv, &c), y, j, &c).store(a.as_mut_ptr().add(i));
        }
    }
    for i in full..n {
        let y = b.map_or(a[i], |b| b[i]);
        a[i] = mont_s(p.mulw(a[i], k), y, p);
    }
}

/// Safety: `V` values may be created.
#[inline(always)]
unsafe fn pointwise_v<V: L8>(a: &mut [u64], b: &[u64], p: Prime52) {
    assert_eq!(a.len(), b.len());
    // SAFETY: the caller's.
    unsafe { product_v::<V>(a, Some(b), p) }
}

/// Safety: `V` values may be created.
#[inline(always)]
unsafe fn square_v<V: L8>(a: &mut [u64], p: Prime52) {
    // SAFETY: the caller's.
    unsafe { product_v::<V>(a, None, p) }
}

/// The 32-bit halves of `limbs`, least significant first, then zeros, each `w(i, x)`.
///
/// Safety: `V` values may be created.
#[inline(always)]
unsafe fn load_with<V: L8>(
    dst: &mut [u64],
    limbs: &[u64],
    mut vf: impl FnMut(usize, V) -> V,
    mut sf: impl FnMut(usize, u64) -> u64,
) {
    let m = 2 * limbs.len();
    assert!(dst.len() >= m);
    let full = limbs.len() / 4 * 4;
    let src = limbs.as_ptr().cast::<u32>();
    for i in (0..2 * full).step_by(8) {
        // SAFETY: 8 u32 halves of 4 limbs are readable, 8 lanes of dst writable.
        unsafe { vf(i, V::load_u32(src.add(i))).store(dst.as_mut_ptr().add(i)) }
    }
    for i in 2 * full..m {
        dst[i] = sf(i, (limbs[i / 2] >> (32 * (i % 2))) & 0xFFFF_FFFF);
    }
    dst[m..].fill(0);
}

/// Safety: `V` values may be created.
#[inline(always)]
unsafe fn load_v<V: L8>(dst: &mut [u64], limbs: &[u64]) {
    // SAFETY: the caller's.
    unsafe { load_with::<V>(dst, limbs, |_, x| x, |_, x| x) }
}

/// Safety: `V` values may be created.
#[inline(always)]
unsafe fn load_twist_v<V: L8>(dst: &mut [u64], limbs: &[u64], tw: &Tw52) {
    let m = dst.len();
    assert!(2 * m <= tw.len() && m.is_power_of_two());
    // SAFETY: the caller permits V creation.
    let c = unsafe { C::<V>::new(tw.prime()) };
    let w = |i: usize| {
        // SAFETY: i + 8 <= 2 * limbs.len() <= m, so m + i + 8 <= 2 m <= tw.w().len().
        unsafe {
            (
                V::load(tw.w().as_ptr().add(m + i)),
                V::load(tw.wp().as_ptr().add(m + i)),
            )
        }
    };
    let p = tw.prime();
    // SAFETY: the caller's.
    unsafe {
        load_with::<V>(
            dst,
            limbs,
            |i, x| mulw(x, w(i), &c),
            |i, x| p.mulw(x, (tw.w()[m + i], tw.wp()[m + i])),
        )
    }
}

/// Adds or subtracts: `[0, 2p)` in and out.
///
/// Safety: `V` values may be created.
#[inline(always)]
unsafe fn load_add_v<V: L8>(dst: &mut [u64], limbs: &[u64], p: Prime52, sub: bool) {
    let m = 2 * limbs.len();
    assert!(dst.len() >= m);
    // SAFETY: the caller permits V creation.
    let c = unsafe { C::<V>::new(p) };
    let full = limbs.len() / 4 * 4;
    let src = limbs.as_ptr().cast::<u32>();
    for i in (0..2 * full).step_by(8) {
        // SAFETY: 4 limbs readable, 8 lanes of dst readable and writable.
        unsafe {
            let (d, x) = (V::load(dst.as_ptr().add(i)), V::load_u32(src.add(i)));
            // `x < 2^32 < 2p`.
            let s = if sub { d.add(c.p2.sub(x)) } else { d.add(x) };
            vcsub(s, c.p2).store(dst.as_mut_ptr().add(i));
        }
    }
    for i in 2 * full..m {
        let x = (limbs[i / 2] >> (32 * (i % 2))) & 0xFFFF_FFFF;
        let s = if sub {
            dst[i] + 2 * p.p - x
        } else {
            dst[i] + x
        };
        dst[i] = csub(s, 2 * p.p);
    }
}

/// `a[i] w_2m^i`, `m = a.len()`: any input below `2^52`, out `[0, 2p)`.
///
/// Safety: `V` values may be created.
#[inline(always)]
unsafe fn twist_v<V: L8>(a: &mut [u64], tw: &Tw52) {
    let m = a.len();
    assert!(2 * m <= tw.len());
    let p = tw.prime();
    // SAFETY: the caller permits V creation.
    let c = unsafe { C::<V>::new(p) };
    let full = m / 8 * 8;
    for i in (0..full).step_by(8) {
        // SAFETY: i + 8 <= m; twiddles m + i + 8 <= 2 m.
        unsafe {
            let w = (
                V::load(tw.w().as_ptr().add(m + i)),
                V::load(tw.wp().as_ptr().add(m + i)),
            );
            mulw(V::load(a.as_ptr().add(i)), w, &c).store(a.as_mut_ptr().add(i));
        }
    }
    for (i, x) in a.iter_mut().enumerate().skip(full) {
        *x = p.mulw(*x, (tw.w()[m + i], tw.wp()[m + i]));
    }
}

/// `a[i] w_2m^-(start + i)`: input below `2^52` (`[0, 4p)` at `start + i = 0`), out
/// `[0, 4p)`. Uses `w^-k = -w^(m - k)`.
///
/// Safety: `V` values may be created.
#[inline(always)]
unsafe fn untwist_v<V: L8>(a: &mut [u64], m: usize, start: usize, tw: &Tw52) {
    let len = a.len();
    assert!(start + len <= m && 2 * m <= tw.len());
    let p = tw.prime();
    // SAFETY: the caller permits V creation.
    let (c, rev, m1) = unsafe {
        (
            C::<V>::new(p),
            V::from_array(REVERSE),
            (V::splat(p.m1.0), V::splat(p.m1.1)),
        )
    };
    let full = len / 8 * 8;
    for i in (0..full).step_by(8) {
        // SAFETY: start + i + 8 <= m; neg_inverse reads at most 2 m + 1 entries.
        unsafe {
            let w = neg_inverse(tw, m, start + i, rev, m1);
            let x = V::load(a.as_ptr().add(i));
            // `-(x (-w^-k))`, below 2p + 1.
            c.p2.sub(mulw(x, w, &c)).store(a.as_mut_ptr().add(i));
        }
    }
    for (i, x) in a.iter_mut().enumerate().skip(full) {
        let k = start + i;
        let w = if k == 0 {
            p.m1
        } else {
            (tw.w()[2 * m - k], tw.wp()[2 * m - k])
        };
        *x = 2 * p.p - p.mulw(*x, w);
    }
}

/// `s mod p` in `[0, 2p)`, for `s < 2^63`: `s = h 2^52 + l` and `2^52 = 4 (2^50 - p) mod p`.
///
/// Safety: `V` values may be created.
#[inline(always)]
unsafe fn narrow_v<V: L8>(dst: &mut [u64], s: &[u64], p: Prime52) {
    let n = dst.len().min(s.len());
    // `h < 2^11`, `c = 2^52 mod p < 2^32`, so `l + h c < 2^52 + 2^43 < 5p`.
    let r52 = (1u64 << 52) % p.p;
    // SAFETY: the caller permits V creation.
    let (c, cv) = unsafe { (C::<V>::new(p), V::splat(r52)) };
    let full = n / 8 * 8;
    for i in (0..full).step_by(8) {
        // SAFETY: i + 8 <= n for both slices.
        unsafe {
            let x = V::load(s.as_ptr().add(i));
            let y = x.and(c.mask).madd52lo(x.shr::<52>(), cv);
            vcsub(vcsub(y, c.p4), c.p2).store(dst.as_mut_ptr().add(i));
        }
    }
    for i in full..n {
        let x = s[i];
        let y = (x & MASK) + (x >> 52) * r52;
        dst[i] = csub(csub(y, 4 * p.p), 2 * p.p);
    }
}

/// Subtracts `x^start src` from `dst` modulo `x^d + 1` (`neg`) or `x^d - 1`, `d =
/// dst.len()`: `[0, 4p)` in and out.
///
/// Safety: `V` values may be created.
#[inline(always)]
unsafe fn fold_sub_v<V: L8>(dst: &mut [u64], src: &[u64], start: usize, neg: bool, p: Prime52) {
    let d = dst.len();
    // SAFETY: the caller permits V creation.
    let c = unsafe { C::<V>::new(p) };
    let (mut pos, mut src) = (start, src);
    while !src.is_empty() {
        let at = pos % d;
        let (run, rest) = src.split_at((d - at).min(src.len()));
        let add = neg && (pos / d) % 2 == 1;
        let x = &mut dst[at..at + run.len()];
        let full = run.len() / 8 * 8;
        for i in (0..full).step_by(8) {
            // SAFETY: i + 8 <= run.len() = x.len().
            unsafe {
                let (a, b) = (V::load(x.as_ptr().add(i)), V::load(run.as_ptr().add(i)));
                let s = if add { a.add(b) } else { a.add(c.p4.sub(b)) };
                vcsub(s, c.p4).store(x.as_mut_ptr().add(i));
            }
        }
        for i in full..run.len() {
            let s = if add {
                x[i] + run[i]
            } else {
                x[i] + 4 * p.p - run[i]
            };
            x[i] = csub(s, 4 * p.p);
        }
        (pos, src) = (pos + run.len(), rest);
    }
}

/// `(lo, hi) = ((hi + lo) / 2, (hi - lo) / 2)`: `[0, 4p)` in, `[0, 2p)` out.
///
/// Safety: `V` values may be created.
#[inline(always)]
unsafe fn halves_v<V: L8>(lo: &mut [u64], hi: &mut [u64], p: Prime52) {
    let n = lo.len();
    assert_eq!(n, hi.len());
    // SAFETY: the caller permits V creation.
    let (c, h) = unsafe { (C::<V>::new(p), (V::splat(p.half.0), V::splat(p.half.1))) };
    let full = n / 8 * 8;
    for i in (0..full).step_by(8) {
        // SAFETY: i + 8 <= n for both slices.
        unsafe {
            let (a, b) = (V::load(lo.as_ptr().add(i)), V::load(hi.as_ptr().add(i)));
            let s = vcsub(b.add(a), c.p4);
            let d = vcsub(b.add(c.p4.sub(a)), c.p4);
            mulw(s, h, &c).store(lo.as_mut_ptr().add(i));
            mulw(d, h, &c).store(hi.as_mut_ptr().add(i));
        }
    }
    for i in full..n {
        let s = csub(hi[i] + lo[i], 4 * p.p);
        let d = csub(hi[i] + 4 * p.p - lo[i], 4 * p.p);
        (lo[i], hi[i]) = (p.mulw(s, p.half), p.mulw(d, p.half));
    }
}

/// `1 / p1 mod p2` and its companion.
const GARNER: (u64, u64) = {
    let (p1, p2) = (PRIMES_AND_ROOTS[0].0, PRIMES_AND_ROOTS[1].0);
    let c = powmod(p1 % p2, p2 - 2, p2);
    (c, shoup(c, p2))
};

/// `r1` to `[0, p1)` and `r2` to `(r2 - r1) / p1 mod p2` in `[0, p2)`, from `[0, 4p)`:
/// with `r1 < p1 < 2 p2` and `r2 < 2 p2`, `r2 + 2 p2 - r1` is in `(0, 4 p2)`.
///
/// Safety: `V` values may be created.
#[inline(always)]
unsafe fn garner_v<V: L8>(r1: &mut [u64], r2: &mut [u64]) {
    let n = r1.len();
    assert_eq!(n, r2.len());
    let [p1, p2] = PRIMES;
    // SAFETY: the caller permits V creation.
    let (c1, c2, g) = unsafe {
        (
            C::<V>::new(p1),
            C::<V>::new(p2),
            (V::splat(GARNER.0), V::splat(GARNER.1)),
        )
    };
    let full = n / 8 * 8;
    for i in (0..full).step_by(8) {
        // SAFETY: i + 8 <= n for both slices.
        unsafe {
            let x = vcsub(vcsub(V::load(r1.as_ptr().add(i)), c1.p2), c1.p);
            let y = vcsub(V::load(r2.as_ptr().add(i)), c2.p2);
            let v = vcsub(mulw(y.add(c2.p2).sub(x), g, &c2), c2.p);
            x.store(r1.as_mut_ptr().add(i));
            v.store(r2.as_mut_ptr().add(i));
        }
    }
    for i in full..n {
        let x = csub(csub(r1[i], 2 * p1.p), p1.p);
        let y = csub(r2[i], 2 * p2.p);
        r1[i] = x;
        r2[i] = csub(p2.mulw(y + 2 * p2.p - x, GARNER), p2.p);
    }
}

/// [`Tw52`]'s roots: the top stage by eight chains 32 apart (Shoup steps), companions by
/// a Barrett estimate, the lower stages copied from every other root of the stage above.
///
/// Companion: with `r = floor(2^101 / p)`, `q = floor(w r / 2^49)` is at most 2 below
/// `floor(w 2^52 / p)`, since `w (2^101 / p - r) / 2^49 < w / 2^49 < 2`; then
/// `w 2^52 - q p < 3p < 2^52` is `-q p mod 2^52`, and two corrections finish.
///
/// Safety: `V` values may be created.
#[inline(always)]
unsafe fn roots_v<V: L8>(w: &mut [u64], wp: &mut [u64], p: Prime52, root: u64) {
    let n = w.len() - 8;
    assert!(n.is_power_of_two() && wp.len() == w.len());
    let h = n / 2;
    // SAFETY: the caller permits V creation.
    let c = unsafe { C::<[V; 4]>::new(p) };
    let lead = h.min(32);
    let mut x = 1;
    for j in 0..lead {
        w[h + j] = x;
        x = mulmod(x, root, p.p);
    }
    if h > lead {
        // SAFETY: the caller permits V creation.
        let s = unsafe { ([V::splat(x); 4], [V::splat(shoup(x, p.p)); 4]) };
        for j in (h + lead..n).step_by(32) {
            // SAFETY: j - 32 >= h and j + 32 <= n.
            unsafe {
                let prev = <[V; 4]>::load(w.as_ptr().add(j - 32));
                vcsub(mulw(prev, s, &c), c.p).store(w.as_mut_ptr().add(j));
            }
        }
    }
    // SAFETY: the caller permits V creation.
    let (c1, r) = unsafe { (C::<V>::new(p), V::splat(p.r101)) };
    let full = h / 8 * 8;
    for j in (h..h + full).step_by(8) {
        // SAFETY: j + 8 <= n.
        unsafe {
            let x = V::load(w.as_ptr().add(j));
            let q = c1.zero.madd52hi(x, r).shl::<3>();
            let q = q.or(c1.zero.madd52lo(x, r).shr::<49>());
            let rem = c1.zero.madd52lo(q, c1.negp).and(c1.mask);
            let q = q.inc_ge(rem, c1.p);
            let rem = vcsub(rem, c1.p);
            q.inc_ge(rem, c1.p).store(wp.as_mut_ptr().add(j));
        }
    }
    for j in h + full..n {
        wp[j] = shoup(w[j], p.p);
    }
    // SAFETY: the caller permits V creation.
    let even = unsafe { V::from_array(core::array::from_fn(|i| 2 * i as u64)) };
    let mut g = h / 2;
    while g >= 1 {
        for t in [&mut *w, &mut *wp] {
            if g >= 8 {
                for j in (0..g).step_by(8) {
                    // SAFETY: 2 g + 2 j + 16 <= 4 g; g + j + 8 <= 2 g.
                    unsafe {
                        let a = V::load(t.as_ptr().add(2 * g + 2 * j));
                        let b = V::load(t.as_ptr().add(2 * g + 2 * j + 8));
                        a.permute2(even, b).store(t.as_mut_ptr().add(g + j));
                    }
                }
            } else {
                for j in 0..g {
                    t[g + j] = t[2 * g + 2 * j];
                }
            }
        }
        g /= 2;
    }
}

/// `out = sum_i (r1[i] + p1 v[i]) 2^(32 i)`, returning the carry out of the top limb.
/// The inputs are [`garner_v`]'s output, at least `2 out.len()` long.
pub(crate) fn carry_out(out: &mut [u64], r1: &[u64], v: &[u64]) -> u128 {
    let p1 = PRIMES[0].p as u128;
    let n = 2 * out.len();
    let (r1, v) = (r1[..n].as_chunks::<2>().0, v[..n].as_chunks::<2>().0);
    let mut c = 0u128;
    for ((o, x), y) in out.iter_mut().zip(r1).zip(v) {
        // Coefficients are below 2^87, so `c` stays below 2^120.
        c += x[0] as u128 + p1 * y[0] as u128;
        c += (x[1] as u128 + p1 * y[1] as u128) << 32;
        *o = c as u64;
        c >>= 64;
    }
    c
}

#[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
mod z {
    use super::{Kernels52, L8};
    use core::arch::x86_64::*;

    // Safety (each wrapper): the CPU has AVX-512F and IFMA.
    macro_rules! wrap {
        ($($name:ident($($arg:ident: $ty:ty),*);)*) => {
            $(
                #[target_feature(enable = "avx512f,avx512ifma")]
                unsafe fn $name($($arg: $ty),*) {
                    // SAFETY: this function requires the features, so Z values may be created.
                    unsafe { super::$name::<Z>($($arg),*) }
                }
            )*
        };
    }

    wrap! {
        roots_v(w: &mut [u64], wp: &mut [u64], p: super::Prime52, r: u64);
        load_v(d: &mut [u64], l: &[u64]);
        load_twist_v(d: &mut [u64], l: &[u64], t: &super::Tw52);
        load_add_v(d: &mut [u64], l: &[u64], p: super::Prime52, s: bool);
        twist_v(a: &mut [u64], t: &super::Tw52);
        untwist_v(a: &mut [u64], m: usize, s: usize, t: &super::Tw52);
        narrow_v(d: &mut [u64], s: &[u64], p: super::Prime52);
        forward_v(a: &mut [u64], t: &super::Tw52);
        inverse_v(a: &mut [u64], t: &super::Tw52);
        pointwise_v(a: &mut [u64], b: &[u64], p: super::Prime52);
        square_v(a: &mut [u64], p: super::Prime52);
        fold_sub_v(d: &mut [u64], s: &[u64], st: usize, n: bool, p: super::Prime52);
        halves_v(l: &mut [u64], h: &mut [u64], p: super::Prime52);
        garner_v(a: &mut [u64], b: &mut [u64]);
    }

    /// Requires AVX-512F and IFMA.
    #[allow(
        clippy::undocumented_unsafe_blocks,
        reason = "One check covers every wrapper: `Kernels52::best` tested the CPU features"
    )]
    pub(super) fn kernels() -> Kernels52 {
        Kernels52 {
            roots: |w, wp, p, r| unsafe { roots_v(w, wp, p, r) },
            load: |d, l| unsafe { load_v(d, l) },
            load_twist: |d, l, t| unsafe { load_twist_v(d, l, t) },
            load_add: |d, l, p, s| unsafe { load_add_v(d, l, p, s) },
            twist: |a, t| unsafe { twist_v(a, t) },
            untwist: |a, m, s, t| unsafe { untwist_v(a, m, s, t) },
            narrow: |d, s, p| unsafe { narrow_v(d, s, p) },
            forward: |a, t| unsafe { forward_v(a, t) },
            inverse: |a, t| unsafe { inverse_v(a, t) },
            pointwise: |a, b, p| unsafe { pointwise_v(a, b, p) },
            square: |a, p| unsafe { square_v(a, p) },
            fold_sub: |d, s, st, n, p| unsafe { fold_sub_v(d, s, st, n, p) },
            halves: |l, h, p| unsafe { halves_v(l, h, p) },
            garner: |a, b| unsafe { garner_v(a, b) },
        }
    }

    #[derive(Clone, Copy)]
    struct Z(__m512i);

    impl L8 for Z {
        #[inline(always)]
        unsafe fn splat(x: u64) -> Self {
            // SAFETY: the caller guarantees the CPU features.
            Z(unsafe { _mm512_set1_epi64(x as i64) })
        }
        #[inline(always)]
        unsafe fn from_array(x: [u64; 8]) -> Self {
            // SAFETY: both types hold 512 initialized bits; all bit patterns are valid.
            Z(unsafe { core::mem::transmute::<[u64; 8], __m512i>(x) })
        }
        #[inline(always)]
        unsafe fn load(p: *const u64) -> Self {
            // SAFETY: the caller provides 8 readable lanes and the features; unaligned.
            Z(unsafe { _mm512_loadu_epi64(p.cast()) })
        }
        #[inline(always)]
        unsafe fn load_u32(p: *const u32) -> Self {
            // SAFETY: the caller provides 8 readable u32 and the features; unaligned.
            Z(unsafe { _mm512_cvtepu32_epi64(_mm256_loadu_si256(p.cast())) })
        }
        #[inline(always)]
        unsafe fn store(self, p: *mut u64) {
            // SAFETY: Z proves the features; the caller provides 8 writable lanes.
            unsafe { _mm512_storeu_epi64(p.cast(), self.0) }
        }
        #[inline(always)]
        fn add(self, b: Self) -> Self {
            // SAFETY: holding Z proves the CPU features.
            Z(unsafe { _mm512_add_epi64(self.0, b.0) })
        }
        #[inline(always)]
        fn sub(self, b: Self) -> Self {
            // SAFETY: holding Z proves the CPU features.
            Z(unsafe { _mm512_sub_epi64(self.0, b.0) })
        }
        #[inline(always)]
        fn min(self, b: Self) -> Self {
            // SAFETY: holding Z proves the CPU features.
            Z(unsafe { _mm512_min_epu64(self.0, b.0) })
        }
        #[inline(always)]
        fn and(self, b: Self) -> Self {
            // SAFETY: holding Z proves the CPU features.
            Z(unsafe { _mm512_and_si512(self.0, b.0) })
        }
        #[inline(always)]
        fn or(self, b: Self) -> Self {
            // SAFETY: holding Z proves the CPU features.
            Z(unsafe { _mm512_or_si512(self.0, b.0) })
        }
        #[inline(always)]
        fn shl<const N: u32>(self) -> Self {
            // SAFETY: holding Z proves the CPU features.
            Z(unsafe { _mm512_slli_epi64::<N>(self.0) })
        }
        #[inline(always)]
        fn shr<const N: u32>(self) -> Self {
            // SAFETY: holding Z proves the CPU features.
            Z(unsafe { _mm512_srli_epi64::<N>(self.0) })
        }
        #[inline(always)]
        fn madd52lo(self, x: Self, y: Self) -> Self {
            // SAFETY: holding Z proves the CPU features.
            Z(unsafe { _mm512_madd52lo_epu64(self.0, x.0, y.0) })
        }
        #[inline(always)]
        fn madd52hi(self, x: Self, y: Self) -> Self {
            // SAFETY: holding Z proves the CPU features.
            Z(unsafe { _mm512_madd52hi_epu64(self.0, x.0, y.0) })
        }
        #[inline(always)]
        fn permute(self, idx: Self) -> Self {
            // SAFETY: holding Z proves the CPU features.
            Z(unsafe { _mm512_permutexvar_epi64(idx.0, self.0) })
        }
        #[inline(always)]
        fn permute2(self, idx: Self, b: Self) -> Self {
            // SAFETY: holding Z proves the CPU features.
            Z(unsafe { _mm512_permutex2var_epi64(self.0, idx.0, b.0) })
        }
        #[inline(always)]
        fn set0(self, b: Self) -> Self {
            // SAFETY: holding Z proves the CPU features.
            Z(unsafe { _mm512_mask_mov_epi64(self.0, 1, b.0) })
        }
        #[inline(always)]
        fn inc_nonzero(self, x: Self) -> Self {
            // SAFETY: holding Z proves the CPU features.
            Z(unsafe {
                let k = _mm512_test_epi64_mask(x.0, x.0);
                _mm512_mask_add_epi64(self.0, k, self.0, _mm512_set1_epi64(1))
            })
        }
        #[inline(always)]
        fn inc_ge(self, r: Self, m: Self) -> Self {
            // SAFETY: holding Z proves the CPU features.
            Z(unsafe {
                let k = _mm512_cmpge_epu64_mask(r.0, m.0);
                _mm512_mask_add_epi64(self.0, k, self.0, _mm512_set1_epi64(1))
            })
        }
    }

    #[cfg(test)]
    #[test]
    fn ifma_matches_emulation() {
        if !crate::cpu::ifma() {
            assert!(
                std::env::var_os("BASE58_REQUIRE_IFMA").is_none(),
                "no AVX-512 IFMA"
            );
            eprintln!("skipped: no AVX-512 IFMA");
            return;
        }
        super::tests::kernels_match(&kernels(), &super::tests::emu());
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use alloc::vec::Vec;
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    /// The lanes of [`L8`] in an array, each operation as the instruction defines it.
    #[derive(Clone, Copy)]
    pub(crate) struct Emu([u64; 8]);

    impl Emu {
        fn map(self, b: Self, f: impl Fn(u64, u64) -> u64) -> Self {
            Emu(core::array::from_fn(|i| f(self.0[i], b.0[i])))
        }
    }

    impl L8 for Emu {
        unsafe fn splat(x: u64) -> Self {
            Emu([x; 8])
        }
        unsafe fn from_array(x: [u64; 8]) -> Self {
            Emu(x)
        }
        unsafe fn load(p: *const u64) -> Self {
            // SAFETY: the caller provides 8 readable lanes.
            Emu(core::array::from_fn(|i| unsafe {
                p.add(i).read_unaligned()
            }))
        }
        unsafe fn load_u32(p: *const u32) -> Self {
            // SAFETY: the caller provides 8 readable u32.
            Emu(core::array::from_fn(
                |i| unsafe { p.add(i).read_unaligned() } as u64,
            ))
        }
        unsafe fn store(self, p: *mut u64) {
            for (i, &x) in self.0.iter().enumerate() {
                // SAFETY: the caller provides 8 writable lanes.
                unsafe { p.add(i).write_unaligned(x) }
            }
        }
        fn add(self, b: Self) -> Self {
            self.map(b, u64::wrapping_add)
        }
        fn sub(self, b: Self) -> Self {
            self.map(b, u64::wrapping_sub)
        }
        fn min(self, b: Self) -> Self {
            self.map(b, u64::min)
        }
        fn and(self, b: Self) -> Self {
            self.map(b, |x, y| x & y)
        }
        fn or(self, b: Self) -> Self {
            self.map(b, |x, y| x | y)
        }
        fn shl<const N: u32>(self) -> Self {
            Emu(self.0.map(|x| x << N))
        }
        fn shr<const N: u32>(self) -> Self {
            Emu(self.0.map(|x| x >> N))
        }
        fn madd52lo(self, x: Self, y: Self) -> Self {
            let p = x.map(y, |x, y| {
                ((x & MASK) as u128 * (y & MASK) as u128) as u64 & MASK
            });
            self.add(p)
        }
        fn madd52hi(self, x: Self, y: Self) -> Self {
            let p = x.map(y, |x, y| {
                (((x & MASK) as u128 * (y & MASK) as u128) >> 52) as u64
            });
            self.add(p)
        }
        fn permute(self, idx: Self) -> Self {
            Emu(core::array::from_fn(|i| self.0[idx.0[i] as usize % 8]))
        }
        fn permute2(self, idx: Self, b: Self) -> Self {
            Emu(core::array::from_fn(|i| {
                let k = idx.0[i] as usize % 16;
                if k < 8 { self.0[k] } else { b.0[k - 8] }
            }))
        }
        fn set0(self, b: Self) -> Self {
            let mut x = self.0;
            x[0] = b.0[0];
            Emu(x)
        }
        fn inc_nonzero(self, x: Self) -> Self {
            self.map(x, |s, x| s.wrapping_add((x != 0) as u64))
        }
        fn inc_ge(self, r: Self, m: Self) -> Self {
            let ge = r.map(m, |r, m| (r >= m) as u64);
            self.add(ge)
        }
    }

    /// The kernels on [`Emu`].
    #[allow(
        clippy::undocumented_unsafe_blocks,
        reason = "Emu values may always be created"
    )]
    pub(crate) fn emu() -> Kernels52 {
        kernels!(Emu)
    }

    fn modp(x: u64, p: Prime52) -> u64 {
        x % p.p
    }

    #[test]
    fn primes_and_roots() {
        let mut product = 1u128;
        for (p, root) in PRIMES_AND_ROOTS {
            assert!(p < 1 << 50 && (p - 1) % (1 << 26) == 0);
            // Miller-Rabin with the first 12 primes is exact below 3.3e24.
            let d = (p - 1) >> (p - 1).trailing_zeros();
            for a in [2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
                let mut x = powmod(a, d, p);
                let mut ok = x == 1 || x == p - 1;
                let mut e = d;
                while !ok && e < p - 1 {
                    x = mulmod(x, x, p);
                    e *= 2;
                    ok = x == p - 1;
                }
                assert!(ok, "{p} is not prime");
            }
            assert_eq!(powmod(root, 1 << (MAX_LOG - 1), p), p - 1);
            product *= p as u128;
        }
        assert!(product > 1u128 << 99);
        for p in PRIMES {
            assert_eq!(p.j.wrapping_mul(p.p) & MASK, MASK);
            assert_eq!(p.r101 as u128, (1u128 << 101) / p.p as u128);
            assert!(p.r101 < 1 << 52);
        }
        assert_eq!(mulmod(GARNER.0, PRIMES[0].p, PRIMES[1].p), 1);
    }

    #[test]
    fn roots_and_companions() {
        let k = emu();
        for log in [1, 2, 3, 5, 6, 9, 12] {
            let t = Tables52::new(log, &k);
            let n = 1usize << log;
            for (j, tw) in t.tw.iter().enumerate() {
                let p = PRIMES_AND_ROOTS[j].0;
                let w = (log..MAX_LOG).fold(PRIMES_AND_ROOTS[j].1, |w, _| mulmod(w, w, p));
                let mut h = n / 2;
                let mut s = w;
                while h >= 1 {
                    for i in 0..h {
                        assert_eq!(
                            tw.w()[h + i],
                            powmod(s, i as u64, p),
                            "log {log} h {h} i {i}"
                        );
                        assert_eq!(tw.wp()[h + i], shoup(tw.w()[h + i], p));
                    }
                    s = mulmod(s, s, p);
                    h /= 2;
                }
            }
        }
    }

    fn rand_residues(rng: &mut Xoshiro256PlusPlus, n: usize, bound: u64) -> Vec<u64> {
        let mut v: Vec<u64> = (0..n).map(|_| rng.next_u64() % bound).collect();
        for (i, x) in v.iter_mut().enumerate().take(n.min(3)) {
            *x = [0, bound - 1, bound / 2][i];
        }
        v
    }

    fn bitrev(i: usize, log: u32) -> usize {
        if log == 0 {
            0
        } else {
            i.reverse_bits() >> (usize::BITS - log)
        }
    }

    /// Forward is a DFT in bit-reversed order; inverse undoes it times `n`; ranges hold.
    #[test]
    fn transforms_are_dfts() {
        let k = emu();
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(52);
        let t = Tables52::new(8, &k);
        for tw in &t.tw {
            let p = tw.prime();
            for log in 1..=8 {
                let n = 1usize << log;
                let x = rand_residues(&mut rng, n, 2 * p.p);
                let mut a = x.clone();
                (k.forward)(&mut a, tw);
                let w = if n == 2 { p.p - 1 } else { tw.w()[n / 2 + 1] };
                for (j, &got) in a.iter().enumerate() {
                    assert!(got < 2 * p.p);
                    let wk = powmod(w, bitrev(j, log) as u64, p.p);
                    let want = x
                        .iter()
                        .rev()
                        .fold(0, |acc, &v| (mulmod(acc, wk, p.p) + v) % p.p);
                    assert_eq!(modp(got, p), want, "n {n}, j {j}");
                }
                (k.inverse)(&mut a, tw);
                for (&got, &x) in a.iter().zip(&x) {
                    assert!(got < 4 * p.p);
                    assert_eq!(modp(got, p), mulmod(x % p.p, n as u64, p.p), "n {n}");
                }
            }
        }
    }

    /// Every kernel of `a` gives exactly what `b`'s gives.
    pub(crate) fn kernels_match(a: &Kernels52, b: &Kernels52) {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(53);
        let (ta, tb) = (Tables52::new(14, a), Tables52::new(14, b));
        for (twa, twb) in ta.tw.iter().zip(&tb.tw) {
            assert_eq!(twa.roots, twb.roots);
            let p = twa.prime();
            for log in [1, 3, 5, 6, 7, 10, 14] {
                let n = 1usize << log;
                let x = rand_residues(&mut rng, n, 2 * p.p);
                let y = rand_residues(&mut rng, n, 2 * p.p);
                let x4 = rand_residues(&mut rng, n, 4 * p.p);
                let limbs: Vec<u64> = (0..n / 2 - n / 8).map(|_| rng.next_u64()).collect();
                let sums: Vec<u64> = (0..n).map(|_| rng.next_u64() >> 1).collect();
                let run = |k: &Kernels52, tw: &Tw52| -> Vec<Vec<u64>> {
                    let mut o = vec![];
                    let mut a = x.clone();
                    (k.forward)(&mut a, tw);
                    o.push(a.clone());
                    (k.inverse)(&mut a, tw);
                    o.push(a);
                    let mut a = x.clone();
                    (k.pointwise)(&mut a, &y, p);
                    o.push(a);
                    let mut a = y.clone();
                    (k.square)(&mut a, p);
                    o.push(a);
                    let mut a = vec![7; n];
                    (k.load)(&mut a, &limbs);
                    o.push(a.clone());
                    (k.load_add)(&mut a, &limbs[..limbs.len() / 2], p, true);
                    (k.load_add)(&mut a, &limbs[1..], p, false);
                    o.push(a);
                    if n <= tw.len() / 2 {
                        let mut a = vec![7; n];
                        (k.load_twist)(&mut a, &limbs, tw);
                        o.push(a);
                        let mut a = x4.clone();
                        (k.twist)(&mut a, tw);
                        o.push(a);
                        let mut a = x4.clone();
                        (k.untwist)(&mut a[..n - n / 4], n, n / 8, tw);
                        (k.untwist)(&mut a, n, 0, tw);
                        o.push(a);
                    }
                    let mut a = vec![0; n];
                    (k.narrow)(&mut a, &sums, p);
                    o.push(a);
                    let mut a = x4.clone();
                    (k.fold_sub)(&mut a[..n / 2], &x4[..n / 2 + n / 4], n / 8, true, p);
                    (k.fold_sub)(&mut a, &x4[n / 4..], 3, false, p);
                    o.push(a);
                    let (mut lo, mut hi) = (x4.clone(), y.clone());
                    (k.halves)(&mut lo, &mut hi, p);
                    o.push(lo);
                    o.push(hi);
                    o
                };
                assert!(run(a, twa) == run(b, twb), "log {log}");
            }
        }
        for n in [0, 4, 12, 16, 20, 64, 100] {
            let r1 = rand_residues(&mut rng, n.max(3), 4 * PRIMES[0].p)[..n].to_vec();
            let r2 = rand_residues(&mut rng, n.max(3), 4 * PRIMES[1].p)[..n].to_vec();
            let (mut a1, mut a2, mut b1, mut b2) = (r1.clone(), r2.clone(), r1, r2);
            (a.garner)(&mut a1, &mut a2);
            (b.garner)(&mut b1, &mut b2);
            assert!(a1 == b1 && a2 == b2, "garner, n {n}");
        }
    }

    /// The helper kernels compute what their contracts state, mod p and in range.
    #[test]
    fn helpers_are_exact() {
        let k = emu();
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(54);
        let t = Tables52::new(10, &k);
        for tw in &t.tw {
            let p = tw.prime();
            let q = p.p;
            for n in [8usize, 64, 256] {
                let m = n;
                let x4 = rand_residues(&mut rng, n, 4 * q);
                let wm = |i: usize| tw.w()[m + i];
                // twist / untwist
                let mut a = x4.clone();
                (k.twist)(&mut a, tw);
                for i in 0..n {
                    assert!(a[i] < 2 * q);
                    assert_eq!(a[i] % q, mulmod(x4[i] % q, wm(i), q));
                }
                let mut b = a.clone();
                (k.untwist)(&mut b, m, 0, tw);
                for i in 0..n {
                    assert!(b[i] < 4 * q);
                    assert_eq!(b[i] % q, x4[i] % q, "untwist {i}");
                }
                // pointwise, square
                let (x, y) = (
                    rand_residues(&mut rng, n, 2 * q),
                    rand_residues(&mut rng, n, 2 * q),
                );
                let ninv = powmod(n as u64, q - 2, q);
                let mut a = x.clone();
                (k.pointwise)(&mut a, &y, p);
                let mut s = x.clone();
                (k.square)(&mut s, p);
                for i in 0..n {
                    assert!(a[i] < 2 * q && s[i] < 2 * q);
                    let want = mulmod(mulmod(x[i] % q, y[i] % q, q), ninv, q);
                    assert_eq!(a[i] % q, want);
                    assert_eq!(s[i] % q, mulmod(mulmod(x[i] % q, x[i] % q, q), ninv, q));
                }
                // halves
                let (mut lo, mut hi) = (x4.clone(), rand_residues(&mut rng, n, 4 * q));
                let h0 = hi.clone();
                (k.halves)(&mut lo, &mut hi, p);
                let inv2 = q.div_ceil(2);
                for i in 0..n {
                    assert!(lo[i] < 2 * q && hi[i] < 2 * q);
                    assert_eq!(lo[i] % q, mulmod((h0[i] + x4[i]) % q, inv2, q));
                    assert_eq!(hi[i] % q, mulmod((h0[i] + 4 * q - x4[i]) % q, inv2, q));
                }
                // narrow, with the largest sums
                let mut s: Vec<u64> = (0..n).map(|_| rng.next_u64() >> 1).collect();
                s[0] = (1 << 63) - 1;
                let mut a = vec![0; n];
                (k.narrow)(&mut a, &s, p);
                for i in 0..n {
                    assert!(a[i] < 2 * q);
                    assert_eq!(a[i] % q, s[i] % q);
                }
                // load_add
                let limbs: Vec<u64> = (0..n / 2).map(|_| rng.next_u64()).collect();
                let x2 = rand_residues(&mut rng, n, 2 * q);
                let mut a = x2.clone();
                (k.load_add)(&mut a, &limbs, p, true);
                for i in 0..n {
                    let c = (limbs[i / 2] >> (32 * (i % 2))) & 0xFFFF_FFFF;
                    assert!(a[i] < 2 * q);
                    assert_eq!(a[i] % q, (x2[i] % q + q - c % q) % q);
                }
            }
        }
        // garner and carry against direct reconstruction
        let [p1, p2] = PRIMES;
        let max = (1u128 << 87) - 1;
        let mut coefs: Vec<u128> = vec![0, 1, max, max - 1, 1 << 64];
        coefs.extend((0..400).map(|_| ((rng.next_u64() as u128) << 23) ^ rng.next_u64() as u128));
        coefs.truncate(coefs.len() / 2 * 2 - 2);
        coefs.extend([0, 0]);
        let res = |p: Prime52, j: usize| -> Vec<u64> {
            coefs
                .iter()
                .enumerate()
                .map(|(i, &c)| (c % p.p as u128) as u64 + p.p * ((i + j) % 4) as u64)
                .collect()
        };
        let (mut r1, mut r2) = (res(p1, 0), res(p2, 1));
        (k.garner)(&mut r1, &mut r2);
        let mut want = vec![0u64; coefs.len() / 2 + 2];
        let mut c = 0u128;
        for (i, w) in want.iter_mut().enumerate() {
            let at = |i: usize| coefs.get(i).copied().unwrap_or(0);
            c += at(2 * i) + (at(2 * i + 1) << 32);
            *w = c as u64;
            c >>= 64;
        }
        let mut got = vec![0u64; coefs.len() / 2];
        assert_eq!(carry_out(&mut got, &r1, &r2), 0);
        assert_eq!(got[..], want[..got.len()]);
    }
}
