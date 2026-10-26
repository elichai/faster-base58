//! The transform kernels with AVX-512F intrinsics, sixteen `i32` lanes per vector.
//!
//! Each computes exactly what its `portable.rs` counterpart computes, bit for bit. AVX-512
//! has no 32-bit high multiply, so [`mont`] and [`reduce`] multiply the even and the odd
//! lanes into 64-bit products (`vpmuldq`) and keep the halves they need. The stages with
//! `h <= 16` run on blocks of 32 values in two registers, regrouped between stages by
//! `vpermt2d` (see [`regroup`]); the other stages run two per pass. The forward transform
//! works on pairs of independent vectors throughout (`[V; 2]`).
//!
//! The code is written against [`V16`]: `__m512i` on x86-64, and an emulation in tests
//! that checks the arithmetic and the regrouping on any CPU, so tests build this module
//! on every target.

use super::portable;
use super::tables::{Prime, S, Twiddles};

/// [`super::Kernels`] with the transforms and pointwise products written here, the rest
/// from [`super::x86::kernels`], when this CPU has AVX-512F.
#[cfg(target_arch = "x86_64")]
pub(crate) fn kernels() -> Option<super::Kernels> {
    if !crate::cpu::avx512f() {
        return None;
    }
    // SAFETY (each call): the CPU has AVX-512F.
    Some(super::Kernels {
        forward: |a, p, t| unsafe { z::forward(a, p, t) },
        inverse: |a, p, t| unsafe { z::inverse(a, p, t) },
        pointwise: |a, b, p| unsafe { z::pointwise(a, b, p) },
        square: |a, p| unsafe { z::square(a, p) },
        ..super::x86::kernels()?
    })
}

/// Sixteen `i32` lanes, or eight `i64` ones for the 64-bit operations. For `__m512i` the
/// CPU must have AVX-512F: creating a value is `unsafe` with that contract, so holding one
/// proves it and the other methods are safe.
trait V16: Copy {
    unsafe fn splat(x: i32) -> Self;
    unsafe fn splat64(x: i64) -> Self;
    unsafe fn from_array(x: [i32; 16]) -> Self;
    /// Also requires 16 readable lanes at `p`.
    unsafe fn load(p: *const i32) -> Self;
    /// Requires 16 writable lanes at `p`.
    unsafe fn store(self, p: *mut i32);
    fn add(self, b: Self) -> Self;
    fn sub(self, b: Self) -> Self;
    /// The low 32 bits of the products.
    fn mullo(self, b: Self) -> Self;
    /// The products of the even lanes as `i64` lanes: lane `i` is `self[2 i] b[2 i]`.
    fn mul_even(self, b: Self) -> Self;
    fn add64(self, b: Self) -> Self;
    fn sub64(self, b: Self) -> Self;
    /// The `i64` lanes shifted right by `N`, arithmetically.
    fn sra64<const N: u32>(self) -> Self;
    /// The odd lanes moved to the even ones: the `i64` lanes shifted right by 32.
    fn odd(self) -> Self;
    /// The even lanes of `self` and the odd lanes of `odd`.
    fn blend(self, odd: Self) -> Self;
    /// The high halves of the `i64` lanes of `self`, then of `odd`: lane `2 i` is
    /// `self[2 i + 1]`, lane `2 i + 1` is `odd[2 i + 1]`.
    fn highs(self, odd: Self) -> Self;
    /// Lane `i` is lane `idx[i] % 32` of `self` followed by `b`.
    fn permute2(self, idx: Self, b: Self) -> Self;
}

