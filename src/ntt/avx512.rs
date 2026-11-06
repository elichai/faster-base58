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
use super::tables::{Prime, S};

/// [`super::Kernels`] with the transforms and pointwise products written here, the rest
/// from [`super::x86::kernels`], when this CPU has AVX-512F.
#[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
pub(crate) fn kernels() -> Option<super::Kernels> {
    if !crate::cpu::avx512f() {
        return None;
    }
    Some(super::Kernels {
        // SAFETY: the AVX-512F check above succeeded.
        forward: |a, p, t| unsafe { z::forward(a, p, t) },
        // SAFETY: the AVX-512F check above succeeded.
        inverse: |a, p, iw| unsafe { z::inverse(a, p, iw) },
        // SAFETY: the AVX-512F check above succeeded.
        pointwise: |a, b, p| unsafe { z::pointwise(a, b, p) },
        // SAFETY: the AVX-512F check above succeeded.
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

/// Two vectors as one, each operation applied to each in turn, so that code on them lists
/// the steps of two independent computations together. On Zen 5 a forward transform that
/// did one butterfly after the other took 1.33-1.55x as long with default flags as with
/// `-C target-cpu=native`: the same instructions, in another order. On pairs
/// ([`forward_v`]) the two builds are within 3% of each other, and both faster.
///
/// Code on vectors here uses no closures and no `core` array or iterator adapters: those
/// are separate functions without the kernels' target features, so a copy that LLVM keeps
/// out of line calls each intrinsic in it.
impl<V: V16> V16 for [V; 2] {
    #[inline(always)]
    unsafe fn splat(x: i32) -> Self {
        // SAFETY: the caller permits V creation; both elements use that same V.
        [unsafe { V::splat(x) }; 2]
    }
    #[inline(always)]
    unsafe fn splat64(x: i64) -> Self {
        // SAFETY: the caller permits V creation; both elements use that same V.
        [unsafe { V::splat64(x) }; 2]
    }
    /// The same lanes in each vector.
    #[inline(always)]
    unsafe fn from_array(x: [i32; 16]) -> Self {
        // SAFETY: the caller permits V creation; both elements use that same V.
        [unsafe { V::from_array(x) }; 2]
    }
    /// Two vectors one after the other.
    #[inline(always)]
    unsafe fn load(p: *const i32) -> Self {
        // SAFETY: the caller provides 32 readable lanes and permits V creation.
        unsafe { [V::load(p), V::load(p.add(16))] }
    }
    #[inline(always)]
    unsafe fn store(self, p: *mut i32) {
        // SAFETY: the caller provides 32 writable lanes.
        unsafe {
            self[0].store(p);
            self[1].store(p.add(16));
        }
    }
    #[inline(always)]
    fn add(self, b: Self) -> Self {
        [self[0].add(b[0]), self[1].add(b[1])]
    }
    #[inline(always)]
    fn sub(self, b: Self) -> Self {
        [self[0].sub(b[0]), self[1].sub(b[1])]
    }
    #[inline(always)]
    fn mullo(self, b: Self) -> Self {
        [self[0].mullo(b[0]), self[1].mullo(b[1])]
    }
    #[inline(always)]
    fn mul_even(self, b: Self) -> Self {
        [self[0].mul_even(b[0]), self[1].mul_even(b[1])]
    }
    #[inline(always)]
    fn add64(self, b: Self) -> Self {
        [self[0].add64(b[0]), self[1].add64(b[1])]
    }
    #[inline(always)]
    fn sub64(self, b: Self) -> Self {
        [self[0].sub64(b[0]), self[1].sub64(b[1])]
    }
    #[inline(always)]
    fn sra64<const N: u32>(self) -> Self {
        [self[0].sra64::<N>(), self[1].sra64::<N>()]
    }
    #[inline(always)]
    fn odd(self) -> Self {
        [self[0].odd(), self[1].odd()]
    }
    #[inline(always)]
    fn blend(self, odd: Self) -> Self {
        [self[0].blend(odd[0]), self[1].blend(odd[1])]
    }
    #[inline(always)]
    fn highs(self, odd: Self) -> Self {
        [self[0].highs(odd[0]), self[1].highs(odd[1])]
    }
    #[inline(always)]
    fn permute2(self, idx: Self, b: Self) -> Self {
        [
            self[0].permute2(idx[0], b[0]),
            self[1].permute2(idx[1], b[1]),
        ]
    }
}

/// The constants of one prime.
#[derive(Clone, Copy)]
struct Consts<V> {
    q: V,
    qinv: V,
    v: V,
    /// `2^(30 + S)`, which rounds [`reduce`]'s quotient.
    round: V,
}

impl<V: V16> Consts<V> {
    /// Safety: `V` values may be created.
    #[inline(always)]
    unsafe fn new(p: Prime) -> Self {
        // SAFETY: the caller permits creation of V constants.
        unsafe {
            Consts {
                q: V::splat(p.q),
                qinv: V::splat(p.qinv),
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
    /// From the twiddles `w` (forward or inverse) of a transform of at least 32
    /// points. Safety: `V` values may be created.
    #[inline(always)]
    unsafe fn new(w: &[i32], qinv: V, idx: &[[[i32; 16]; 2]; 5]) -> Self {
        // SAFETY: the caller permits V creation.
        unsafe {
            Block {
                w: [
                    layout_twiddles(w, 16, qinv),
                    layout_twiddles(w, 8, qinv),
                    layout_twiddles(w, 4, qinv),
                    layout_twiddles(w, 2, qinv),
                ],
                idx: [
                    vectors(&idx[0]),
                    vectors(&idx[1]),
                    vectors(&idx[2]),
                    vectors(&idx[3]),
                    vectors(&idx[4]),
                ],
            }
        }
    }
}

/// The twiddles of stage `h` in layout `h`, whose second register's lane `i` holds a
/// position that is `i` modulo `h`, and their [`mont`] companions.
///
/// Safety: `V` values may be created.
#[inline(always)]
unsafe fn layout_twiddles<V: V16>(w: &[i32], h: usize, qinv: V) -> (V, V) {
    let mut lanes = [0; 16];
    for (i, lane) in lanes.iter_mut().enumerate() {
        *lane = w[h + (i & (h - 1))];
    }
    // SAFETY: the caller permits V creation.
    let w = unsafe { V::from_array(lanes) };
    (w, w.mullo(qinv))
}

/// The 16 twiddles at `w` and their [`mont`] companions.
///
/// Safety: `V` values may be created; 16 values at `w` are readable.
#[inline(always)]
unsafe fn twiddles<V: V16>(w: *const i32, qinv: V) -> (V, V) {
    // SAFETY: as the caller promises.
    let w = unsafe { V::load(w) };
    (w, w.mullo(qinv))
}

/// Both index vectors of a regrouping. Safety: `V` values may be created.
#[inline(always)]
unsafe fn vectors<V: V16>([a, b]: &[[i32; 16]; 2]) -> (V, V) {
    // SAFETY: the caller permits V creation.
    unsafe { (V::from_array(*a), V::from_array(*b)) }
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
    // SAFETY: the caller provides 32 readable lanes and permits V creation.
    let xy = unsafe { (V::load(p), V::load(p.add(16))) };
    let xy = last(regroup2(xy, b.idx[0]), c);
    let xy = dit::<V, true>(regroup2(xy, b.idx[1]), b.w[3], c);
    let xy = dit::<V, false>(regroup2(xy, b.idx[2]), b.w[2], c);
    let xy = dit::<V, true>(regroup2(xy, b.idx[3]), b.w[1], c);
    let (x, y) = dit::<V, false>(regroup2(xy, b.idx[4]), b.w[0], c);
    // SAFETY: the caller provides 32 writable lanes; these two stores cover them exactly.
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
unsafe fn forward_v<V: V16>(a: &mut [i32], p: Prime, fw: &[i32]) {
    let n = a.len();
    assert!(n.is_power_of_two() && n <= fw.len());
    if n < 64 {
        return portable::forward(a, p, fw);
    }
    // SAFETY: the caller permits V creation, including pairs of V constants.
    let c = unsafe { Consts::<[V; 2]>::new(p) };
    let (pa, w, qinv) = (a.as_mut_ptr(), fw.as_ptr(), c.qinv[0]);
    let mut h = n / 2;
    while h >= 32 {
        if h >= 64 {
            // Stages `h` and `g = h / 2` on four values `g` apart: first `(x0, x2)` with
            // `(x1, x3)`, then `(x0, x1)` with `(x2, x3)`.
            let g = h / 2;
            for s in (0..n).step_by(2 * h) {
                for j in (0..g).step_by(16) {
                    // SAFETY: s + 2*h <= n and j + 16 <= g, so all four vectors fit in a;
                    // the twiddles end by 2 * h <= n, within the asserted length.
                    unsafe {
                        let p = pa.add(s + j);
                        let ps = [p, p.add(g), p.add(h), p.add(h + g)];
                        let x = [
                            V::load(ps[0]),
                            V::load(ps[1]),
                            V::load(ps[2]),
                            V::load(ps[3]),
                        ];
                        let tw1 = both(
                            twiddles(w.add(h + j), qinv),
                            twiddles(w.add(h + g + j), qinv),
                        );
                        let ([x0, x1], [x2, x3]) = dif(([x[0], x[1]], [x[2], x[3]]), tw1, &c);
                        let tw2 = twiddles(w.add(g + j), qinv);
                        let ([x0, x2], [x1, x3]) = dif(([x0, x2], [x1, x3]), both(tw2, tw2), &c);
                        x0.store(ps[0]);
                        x1.store(ps[1]);
                        x2.store(ps[2]);
                        x3.store(ps[3]);
                    }
                }
            }
            h /= 4;
        } else {
            for s in (0..n).step_by(2 * h) {
                for j in (0..h).step_by(32) {
                    // SAFETY: s + 2*h <= n and j + 32 <= h, so both vector pairs fit in a;
                    // the twiddles end by 2 * h <= n, within the asserted length.
                    unsafe {
                        let p = pa.add(s + j);
                        let (x, y) = (<[V; 2]>::load(p), <[V; 2]>::load(p.add(h)));
                        let tw = both(
                            twiddles(w.add(h + j), qinv),
                            twiddles(w.add(h + j + 16), qinv),
                        );
                        let (x, y) = dif((x, y), tw, &c);
                        x.store(p);
                        y.store(p.add(h));
                    }
                }
            }
            h /= 2;
        }
    }
    // SAFETY: the caller permits V creation; n >= 64 guarantees the 32-entry twiddles.
    let b = unsafe { Block::new(fw, c.qinv, &FORWARD) };
    for i in (0..n).step_by(64) {
        // SAFETY: `i + 64 <= n`: two blocks of 32.
        unsafe {
            let p = pa.add(i);
            let x = [V::load(p), V::load(p.add(32))];
            let y = [V::load(p.add(16)), V::load(p.add(48))];
            let (x, y) = forward32((x, y), &b, &c);
            x[0].store(p);
            x[1].store(p.add(32));
            y[0].store(p.add(16));
            y[1].store(p.add(48));
        }
    }
}

/// [`portable::inverse`], one vector at a time: on pairs, as [`forward_v`], it was only
/// 2-5% faster per butterfly on Zen 5, with no gain measurable end to end, and it ran
/// equally fast with default and native flags either way.
///
/// Safety: `V` values may be created.
#[inline(always)]
unsafe fn inverse_v<V: V16>(a: &mut [i32], p: Prime, iw: &[i32]) {
    let n = a.len();
    assert!(n.is_power_of_two() && n <= iw.len());
    if n < 32 {
        return portable::inverse(a, p, iw);
    }
    debug_assert!(portable::lazy(2, n) && !portable::lazy(4, n) && portable::lazy(8, n));
    // SAFETY: the caller permits V creation.
    let c = unsafe { Consts::<V>::new(p) };
    let pa = a.as_mut_ptr();
    // SAFETY: the caller permits V creation; n >= 32 guarantees the 32-entry twiddles.
    let b = unsafe { Block::new(iw, c.qinv, &INVERSE) };
    for i in (0..n).step_by(32) {
        // SAFETY: `i + 32 <= n`.
        unsafe { inverse32(pa.add(i), &b, &c) }
    }
    let mut h = 32;
    while h < n {
        if 4 * h <= n {
            // SAFETY: `4 h <= n` and `n <= iw.len()`.
            unsafe {
                match (portable::lazy(h, n), portable::lazy(2 * h, n)) {
                    (false, false) => inverse4::<V, false, false>(pa, n, h, iw, &c),
                    (false, true) => inverse4::<V, false, true>(pa, n, h, iw, &c),
                    (true, false) => inverse4::<V, true, false>(pa, n, h, iw, &c),
                    (true, true) => inverse4::<V, true, true>(pa, n, h, iw, &c),
                }
            }
            h *= 4;
        } else {
            // SAFETY: `2 h <= n` and `n <= iw.len()`.
            unsafe {
                match portable::lazy(h, n) {
                    false => inverse2::<V, false>(pa, n, h, iw, &c),
                    true => inverse2::<V, true>(pa, n, h, iw, &c),
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
/// `h` of 16, and `iw` has at least `4 h`.
#[inline(always)]
unsafe fn inverse4<V: V16, const L1: bool, const L2: bool>(
    p: *mut i32,
    n: usize,
    h: usize,
    iw: &[i32],
    c: &Consts<V>,
) {
    let w = iw.as_ptr();
    for s in (0..n).step_by(4 * h) {
        for j in (0..h).step_by(16) {
            // SAFETY: s + 4*h <= n and j + 16 <= h, so all four vectors fit in p; the
            // twiddles end by 4 * h, within the promised length.
            unsafe {
                let q = p.add(s + j);
                let ps = [q, q.add(h), q.add(2 * h), q.add(3 * h)];
                let x = [
                    V::load(ps[0]),
                    V::load(ps[1]),
                    V::load(ps[2]),
                    V::load(ps[3]),
                ];
                let tw1 = twiddles(w.add(h + j), c.qinv);
                let (x0, x1) = dit::<V, L1>((x[0], x[1]), tw1, c);
                let (x2, x3) = dit::<V, L1>((x[2], x[3]), tw1, c);
                let (x0, x2) = dit::<V, L2>((x0, x2), twiddles(w.add(2 * h + j), c.qinv), c);
                let (x1, x3) = dit::<V, L2>((x1, x3), twiddles(w.add(3 * h + j), c.qinv), c);
                x0.store(ps[0]);
                x1.store(ps[1]);
                x2.store(ps[2]);
                x3.store(ps[3]);
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
    iw: &[i32],
    c: &Consts<V>,
) {
    let w = iw.as_ptr();
    for s in (0..n).step_by(2 * h) {
        for j in (0..h).step_by(16) {
            // SAFETY: `s + 2 h <= n`, `j + 16 <= h`, twiddle `h + j + 16 <= 2 h`.
            unsafe {
                let (px, py) = (p.add(s + j), p.add(s + j + h));
                let tw = twiddles(w.add(h + j), c.qinv);
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
    // SAFETY: the caller permits V creation.
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
    // SAFETY: the caller permits V creation.
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
#[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
mod z {
    use super::{Prime, V16};
    use core::arch::x86_64::*;

    // Safety (each): the CPU has AVX-512F.

    #[target_feature(enable = "avx512f")]
    pub(super) unsafe fn forward(a: &mut [i32], p: Prime, fw: &[i32]) {
        // SAFETY: this function requires AVX-512F, so Z values may be created.
        unsafe { super::forward_v::<Z>(a, p, fw) }
    }

    #[target_feature(enable = "avx512f")]
    pub(super) unsafe fn inverse(a: &mut [i32], p: Prime, iw: &[i32]) {
        // SAFETY: this function requires AVX-512F, so Z values may be created.
        unsafe { super::inverse_v::<Z>(a, p, iw) }
    }

    #[target_feature(enable = "avx512f")]
    pub(super) unsafe fn pointwise(a: &mut [i32], b: &[i32], p: Prime) {
        // SAFETY: this function requires AVX-512F, so Z values may be created.
        unsafe { super::pointwise_v::<Z>(a, b, p) }
    }

    #[target_feature(enable = "avx512f")]
    pub(super) unsafe fn square(a: &mut [i32], p: Prime) {
        // SAFETY: this function requires AVX-512F, so Z values may be created.
        unsafe { super::square_v::<Z>(a, p) }
    }

    #[derive(Clone, Copy)]
    struct Z(__m512i);

    impl V16 for Z {
        #[inline(always)]
        unsafe fn splat(x: i32) -> Self {
            // SAFETY: the caller guarantees AVX-512F support.
            Z(unsafe { _mm512_set1_epi32(x) })
        }
        #[inline(always)]
        unsafe fn splat64(x: i64) -> Self {
            // SAFETY: the caller guarantees AVX-512F support.
            Z(unsafe { _mm512_set1_epi64(x) })
        }
        #[inline(always)]
        unsafe fn from_array(x: [i32; 16]) -> Self {
            // SAFETY: both types contain 512 initialized bits; all bit patterns are valid.
            Z(unsafe { core::mem::transmute::<[i32; 16], __m512i>(x) })
        }
        #[inline(always)]
        unsafe fn load(p: *const i32) -> Self {
            // SAFETY: the caller provides 16 readable lanes and AVX-512F support; unaligned load.
            Z(unsafe { _mm512_loadu_epi32(p) })
        }
        #[inline(always)]
        unsafe fn store(self, p: *mut i32) {
            // SAFETY: Z proves CPU support; the caller provides 16 writable lanes; unaligned store.
            unsafe { _mm512_storeu_epi32(p, self.0) }
        }
        #[inline(always)]
        fn add(self, b: Self) -> Self {
            // SAFETY: holding Z proves AVX-512F support.
            Z(unsafe { _mm512_add_epi32(self.0, b.0) })
        }
        #[inline(always)]
        fn sub(self, b: Self) -> Self {
            // SAFETY: holding Z proves AVX-512F support.
            Z(unsafe { _mm512_sub_epi32(self.0, b.0) })
        }
        #[inline(always)]
        fn mullo(self, b: Self) -> Self {
            // SAFETY: holding Z proves AVX-512F support.
            Z(unsafe { _mm512_mullo_epi32(self.0, b.0) })
        }
        #[inline(always)]
        fn mul_even(self, b: Self) -> Self {
            // SAFETY: holding Z proves AVX-512F support.
            Z(unsafe { _mm512_mul_epi32(self.0, b.0) })
        }
        #[inline(always)]
        fn add64(self, b: Self) -> Self {
            // SAFETY: holding Z proves AVX-512F support.
            Z(unsafe { _mm512_add_epi64(self.0, b.0) })
        }
        #[inline(always)]
        fn sub64(self, b: Self) -> Self {
            // SAFETY: holding Z proves AVX-512F support.
            Z(unsafe { _mm512_sub_epi64(self.0, b.0) })
        }
        #[inline(always)]
        fn sra64<const N: u32>(self) -> Self {
            // SAFETY: holding Z proves AVX-512F support.
            Z(unsafe { _mm512_srai_epi64::<N>(self.0) })
        }
        #[inline(always)]
        fn odd(self) -> Self {
            // SAFETY: holding Z proves AVX-512F support.
            Z(unsafe { _mm512_srli_epi64::<32>(self.0) })
        }
        #[inline(always)]
        fn blend(self, odd: Self) -> Self {
            // SAFETY: holding Z proves AVX-512F support.
            Z(unsafe { _mm512_mask_blend_epi32(0xAAAA, self.0, odd.0) })
        }
        #[inline(always)]
        fn highs(self, odd: Self) -> Self {
            // SAFETY: holding Z proves AVX-512F support.
            Z(unsafe { _mm512_mask_shuffle_epi32::<_MM_PERM_DDBB>(odd.0, 0x5555, self.0) })
        }
        #[inline(always)]
        fn permute2(self, idx: Self, b: Self) -> Self {
            // SAFETY: holding Z proves AVX-512F support.
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
            // SAFETY: the caller provides 16 readable i32 lanes.
            Emu(core::array::from_fn(|i| unsafe { *p.add(i) }))
        }
        unsafe fn store(self, p: *mut i32) {
            for (i, &x) in self.0.iter().enumerate() {
                // SAFETY: i < 16, within the caller's writable range.
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
            // SAFETY: Emu has no CPU requirements.
            let c = unsafe { Consts::<Emu>::new(p) };
            for (&a, &w) in x.iter().zip(x.iter().rev()) {
                let (a, w) = (Emu(a), Emu(w));
                let wq = w.mullo(c.qinv);
                assert_eq!(wq.0, w.0.map(|w| p.wq(w)));
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

    type Forward = unsafe fn(&mut [i32], Prime, &[i32]);
    type Inverse = unsafe fn(&mut [i32], Prime, &[i32]);
    type Pointwise = unsafe fn(&mut [i32], &[i32], Prime);
    type Square = unsafe fn(&mut [i32], Prime);

    /// The kernels on `V` give exactly what the portable ones give: on random values in
    /// `(-q, q)` and at its ends, for every transform length up to 2^16.
    fn matches_portable(forward: Forward, inverse: Inverse, pointwise: Pointwise, square: Square) {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(43);
        let tables = Tables::new(16);
        for (p, tw) in PRIMES.into_iter().zip(&tables.tw) {
            let mut iw = vec![0; tw.fw().len()];
            let iw = tw.inverse(&mut iw);
            let m = p.q - 1;
            for log in 3..=16 {
                let n = 1 << log;
                let x = samples(&mut rng, p.q, n)[..n].to_vec();
                let y = samples(&mut rng, p.q, n)[..n].to_vec();
                let ends: Vec<i32> = (0..n).map(|i| if i % 3 == 0 { m } else { -m }).collect();
                for x in [x, ends, vec![m; n], vec![-m; n]] {
                    let (mut a, mut b) = (x.clone(), x.clone());
                    // SAFETY: matches_portable receives Emu or CPU-checked AVX-512F kernels.
                    unsafe { forward(&mut a, p, tw.fw()) };
                    portable::forward(&mut b, p, tw.fw());
                    assert!(a == b, "forward, n {n}");
                    let (mut a, mut b) = (x.clone(), x.clone());
                    // SAFETY: the selected kernel supports this CPU; iw serves n points.
                    unsafe { inverse(&mut a, p, iw) };
                    portable::inverse(&mut b, p, iw);
                    assert!(a == b, "inverse, n {n}");
                    let (mut a, mut b) = (x.clone(), x.clone());
                    // SAFETY: the selected kernel supports this CPU; a and y have length n.
                    unsafe { pointwise(&mut a, &y, p) };
                    portable::pointwise(&mut b, &y, p);
                    assert!(a == b, "pointwise, n {n}");
                    let (mut a, mut b) = (x.clone(), x.clone());
                    // SAFETY: the selected kernel supports this CPU; n is a valid transform length.
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
    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
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