/// `L` vectors as one, each operation applied to each in turn, so that code on them lists
/// the steps of `L` independent computations together. On Zen 5 a forward transform that
/// did one butterfly after the other took 1.33-1.55x as long with default flags as with
/// `-C target-cpu=native`: the same instructions, in another order. On pairs
/// ([`forward_v`]) the two builds are within 3% of each other, and both faster.
impl<V: V16, const L: usize> V16 for [V; L] {
    #[inline(always)]
    unsafe fn splat(x: i32) -> Self {
        [unsafe { V::splat(x) }; L]
    }
    #[inline(always)]
    unsafe fn splat64(x: i64) -> Self {
        [unsafe { V::splat64(x) }; L]
    }
    /// The same lanes in each vector.
    #[inline(always)]
    unsafe fn from_array(x: [i32; 16]) -> Self {
        [unsafe { V::from_array(x) }; L]
    }
    /// `L` vectors one after the other.
    #[inline(always)]
    unsafe fn load(p: *const i32) -> Self {
        core::array::from_fn(|i| unsafe { V::load(p.add(16 * i)) })
    }
    #[inline(always)]
    unsafe fn store(self, p: *mut i32) {
        for (i, v) in self.into_iter().enumerate() {
            unsafe { v.store(p.add(16 * i)) }
        }
    }
    #[inline(always)]
    fn add(self, b: Self) -> Self {
        each(self, b, V::add)
    }
    #[inline(always)]
    fn sub(self, b: Self) -> Self {
        each(self, b, V::sub)
    }
    #[inline(always)]
    fn mullo(self, b: Self) -> Self {
        each(self, b, V::mullo)
    }
    #[inline(always)]
    fn mul_even(self, b: Self) -> Self {
        each(self, b, V::mul_even)
    }
    #[inline(always)]
    fn add64(self, b: Self) -> Self {
        each(self, b, V::add64)
    }
    #[inline(always)]
    fn sub64(self, b: Self) -> Self {
        each(self, b, V::sub64)
    }
    #[inline(always)]
    fn sra64<const N: u32>(self) -> Self {
        self.map(V::sra64::<N>)
    }
    #[inline(always)]
    fn odd(self) -> Self {
        self.map(V::odd)
    }
    #[inline(always)]
    fn blend(self, odd: Self) -> Self {
        each(self, odd, V::blend)
    }
    #[inline(always)]
    fn highs(self, odd: Self) -> Self {
        each(self, odd, V::highs)
    }
    #[inline(always)]
    fn permute2(self, idx: Self, b: Self) -> Self {
        core::array::from_fn(|i| self[i].permute2(idx[i], b[i]))
    }
}

/// `f` on the elements of `a` and `b` in turn.
#[inline(always)]
fn each<V: Copy, const L: usize>(a: [V; L], b: [V; L], f: impl Fn(V, V) -> V) -> [V; L] {
    core::array::from_fn(|i| f(a[i], b[i]))
}

/// The constants of one prime.
#[derive(Clone, Copy)]
struct Consts<V> {
    q: V,
    v: V,
    /// `2^(30 + S)`, which rounds [`reduce`]'s quotient.
    round: V,
}

impl<V: V16> Consts<V> {
    /// Safety: `V` values may be created.
    #[inline(always)]
    unsafe fn new(p: Prime) -> Self {
        unsafe {
            Consts {
                q: V::splat(p.q),
                v: V::splat(p.v),
                round: V::splat64(1 << (30 + S)),
            }
        }
    }
}

/// [`Prime::reduce`]. Its quotient `round(floor(x v / 2^31) / 2^S)` is
/// `floor((x v + 2^(30 + S)) / 2^(31 + S))`: adding less than one to `x v / 2^31` before
/// the outer floor changes nothing. So it is the low half of an even lane's product shifted
/// by `31 + S`, and the high half of an odd lane's shifted by `S - 1`.
#[inline(always)]
fn reduce<V: V16>(x: V, c: &Consts<V>) -> V {
    let even = x.mul_even(c.v).add64(c.round).sra64::<{ 31 + S as u32 }>();
    let odd = x
        .odd()
        .mul_even(c.v)
        .add64(c.round)
        .sra64::<{ S as u32 - 1 }>();
    x.sub(even.blend(odd).mullo(c.q))
}

/// [`Prime::mont`]: `(a w - m q) / 2^32` is exact in 64 bits, and its high half is the
/// result.
#[inline(always)]
fn mont<V: V16>(a: V, w: V, wq: V, q: V) -> V {
    let m = a.mullo(wq);
    let even = a.mul_even(w).sub64(m.mul_even(q));
    let odd = a.odd().mul_even(w.odd()).sub64(m.odd().mul_even(q));
    even.highs(odd)
}

/// A stage of [`portable::forward`] on one vector of pairs.
#[inline(always)]
fn dif<V: V16>((x, y): (V, V), (w, wq): (V, V), c: &Consts<V>) -> (V, V) {
    (reduce(x.add(y), c), mont(x.sub(y), w, wq, c.q))
}

/// A stage of [`portable::inverse`] on one vector of pairs; `LAZY` skips the reductions.
#[inline(always)]
fn dit<V: V16, const LAZY: bool>((x, y): (V, V), (w, wq): (V, V), c: &Consts<V>) -> (V, V) {
    let t = mont(y, w, wq, c.q);
    let (s, d) = (x.add(t), x.sub(t));
    if LAZY {
        (s, d)
    } else {
        (reduce(s, c), reduce(d, c))
    }
}

/// The `h = 1` stage, whose twiddle is 1.
#[inline(always)]
fn last<V: V16>((x, y): (V, V), c: &Consts<V>) -> (V, V) {
    (reduce(x.add(y), c), reduce(x.sub(y), c))
}

// A block of 32 values sits in two registers in "layout h", for h = 1, 2, 4, 8, 16: the
// first holds the positions whose bit h is clear, in order, the second their partners in
// a stage of distance h. Layout 16 is the natural order.

/// The position of lane `i` of the first register of layout `h`.
const fn first(i: usize, h: usize) -> usize {
    (i & (h - 1)) | ((i & !(h - 1)) << 1)
}

/// The lane of the two registers (0 to 31) that holds position `pos` in layout `h`.
const fn lane(pos: usize, h: usize) -> usize {
    16 * (pos & h != 0) as usize + ((pos & (h - 1)) | ((pos >> 1) & !(h - 1)))
}

/// [`V16::permute2`] indices from layout `from` to the two registers of layout `to`.
const fn regroup(from: usize, to: usize) -> [[i32; 16]; 2] {
    let mut idx = [[0; 16]; 2];
    let mut i = 0;
    while i < 16 {
        idx[0][i] = lane(first(i, to), from) as i32;
        idx[1][i] = lane(first(i, to) + to, from) as i32;
        i += 1;
    }
    idx
}

/// The forward block's regroupings: after the stages `h = 16, 8, 4, 2`, then back to the
/// natural order after `h = 1`.
const FORWARD: [[[i32; 16]; 2]; 5] = [
    regroup(16, 8),
    regroup(8, 4),
    regroup(4, 2),
    regroup(2, 1),
    regroup(1, 16),
];

/// The inverse block's: from the natural order to layout 1, then after the stages
/// `h = 1, 2, 4, 8`.
const INVERSE: [[[i32; 16]; 2]; 5] = [
    regroup(16, 1),
    regroup(1, 2),
    regroup(2, 4),
    regroup(4, 8),
    regroup(8, 16),
];

/// What every block of a transform takes: the twiddles of the stages `h = 16, 8, 4, 2` in
/// their layouts, and the regroupings.
struct Block<V> {
    w: [(V, V); 4],
    idx: [(V, V); 5],
}

impl<V: V16> Block<V> {
    /// From the twiddles `w`, `wq` (forward or inverse) of a transform of at least 32
    /// points. Safety: `V` values may be created.
    #[inline(always)]
    unsafe fn new(w: &[i32], wq: &[i32], idx: &[[[i32; 16]; 2]; 5]) -> Self {
        // In layout `h` the second register's lane `i` holds a position that is `i` modulo
        // `h`.
        let at = |t: &[i32], h: usize| core::array::from_fn(|i| t[h + (i & (h - 1))]);
        let tw = |h| unsafe { (V::from_array(at(w, h)), V::from_array(at(wq, h))) };
        let idx = idx.map(|[a, b]| unsafe { (V::from_array(a), V::from_array(b)) });
        Block {
            w: [16, 8, 4, 2].map(tw),
            idx,
        }
    }
}

/// [`V16::permute2`] of `x` and `y` by both index vectors.
#[inline(always)]
fn regroup2<V: V16>((x, y): (V, V), (i, j): (V, V)) -> (V, V) {
    (x.permute2(i, y), x.permute2(j, y))
}

// The blocks spell out their stages: with default flags LLVM kept a loop over the
// forward ones, reloading their twiddles and indices from the stack.

/// The forward stages `h = 16, 8, 4, 2, 1` on a block of 32 values, its first and second
/// 16.
#[inline(always)]
fn forward32<V: V16>(xy: (V, V), b: &Block<V>, c: &Consts<V>) -> (V, V) {
    let xy = regroup2(dif(xy, b.w[0], c), b.idx[0]);
    let xy = regroup2(dif(xy, b.w[1], c), b.idx[1]);
    let xy = regroup2(dif(xy, b.w[2], c), b.idx[2]);
    let xy = regroup2(dif(xy, b.w[3], c), b.idx[3]);
    regroup2(last(xy, c), b.idx[4])
}

/// The inverse stages `h = 1, 2, 4, 8, 16` on the 32 values at `p`, of a transform of
/// more than 16 points: `h = 2` and `h = 8` are lazy (`portable::lazy`).
///
/// Safety: `V` values may be created; 32 values at `p` are readable and writable.
#[inline(always)]
unsafe fn inverse32<V: V16>(p: *mut i32, b: &Block<V>, c: &Consts<V>) {
    let xy = unsafe { (V::load(p), V::load(p.add(16))) };
    let xy = last(regroup2(xy, b.idx[0]), c);
    let xy = dit::<V, true>(regroup2(xy, b.idx[1]), b.w[3], c);
    let xy = dit::<V, false>(regroup2(xy, b.idx[2]), b.w[2], c);
    let xy = dit::<V, true>(regroup2(xy, b.idx[3]), b.w[1], c);
    let (x, y) = dit::<V, false>(regroup2(xy, b.idx[4]), b.w[0], c);
    unsafe {
        x.store(p);
        y.store(p.add(16));
    }
}

/// Two (vector, companion) twiddle pairs as one for `[V; 2]`.
#[inline(always)]
fn both<V: V16>((a, aq): (V, V), (b, bq): (V, V)) -> ([V; 2], [V; 2]) {
    ([a, b], [aq, bq])
}

/// [`portable::forward`]. Every step works on two independent vectors at once
/// (`[V; 2]`): the two butterflies of a stage in each four-value group, two neighbouring
/// vectors of the `h = 32` stage, or two blocks.
///
/// Safety: `V` values may be created.
#[inline(always)]
unsafe fn forward_v<V: V16>(a: &mut [i32], p: Prime, t: &Twiddles) {
    let n = a.len();
    assert!(n.is_power_of_two() && n <= t.fw.len() && n <= t.fwq.len());
    if n < 64 {
        return portable::forward(a, p, t);
    }
    let c = unsafe { Consts::<[V; 2]>::new(p) };
    let (pa, w, wq) = (a.as_mut_ptr(), t.fw.as_ptr(), t.fwq.as_ptr());
    // SAFETY (loads and stores): `s + 2 h <= n`, `j + 16 <= h / 2`, or `j + 32 <= h`,
    // and twiddle `i` below `2 h <= n`.
    let tw = |i: usize| unsafe { (V::load(w.add(i)), V::load(wq.add(i))) };
    let mut h = n / 2;
    while h >= 32 {
        if h >= 64 {
            // Stages `h` and `g = h / 2` on four values `g` apart: first `(x0, x2)` with
            // `(x1, x3)`, then `(x0, x1)` with `(x2, x3)`.
            let g = h / 2;
            for s in (0..n).step_by(2 * h) {
                for j in (0..g).step_by(16) {
                    unsafe {
                        let ps = [0, g, h, h + g].map(|o| pa.add(s + j + o));
                        let [x0, x1, x2, x3] = ps.map(|p| V::load(p));
                        let tw1 = both(tw(h + j), tw(h + g + j));
                        let ([x0, x1], [x2, x3]) = dif(([x0, x1], [x2, x3]), tw1, &c);
                        let tw2 = tw(g + j);
                        let ([x0, x2], [x1, x3]) = dif(([x0, x2], [x1, x3]), both(tw2, tw2), &c);
                        for (p, x) in ps.into_iter().zip([x0, x1, x2, x3]) {
                            x.store(p);
                        }
                    }
                }
            }
            h /= 4;
        } else {
            for s in (0..n).step_by(2 * h) {
                for j in (0..h).step_by(32) {
                    unsafe {
                        let px = [pa.add(s + j), pa.add(s + j + 16)];
                        let (x, y) = (px.map(|p| V::load(p)), px.map(|p| V::load(p.add(h))));
                        let (x, y) = dif((x, y), both(tw(h + j), tw(h + j + 16)), &c);
                        for (p, (x, y)) in px.into_iter().zip(x.into_iter().zip(y)) {
                            x.store(p);
                            y.store(p.add(h));
                        }
                    }
                }
            }
            h /= 2;
        }
    }
    let b = unsafe { Block::new(&t.fw, &t.fwq, &FORWARD) };
    for i in (0..n).step_by(64) {
        // SAFETY: `i + 64 <= n`: two blocks of 32.
        unsafe {
            let ps = [pa.add(i), pa.add(i + 32)];
            let xy = (ps.map(|p| V::load(p)), ps.map(|p| V::load(p.add(16))));
            let (x, y) = forward32(xy, &b, &c);
            for (p, (x, y)) in ps.into_iter().zip(x.into_iter().zip(y)) {
                x.store(p);
                y.store(p.add(16));
            }
        }
    }
}

/// [`portable::inverse`], one vector at a time: on pairs, as [`forward_v`], it was only
/// 2-5% faster per butterfly on Zen 5, with no gain measurable end to end, and it ran
/// equally fast with default and native flags either way.
///
/// Safety: `V` values may be created.
#[inline(always)]
unsafe fn inverse_v<V: V16>(a: &mut [i32], p: Prime, t: &Twiddles) {
    let n = a.len();
    assert!(n.is_power_of_two() && n <= t.iw.len() && n <= t.iwq.len());
    if n < 32 {
        return portable::inverse(a, p, t);
    }
    debug_assert!(portable::lazy(2, n) && !portable::lazy(4, n) && portable::lazy(8, n));
    let c = unsafe { Consts::<V>::new(p) };
    let pa = a.as_mut_ptr();
    let b = unsafe { Block::new(&t.iw, &t.iwq, &INVERSE) };
    for i in (0..n).step_by(32) {
        // SAFETY: `i + 32 <= n`.
        unsafe { inverse32(pa.add(i), &b, &c) }
    }
    let mut h = 32;
    while h < n {
        if 4 * h <= n {
            // SAFETY: `4 h <= n` and `n <= t.iw.len()`.
            unsafe {
                match (portable::lazy(h, n), portable::lazy(2 * h, n)) {
                    (false, false) => inverse4::<V, false, false>(pa, n, h, t, &c),
                    (false, true) => inverse4::<V, false, true>(pa, n, h, t, &c),
                    (true, false) => inverse4::<V, true, false>(pa, n, h, t, &c),
                    (true, true) => inverse4::<V, true, true>(pa, n, h, t, &c),
                }
            }
            h *= 4;
        } else {
            // SAFETY: `2 h <= n` and `n <= t.iw.len()`.
            unsafe {
                match portable::lazy(h, n) {
                    false => inverse2::<V, false>(pa, n, h, t, &c),
                    true => inverse2::<V, true>(pa, n, h, t, &c),
                }
            }
            h *= 2;
        }
    }
}

/// The inverse stages `h` and `2 h` on four values `h` apart; `L1` and `L2` skip their
/// reductions.
///
/// Safety: `V` values may be created; `p` has `n` values, `n` is a multiple of `4 h`,
/// `h` of 16, and `t.iw`, `t.iwq` have at least `4 h`.
#[inline(always)]
unsafe fn inverse4<V: V16, const L1: bool, const L2: bool>(
    p: *mut i32,
    n: usize,
    h: usize,
    t: &Twiddles,
    c: &Consts<V>,
) {
    let (w, wq) = (t.iw.as_ptr(), t.iwq.as_ptr());
    // SAFETY (loads and stores): `s + 4 h <= n`, `j + 16 <= h`, twiddle `i` below `4 h`.
    let tw = |i: usize| unsafe { (V::load(w.add(i)), V::load(wq.add(i))) };
    for s in (0..n).step_by(4 * h) {
        for j in (0..h).step_by(16) {
            unsafe {
                let ps = [0, h, 2 * h, 3 * h].map(|o| p.add(s + j + o));
                let [x0, x1, x2, x3] = ps.map(|p| V::load(p));
                let (x0, x1) = dit::<V, L1>((x0, x1), tw(h + j), c);
                let (x2, x3) = dit::<V, L1>((x2, x3), tw(h + j), c);
                let (x0, x2) = dit::<V, L2>((x0, x2), tw(2 * h + j), c);
                let (x1, x3) = dit::<V, L2>((x1, x3), tw(3 * h + j), c);
                for (p, x) in ps.into_iter().zip([x0, x1, x2, x3]) {
                    x.store(p);
                }
            }
        }
    }
}

/// The inverse stage `h`; `LAZY` skips the reductions.
///
/// Safety: as [`inverse4`], with `2 h` for `4 h`.
#[inline(always)]
unsafe fn inverse2<V: V16, const LAZY: bool>(
    p: *mut i32,
    n: usize,
    h: usize,
    t: &Twiddles,
    c: &Consts<V>,
) {
    let (w, wq) = (t.iw.as_ptr(), t.iwq.as_ptr());
    for s in (0..n).step_by(2 * h) {
        for j in (0..h).step_by(16) {
            // SAFETY: `s + 2 h <= n`, `j + 16 <= h`, twiddle `h + j + 16 <= 2 h`.
            unsafe {
                let (px, py) = (p.add(s + j), p.add(s + j + h));
                let tw = (V::load(w.add(h + j)), V::load(wq.add(h + j)));
                let (x, y) = dit::<V, LAZY>((V::load(px), V::load(py)), tw, c);
                x.store(px);
                y.store(py);
            }
        }
    }
}

/// [`portable::pointwise`].
///
/// Safety: `V` values may be created.
#[inline(always)]
unsafe fn pointwise_v<V: V16>(a: &mut [i32], b: &[i32], p: Prime) {
    let n = a.len();
    assert_eq!(n, b.len());
    if !n.is_multiple_of(16) {
        return portable::pointwise(a, b, p);
    }
    let (k, kq) = p.scale(n);
    let (q, qinv, k, kq) = unsafe { (V::splat(p.q), V::splat(p.qinv), V::splat(k), V::splat(kq)) };
    for i in (0..n).step_by(16) {
        // SAFETY: `i + 16 <= n` for both slices.
        unsafe {
            let (x, y) = (V::load(a.as_ptr().add(i)), V::load(b.as_ptr().add(i)));
            let x = mont(x, y, y.mullo(qinv), q);
            mont(x, k, kq, q).store(a.as_mut_ptr().add(i));
        }
    }
}

/// [`portable::square`].
///
/// Safety: `V` values may be created.
#[inline(always)]
unsafe fn square_v<V: V16>(a: &mut [i32], p: Prime) {
    let n = a.len();
    if !n.is_multiple_of(16) {
        return portable::square(a, p);
    }
    let (k, kq) = p.scale(n);
    let (q, qinv, k, kq) = unsafe { (V::splat(p.q), V::splat(p.qinv), V::splat(k), V::splat(kq)) };
    for i in (0..n).step_by(16) {
        // SAFETY: `i + 16 <= n`.
        unsafe {
            let x = V::load(a.as_ptr().add(i));
            let x = mont(x, x, x.mullo(qinv), q);
            mont(x, k, kq, q).store(a.as_mut_ptr().add(i));
        }
    }
}

/// The kernels on `__m512i`.
#[cfg(target_arch = "x86_64")]
mod z {
    use super::{Prime, Twiddles, V16};
    use core::arch::x86_64::*;

    // Safety (each): the CPU has AVX-512F.

    #[target_feature(enable = "avx512f")]
    pub(super) unsafe fn forward(a: &mut [i32], p: Prime, t: &Twiddles) {
        unsafe { super::forward_v::<Z>(a, p, t) }
    }

    #[target_feature(enable = "avx512f")]
    pub(super) unsafe fn inverse(a: &mut [i32], p: Prime, t: &Twiddles) {
        unsafe { super::inverse_v::<Z>(a, p, t) }
    }

    #[target_feature(enable = "avx512f")]
    pub(super) unsafe fn pointwise(a: &mut [i32], b: &[i32], p: Prime) {
        unsafe { super::pointwise_v::<Z>(a, b, p) }
    }

    #[target_feature(enable = "avx512f")]
    pub(super) unsafe fn square(a: &mut [i32], p: Prime) {
        unsafe { super::square_v::<Z>(a, p) }
    }

    #[derive(Clone, Copy)]
    struct Z(__m512i);

    // SAFETY (each block): a `Z` exists, or the caller promises one may, so the CPU has
    // AVX-512F; pointer contracts are the caller's.
    impl V16 for Z {
        #[inline(always)]
        unsafe fn splat(x: i32) -> Self {
            Z(unsafe { _mm512_set1_epi32(x) })
        }
        #[inline(always)]
        unsafe fn splat64(x: i64) -> Self {
            Z(unsafe { _mm512_set1_epi64(x) })
        }
        #[inline(always)]
        unsafe fn from_array(x: [i32; 16]) -> Self {
            Z(unsafe { core::mem::transmute::<[i32; 16], __m512i>(x) })
        }
        #[inline(always)]
        unsafe fn load(p: *const i32) -> Self {
            Z(unsafe { _mm512_loadu_epi32(p) })
        }
        #[inline(always)]
        unsafe fn store(self, p: *mut i32) {
            unsafe { _mm512_storeu_epi32(p, self.0) }
        }
        #[inline(always)]
        fn add(self, b: Self) -> Self {
            Z(unsafe { _mm512_add_epi32(self.0, b.0) })
        }
        #[inline(always)]
        fn sub(self, b: Self) -> Self {
            Z(unsafe { _mm512_sub_epi32(self.0, b.0) })
        }
        #[inline(always)]
        fn mullo(self, b: Self) -> Self {
            Z(unsafe { _mm512_mullo_epi32(self.0, b.0) })
        }
        #[inline(always)]
        fn mul_even(self, b: Self) -> Self {
            Z(unsafe { _mm512_mul_epi32(self.0, b.0) })
        }
        #[inline(always)]
        fn add64(self, b: Self) -> Self {
            Z(unsafe { _mm512_add_epi64(self.0, b.0) })
        }
        #[inline(always)]
        fn sub64(self, b: Self) -> Self {
            Z(unsafe { _mm512_sub_epi64(self.0, b.0) })
        }
        #[inline(always)]
        fn sra64<const N: u32>(self) -> Self {
            Z(unsafe { _mm512_srai_epi64::<N>(self.0) })
        }
        #[inline(always)]
        fn odd(self) -> Self {
            Z(unsafe { _mm512_srli_epi64::<32>(self.0) })
        }
        #[inline(always)]
        fn blend(self, odd: Self) -> Self {
            Z(unsafe { _mm512_mask_blend_epi32(0xAAAA, self.0, odd.0) })
        }
        #[inline(always)]
        fn highs(self, odd: Self) -> Self {
            Z(unsafe { _mm512_mask_shuffle_epi32::<_MM_PERM_DDBB>(odd.0, 0x5555, self.0) })
        }
        #[inline(always)]
        fn permute2(self, idx: Self, b: Self) -> Self {
            Z(unsafe { _mm512_permutex2var_epi32(self.0, idx.0, b.0) })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ntt::tables::tests::samples;
    use crate::ntt::tables::{PRIMES, Tables};
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    /// The lanes of [`V16`] in an array, with each operation as the instruction defines it.
    #[derive(Clone, Copy)]
    struct Emu([i32; 16]);

    impl Emu {
        fn map(self, b: Self, f: impl Fn(i32, i32) -> i32) -> Self {
            Emu(core::array::from_fn(|i| f(self.0[i], b.0[i])))
        }

        fn wide(self) -> [i64; 8] {
            core::array::from_fn(|i| {
                (self.0[2 * i] as u32 as i64) | ((self.0[2 * i + 1] as i64) << 32)
            })
        }

        fn narrow(x: [i64; 8]) -> Self {
            Emu(core::array::from_fn(|i| {
                (x[i / 2] >> (32 * (i % 2))) as i32
            }))
        }

        fn map64(self, b: Self, f: impl Fn(i64, i64) -> i64) -> Self {
            let (x, y) = (self.wide(), b.wide());
            Emu::narrow(core::array::from_fn(|i| f(x[i], y[i])))
        }
    }

    impl V16 for Emu {
        unsafe fn splat(x: i32) -> Self {
            Emu([x; 16])
        }
        unsafe fn splat64(x: i64) -> Self {
            Emu::narrow([x; 8])
        }
        unsafe fn from_array(x: [i32; 16]) -> Self {
            Emu(x)
        }
        unsafe fn load(p: *const i32) -> Self {
            Emu(core::array::from_fn(|i| unsafe { *p.add(i) }))
        }
        unsafe fn store(self, p: *mut i32) {
            for (i, &x) in self.0.iter().enumerate() {
                unsafe { *p.add(i) = x }
            }
        }
        fn add(self, b: Self) -> Self {
            self.map(b, i32::wrapping_add)
        }
        fn sub(self, b: Self) -> Self {
            self.map(b, i32::wrapping_sub)
        }
        fn mullo(self, b: Self) -> Self {
            self.map(b, i32::wrapping_mul)
        }
        fn mul_even(self, b: Self) -> Self {
            Emu::narrow(core::array::from_fn(|i| {
                self.0[2 * i] as i64 * b.0[2 * i] as i64
            }))
        }
        fn add64(self, b: Self) -> Self {
            self.map64(b, i64::wrapping_add)
        }
        fn sub64(self, b: Self) -> Self {
            self.map64(b, i64::wrapping_sub)
        }
        fn sra64<const N: u32>(self) -> Self {
            self.map64(self, |x, _| x >> N)
        }
        fn odd(self) -> Self {
            self.map64(self, |x, _| (x as u64 >> 32) as i64)
        }
        fn blend(self, odd: Self) -> Self {
            Emu(core::array::from_fn(|i| {
                if i % 2 == 0 { self.0[i] } else { odd.0[i] }
            }))
        }
        fn highs(self, odd: Self) -> Self {
            Emu(core::array::from_fn(|i| {
                if i % 2 == 0 { self.0[i + 1] } else { odd.0[i] }
            }))
        }
        fn permute2(self, idx: Self, b: Self) -> Self {
            Emu(idx.0.map(|j| match j as usize % 32 {
                j @ 0..16 => self.0[j],
                j => b.0[j - 16],
            }))
        }
    }

    /// [`reduce`] and [`mont`] against [`Prime::reduce`] and [`Prime::mont`], on any
    /// `i32`s: both are exact there (`|a w - m q| < 2^63`).
    #[test]
    fn reduce_and_mont_match_scalar() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(44);
        let mut x = vec![
            i32::MIN,
            i32::MIN + 1,
            -2,
            -1,
            0,
            1,
            2,
            i32::MAX - 1,
            i32::MAX,
        ];
        x.extend((0..4000).map(|_| rng.next_u32() as i32));
        x.extend(samples(&mut rng, 1 << 30, 4000));
        let x = x.as_chunks::<16>().0;
        for p in PRIMES {
            let c = unsafe { Consts::<Emu>::new(p) };
            for (&a, &w) in x.iter().zip(x.iter().rev()) {
                let (a, w) = (Emu(a), Emu(w));
                let wq = Emu(w.0.map(|w| p.wq(w)));
                assert_eq!(
                    reduce(a, &c).0,
                    a.0.map(|a| p.reduce(a)),
                    "reduce {:?}",
                    a.0
                );
                let want: [i32; 16] = core::array::from_fn(|i| p.mont(a.0[i], w.0[i], wq.0[i]));
                assert_eq!(mont(a, w, wq, c.q).0, want, "mont {:?} {:?}", a.0, w.0);
            }
        }
    }

    /// Each regrouping moves every position from where layout `from` holds it to where
    /// layout `to` does.
    #[test]
    fn regroup_moves_positions() {
        let all = [
            (16, 8),
            (8, 4),
            (4, 2),
            (2, 1),
            (1, 16),
            (16, 1),
            (1, 2),
            (2, 4),
            (4, 8),
            (8, 16),
        ];
        for (from, to) in all {
            // The registers of layout `from`, holding their positions.
            let mut held = [0; 32];
            for pos in 0..32 {
                held[lane(pos, from)] = pos;
            }
            for (r, idx) in regroup(from, to).iter().enumerate() {
                for (i, &j) in idx.iter().enumerate() {
                    assert_eq!(
                        held[j as usize],
                        first(i, to) + r * to,
                        "{from} -> {to}, lane {i}"
                    );
                }
            }
        }
    }

    type Forward = unsafe fn(&mut [i32], Prime, &Twiddles);
    type Pointwise = unsafe fn(&mut [i32], &[i32], Prime);
    type Square = unsafe fn(&mut [i32], Prime);

    /// The kernels on `V` give exactly what the portable ones give: on random values in
    /// `(-q, q)` and at its ends, for every transform length up to 2^16.
    fn matches_portable(forward: Forward, inverse: Forward, pointwise: Pointwise, square: Square) {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(43);
        let tables = Tables::new(16);
        for (p, tw) in PRIMES.into_iter().zip(&tables.tw) {
            let m = p.q - 1;
            for log in 3..=16 {
                let n = 1 << log;
                let x = samples(&mut rng, p.q, n)[..n].to_vec();
                let y = samples(&mut rng, p.q, n)[..n].to_vec();
                let ends: Vec<i32> = (0..n).map(|i| if i % 3 == 0 { m } else { -m }).collect();
                for x in [x, ends, vec![m; n], vec![-m; n]] {
                    let (mut a, mut b) = (x.clone(), x.clone());
                    unsafe { forward(&mut a, p, tw) };
                    portable::forward(&mut b, p, tw);
                    assert!(a == b, "forward, n {n}");
                    let (mut a, mut b) = (x.clone(), x.clone());
                    unsafe { inverse(&mut a, p, tw) };
                    portable::inverse(&mut b, p, tw);
                    assert!(a == b, "inverse, n {n}");
                    let (mut a, mut b) = (x.clone(), x.clone());
                    unsafe { pointwise(&mut a, &y, p) };
                    portable::pointwise(&mut b, &y, p);
                    assert!(a == b, "pointwise, n {n}");
                    let (mut a, mut b) = (x.clone(), x.clone());
                    unsafe { square(&mut a, p) };
                    portable::square(&mut b, p);
                    assert!(a == b, "square, n {n}");
                }
            }
        }
    }

    #[test]
    fn emulation_matches_portable() {
        matches_portable(
            forward_v::<Emu>,
            inverse_v::<Emu>,
            pointwise_v::<Emu>,
            square_v::<Emu>,
        );
    }

    /// Set `BASE58_REQUIRE_IFMA` to fail instead of skipping on CPUs without AVX-512.
    #[cfg(target_arch = "x86_64")]
    #[test]
    fn avx512_matches_portable() {
        if !crate::cpu::avx512f() {
            assert!(
                std::env::var_os("BASE58_REQUIRE_IFMA").is_none(),
                "no AVX-512F"
            );
            eprintln!("skipped: no AVX-512F");
            return;
        }
        matches_portable(z::forward, z::inverse, z::pointwise, z::square);
    }
}
