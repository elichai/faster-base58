//! Schoolbook multiplication on AVX-512 IFMA (`vpmadd52luq` / `vpmadd52huq`), radix 2^52.
//!
//! IFMA multiplies the low 52 bits of two lanes and adds the low or the high 52 bits of the
//! product to a 64-bit lane; each add is below 2^52. A column of `a * b` gets at most
//! `min(ma, mb)` products for `ma`, `mb` digits. The kernel adds a column's low halves and
//! the previous column's high halves into one sum. Low halves are at most `2^52 - 1`,
//! high halves at most `2^52 - 2`, so with `min(ma, mb) <= 2048` a sum is at most
//! `2048 (2^53 - 3) = 2^64 - 6144`. This leaves room for `finish`'s carry of at most 4095.
//!
//! The code is written against [`V8`]: `__m512i` on x86-64, and an emulation in tests
//! that checks the arithmetic on any CPU.

/// Longest `a` native `mul` takes: `digits(MAX_LEN) = 2048`, the most for exact sums.
pub const MAX_LEN: usize = 1664;
const _: () = assert!(digits(MAX_LEN) == 2048);

const M52: u64 = (1 << 52) - 1;

/// Largest kernel window, in vectors of 8 columns: `2 NW_MAX` accumulators.
const NW_MAX: usize = if cfg!(feature = "compact") { 8 } else { 12 };

/// Digits of `n` limbs.
const fn digits(n: usize) -> usize {
    (64 * n).div_ceil(52)
}

/// Eight `u64` lanes. For `__m512i` the CPU must have AVX-512F and IFMA: creating a value
/// is `unsafe` with that contract, so holding one proves it and the other methods are
/// safe.
trait V8: Copy {
    unsafe fn splat(x: u64) -> Self;
    unsafe fn from_array(x: [u64; 8]) -> Self;
    /// Also requires 8 readable lanes at `p`.
    unsafe fn load(p: *const u64) -> Self;
    /// The first `min(n, 8)` lanes at `p`, which must be readable; the others are zero.
    unsafe fn load_n(p: *const u64, n: usize) -> Self;
    unsafe fn store(self, p: *mut u64);
    /// Stores the first `min(n, 8)` lanes.
    unsafe fn store_n(self, p: *mut u64, n: usize);
    fn add(self, b: Self) -> Self;
    fn and(self, b: Self) -> Self;
    fn or(self, b: Self) -> Self;
    fn shr52(self) -> Self;
    /// Per lane `self >> s` and `self << s`, zero for `s > 63`.
    fn shrv(self, s: Self) -> Self;
    fn shlv(self, s: Self) -> Self;
    /// Lanes moved up by one; lane 0 gets lane 7 of `prev`.
    fn up(self, prev: Self) -> Self;
    /// Lane `i` is lane `idx[i] % 8` of `self`.
    fn permute(self, idx: Self) -> Self;
    /// Lane `i` is lane `idx[i] % 16` of `self` followed by `hi`.
    fn permute2(self, idx: Self, hi: Self) -> Self;
    /// `self` plus the low (high) 52 bits of the product of the low 52 bits of `x`, `y`.
    fn madd52lo(self, x: Self, y: Self) -> Self;
    fn madd52hi(self, x: Self, y: Self) -> Self;
    /// Whether some lane has bit 52 set.
    fn bit52(self) -> bool;
    /// [`kernel`] for window `nw` in `1..=NW_MAX`.
    fn kernel(nw: usize) -> Kernel;
}

type Kernel = unsafe fn(c: *mut u64, a: *const u64, groups: usize, b: *const u64);

/// The function `$f::<NW>` for `NW = $nw`, as a [`Kernel`].
macro_rules! pick {
    ($f:ident, $nw:expr) => {
        core::cfg_select! {
            feature = "compact" => { pick!(@windows $f, $nw; 1, 2, 3, 4, 5, 6, 7, 8) }
            _ => { pick!(@windows $f, $nw; 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12) }
        }
    };
    (@windows $f:ident, $nw:expr; $($width:literal),+) => {
        match $nw {
            $($width => $f::<$width>,)+
            nw => panic!("window {nw}"),
        }
    };
}

#[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
pub(crate) mod avx512;

/// Scratch limbs native `mul` needs for `a.len() == na` and `b.len() == nb`.
///
/// With `ma <= mb` digits and `k = NW_MAX - 1 >= 7`, `chunks <= mb / (8 k) + 2`.
/// Then `blen <= mb + 8 chunks + 14` and `clen <= ma + mb + 21`, so this is at most
/// `(3 + 1/k) mb + 58 < 3.9 nb + 62` for both window limits.
#[cfg(any(test, feature = "__internals"))]
pub const fn scratch_len(na: usize, nb: usize) -> usize {
    Shape::new(na, nb).scratch()
}

/// Layout of one product. The kernel takes `b`'s digits in chunks: `chunks - 1` of `gb`
/// groups of 8 digits, then the rest.
struct Shape {
    /// `a`'s digits in groups of 8 (rows of the kernel).
    groups: usize,
    gb: usize,
    chunks: usize,
    /// Window of the last chunk (the others use `gb + 1`).
    nw_last: usize,
    /// `b`'s digits: `[0; 8]`, then each chunk but the last followed by `[0; 8]`, then the
    /// last chunk zero padded to its window.
    blen: usize,
    /// Column sums: what the last chunk writes, and what `finish` reads.
    clen: usize,
}

impl Shape {
    const fn new(na: usize, nb: usize) -> Shape {
        let mb = digits(nb);
        let gtot = mb.div_ceil(8);
        let (chunks, gb) = if (mb + 7).div_ceil(8) <= NW_MAX {
            (1, gtot)
        } else {
            let gb = gtot.div_ceil(gtot.div_ceil(NW_MAX - 1));
            (gtot.div_ceil(gb), gb)
        };
        let lead = 8 * (chunks - 1) * gb;
        let nw_last = (mb - lead + 7).div_ceil(8);
        let groups = digits(na).div_ceil(8);
        let (kernel, finish) = (lead + 8 * (groups + nw_last), 16 * (na + nb).div_ceil(13));
        Shape {
            groups,
            gb,
            chunks,
            nw_last,
            blen: 8 + (chunks - 1) * (8 * gb + 8) + 8 * nw_last,
            clen: if kernel > finish { kernel } else { finish },
        }
    }

    const fn scratch(&self) -> usize {
        // Up to 7 limbs align the buffers to 64 bytes.
        7 + self.blen + self.clen
    }
}

/// Multiplication on `V`. Safety: `V` values may be created (see [`V8`]).
#[inline(always)]
unsafe fn mul_v<V: V8>(out: &mut [u64], a: &[u64], b: &[u64], scratch: &mut [u64]) {
    let s = Shape::new(a.len(), b.len());
    assert!(8 <= a.len() && a.len() <= b.len() && a.len() <= MAX_LEN);
    assert!(out.len() == a.len() + b.len() && 8 * s.groups <= out.len());
    assert!(scratch.len() >= s.scratch());
    let skip = scratch.as_ptr().addr().wrapping_neg() % 64 / 8;
    let (bd, c) = scratch[skip..].split_at_mut(s.blen);
    let c = &mut c[..s.clen];
    let gtot = digits(b.len()).div_ceil(8);
    let at = |k: usize| 8 + k * (8 * s.gb + 8);
    // SAFETY: the caller's for `V`; see the comment at each kernel call.
    unsafe {
        // `a`'s digits go to `out`, which `finish` overwrites after the last kernel.
        to_digits::<V>(&mut out[..8 * s.groups], a, 0, s.groups);
        bd[..8].fill(0);
        for k in 0..s.chunks {
            let (g0, g1) = (k * s.gb, gtot.min((k + 1) * s.gb));
            let end = if k + 1 < s.chunks { at(k + 1) } else { s.blen };
            to_digits::<V>(&mut bd[at(k)..end], b, g0, g1);
            bd[at(k) + 8 * (g1 - g0)..end].fill(0);
        }
        c.fill(0);
        for k in 0..s.chunks {
            let nw = if k + 1 < s.chunks {
                s.gb + 1
            } else {
                s.nw_last
            };
            // Chunk `k` has at most `8 nw - 7` digits at `bd[at(k)..]`, zeros on both
            // sides to `bd[at(k) - 8..at(k) + 8 nw]`; it adds to
            // `c[8 k gb..8 (k gb + groups + nw)]`, inside `c` by `Shape::new`.
            V::kernel(nw)(
                c.as_mut_ptr().add(8 * k * s.gb),
                out.as_ptr(),
                s.groups,
                bd.as_ptr().add(at(k)),
            );
        }
        finish::<V>(out, c);
    }
}

/// Writes digits `8g..8g + 8` of `src` to `dst[8 (g - g0)..]` for `g` in `g0..g1`; digits
/// past the end of `src` are zero. Safety: `V` values may be created.
#[inline(always)]
unsafe fn to_digits<V: V8>(dst: &mut [u64], src: &[u64], g0: usize, g1: usize) {
    assert!(dst.len() >= 8 * (g1 - g0) && (g1 == g0 || 13 * (g1 - 1) / 2 < src.len()));
    // SAFETY: `V` by the caller. Group `g` starts at bit `416 g`, in limb `13 g / 2 <
    // src.len()`; at most `src.len() - 13 g / 2` limbs are read from there.
    unsafe {
        // Even groups start at bit 0 of their first limb, odd ones at bit 32. Per lane: the
        // limb holding the digit's low bits and the shift; the rest is in the next limb.
        let lo = [
            V::from_array([0, 0, 1, 2, 3, 4, 4, 5]),
            V::from_array([0, 1, 2, 2, 3, 4, 5, 6]),
        ];
        let hi = [
            V::from_array([1, 1, 2, 3, 4, 5, 5, 6]),
            V::from_array([1, 2, 3, 3, 4, 5, 6, 7]),
        ];
        let r = [
            V::from_array([0, 52, 40, 28, 16, 4, 56, 44]),
            V::from_array([32, 20, 8, 60, 48, 36, 24, 12]),
        ];
        let l = [
            V::from_array([64, 12, 24, 36, 48, 60, 8, 20]),
            V::from_array([32, 44, 56, 4, 16, 28, 40, 52]),
        ];
        let m52 = V::splat(M52);
        for g in g0..g1 {
            let p = 13 * g / 2;
            let x = V::load_n(src.as_ptr().add(p), src.len() - p);
            let o = g % 2;
            let d = x
                .permute(lo[o])
                .shrv(r[o])
                .or(x.permute(hi[o]).shlv(l[o]))
                .and(m52);
            d.store(dst.as_mut_ptr().add(8 * (g - g0)));
        }
    }
}

/// Adds the column sums of `a * chunk` to `c`: column `k` gets the low halves of its
/// products and the high halves of column `k - 1`'s. `a` holds `8 groups` digits; `b`
/// points at the chunk's digit 0, with zeros at `b[-7..0]` and from the chunk's end to
/// `b[8 NW - 1]`, so the chunk has at most `8 NW - 7` digits. Adds to `c[..8 (groups + NW)]`.
///
/// Rows go eight at a time. The accumulators cover the `8 NW` columns from `8q`; row
/// `8q + r` adds `a[8q + r] b[8u + s - r]` to column `8 (q + u) + s`. After each group
/// the lowest vector is final and the window moves up by eight. Window slot `u` is in
/// `l[(u + q) % NW]` and `h[(u + q) % NW]`, so nothing moves between registers.
///
/// Safety: `V` values may be created; the pointers as above.
#[inline(always)]
unsafe fn kernel<V: V8, const NW: usize>(c: *mut u64, a: *const u64, groups: usize, b: *const u64) {
    unsafe {
        let zero = V::splat(0);
        let (mut l, mut h) = ([zero; NW], [zero; NW]);
        // High halves of the slot flushed last.
        let mut hp = zero;
        let mut q = 0;
        // Adds slot `i`, columns `col..col + 8`, to `c`.
        macro_rules! flush {
            ($i:expr, $col:expr) => {
                let p = c.add($col);
                V::load(p).add(l[$i]).add(h[$i].up(hp)).store(p);
                hp = h[$i];
            };
        }
        macro_rules! step {
            ($k:literal) => {
                if $k < NW {
                    if q == groups {
                        for u in 0..NW {
                            flush!((u + $k) % NW, 8 * (groups + u));
                        }
                        return;
                    }
                    for r in 0..8 {
                        let x = V::splat(*a.add(8 * q + r));
                        let bp = b.sub(r);
                        for u in 0..NW {
                            let i = (u + $k) % NW;
                            let y = V::load(bp.add(8 * u));
                            l[i] = l[i].madd52lo(y, x);
                            h[i] = h[i].madd52hi(y, x);
                        }
                    }
                    flush!($k % NW, 8 * q);
                    l[$k % NW] = zero;
                    h[$k % NW] = zero;
                    q += 1;
                }
            };
        }
        loop {
            step!(0);
            step!(1);
            step!(2);
            step!(3);
            step!(4);
            step!(5);
            step!(6);
            step!(7);
            step!(8);
            step!(9);
            step!(10);
            step!(11);
        }
    }
}

/// Carries the column sums `c` into 52-bit digits and packs those into `out`. `c` holds
/// the `16 ceil(out.len() / 13)` columns that covers, each at most `2^64 - 6144` (the
/// kernel's bound); it is clobbered.
///
/// `s_k = (c_k mod 2^52) + (c_{k-1} >> 52) < 2^52 + 2^12`, then
/// `u_k = (s_k mod 2^52) + (s_{k-1} >> 52) <= 2^52`. Equality needs a sequential pass;
/// it takes long runs of ones. Safety: `V` values may be created.
#[inline(always)]
unsafe fn finish<V: V8>(out: &mut [u64], c: &mut [u64]) {
    let blocks = out.len().div_ceil(13);
    assert!(c.len() >= 16 * blocks);
    // SAFETY: `V` by the caller; every load is inside `c[..16 blocks]`.
    unsafe {
        let m52 = V::splat(M52);
        let (mut cp, mut sp, mut any) = (V::splat(0), V::splat(0), V::splat(0));
        for j in 0..blocks {
            let p = c.as_ptr().add(16 * j);
            let u0 = carry(carry(V::load(p), &mut cp, m52), &mut sp, m52);
            let u1 = carry(carry(V::load(p.add(8)), &mut cp, m52), &mut sp, m52);
            any = any.or(u0).or(u1);
            pack(out, j, u0, u1);
        }
        if any.bit52() {
            let mut carry = 0;
            for x in &mut c[..16 * blocks] {
                let v = *x + carry;
                (*x, carry) = (v & M52, v >> 52);
            }
            debug_assert_eq!(carry, 0);
            for j in 0..blocks {
                let p = c.as_ptr().add(16 * j);
                pack(out, j, V::load(p), V::load(p.add(8)));
            }
        }
    }
}

/// `v mod 2^52` plus `v >> 52` one lane down; `prev` holds `v >> 52` of the vector below.
#[inline(always)]
fn carry<V: V8>(v: V, prev: &mut V, m52: V) -> V {
    let h = v.shr52();
    let s = v.and(m52).add(h.up(*prev));
    *prev = h;
    s
}

/// Writes limbs `13j..13j + 13` of `out`, as far as `out` goes, from 16 digits below 2^52:
/// `u0` then `u1`. Safety: `V` values may be created.
#[inline(always)]
unsafe fn pack<V: V8>(out: &mut [u64], j: usize, u0: V, u1: V) {
    // SAFETY: `V` by the caller; at most `n` limbs are stored from `13j`.
    unsafe {
        let lo = limbs(
            u0,
            u1,
            [
                [0, 1, 2, 3, 4, 6, 7, 8],
                [1, 2, 3, 4, 5, 7, 8, 9],
                [2, 3, 4, 5, 6, 8, 9, 10],
            ],
            [
                [0, 12, 24, 36, 48, 8, 20, 32],
                [52, 40, 28, 16, 4, 44, 32, 20],
                [104, 92, 80, 68, 56, 96, 84, 72],
            ],
        );
        let hi = limbs(
            u0,
            u1,
            [
                [9, 11, 12, 13, 14, 0, 0, 0],
                [10, 12, 13, 14, 15, 0, 0, 0],
                [11, 13, 14, 15, 0, 0, 0, 0],
            ],
            [
                [44, 4, 16, 28, 40, 64, 64, 64],
                [8, 48, 36, 24, 12, 64, 64, 64],
                [60, 100, 88, 76, 64, 64, 64, 64],
            ],
        );
        let n = out.len() - 13 * j;
        let p = out.as_mut_ptr().add(13 * j);
        lo.store_n(p, n);
        if n > 8 {
            hi.store_n(p.add(8), (n - 8).min(5));
        }
    }
}

/// Lane `t`: digit `i[0][t]` of `u0, u1` shifted right by `s[0][t]`, ORed with digits
/// `i[1][t]` and `i[2][t]` shifted left by `s[1][t]` and `s[2][t]` (zero above 63).
/// Safety: `V` values may be created.
#[inline(always)]
unsafe fn limbs<V: V8>(u0: V, u1: V, i: [[u64; 8]; 3], s: [[u64; 8]; 3]) -> V {
    // SAFETY: the caller's.
    let v = |x: [u64; 8]| unsafe { V::from_array(x) };
    let d0 = u0.permute2(v(i[0]), u1).shrv(v(s[0]));
    d0.or(u0.permute2(v(i[1]), u1).shlv(v(s[1])))
        .or(u0.permute2(v(i[2]), u1).shlv(v(s[2])))
}

#[cfg(any(test, kani))]
mod emulation {
    use super::*;

    /// Plain-Rust lanes, to check the arithmetic on any CPU. Additions panic on overflow.
    #[derive(Clone, Copy)]
    pub(super) struct Emu([u64; 8]);

    impl Emu {
        fn map(self, b: Self, f: impl Fn(u64, u64) -> u64) -> Self {
            Emu(core::array::from_fn(|i| f(self.0[i], b.0[i])))
        }
    }

    unsafe fn kernel_emu<const NW: usize>(
        c: *mut u64,
        a: *const u64,
        groups: usize,
        b: *const u64,
    ) {
        unsafe { kernel::<Emu, NW>(c, a, groups, b) }
    }

    impl V8 for Emu {
        unsafe fn splat(x: u64) -> Self {
            Emu([x; 8])
        }
        unsafe fn from_array(x: [u64; 8]) -> Self {
            Emu(x)
        }
        unsafe fn load(p: *const u64) -> Self {
            Emu(core::array::from_fn(|i| unsafe { *p.add(i) }))
        }
        unsafe fn load_n(p: *const u64, n: usize) -> Self {
            Emu(core::array::from_fn(|i| {
                if i < n { unsafe { *p.add(i) } } else { 0 }
            }))
        }
        unsafe fn store(self, p: *mut u64) {
            unsafe { self.store_n(p, 8) }
        }
        unsafe fn store_n(self, p: *mut u64, n: usize) {
            for (i, &x) in self.0.iter().enumerate().take(n) {
                unsafe { *p.add(i) = x }
            }
        }
        fn add(self, b: Self) -> Self {
            self.map(b, |x, y| x + y)
        }
        fn and(self, b: Self) -> Self {
            self.map(b, |x, y| x & y)
        }
        fn or(self, b: Self) -> Self {
            self.map(b, |x, y| x | y)
        }
        fn shr52(self) -> Self {
            self.map(self, |x, _| x >> 52)
        }
        fn shrv(self, s: Self) -> Self {
            self.map(s, |x, s| if s < 64 { x >> s } else { 0 })
        }
        fn shlv(self, s: Self) -> Self {
            self.map(s, |x, s| if s < 64 { x << s } else { 0 })
        }
        fn up(self, prev: Self) -> Self {
            Emu(core::array::from_fn(|i| {
                if i == 0 { prev.0[7] } else { self.0[i - 1] }
            }))
        }
        fn permute(self, idx: Self) -> Self {
            Emu(idx.0.map(|j| self.0[j as usize % 8]))
        }
        fn permute2(self, idx: Self, hi: Self) -> Self {
            let both = [self.0, hi.0].concat();
            Emu(idx.0.map(|j| both[j as usize % 16]))
        }
        fn madd52lo(self, x: Self, y: Self) -> Self {
            let p = x.map(y, |x, y| {
                ((x & M52) as u128 * (y & M52) as u128) as u64 & M52
            });
            self.add(p)
        }
        fn madd52hi(self, x: Self, y: Self) -> Self {
            let p = x.map(y, |x, y| {
                (((x & M52) as u128 * (y & M52) as u128) >> 52) as u64
            });
            self.add(p)
        }
        fn bit52(self) -> bool {
            self.0.iter().any(|&x| x >> 52 & 1 == 1)
        }
        fn kernel(nw: usize) -> Kernel {
            pick!(kernel_emu, nw)
        }
    }

    #[cfg(kani)]
    #[kani::proof]
    #[kani::unwind(10)]
    fn carry_preserves_radix52_columns() {
        let columns: [u64; 8] = kani::any();
        let previous: u16 = kani::any();
        let previous = (previous & 4095) as u64;
        let mut high = Emu([previous; 8]);
        let carried = carry(Emu(columns), &mut high, Emu([M52; 8]));
        for i in 0..8 {
            let from_below = if i == 0 {
                previous
            } else {
                columns[i - 1] >> 52
            };
            let want = (columns[i] as u128 % (1u128 << 52)) + from_below as u128;
            assert_eq!(carried.0[i] as u128, want);
            assert_eq!(high.0[i], columns[i] >> 52);
        }
    }

    #[cfg(kani)]
    #[kani::proof]
    #[kani::unwind(18)]
    #[kani::solver(z3)]
    fn radix52_unpack_pack_roundtrip() {
        let input: [u64; 13] = kani::any();
        let mut digits = [0u64; 16];
        let mut output = [0u64; 13];
        // SAFETY: Emu has no CPU requirement; both complete eight-digit groups fit.
        unsafe {
            to_digits::<Emu>(&mut digits, &input, 0, 2);
            let lo = Emu(core::array::from_fn(|i| digits[i]));
            let hi = Emu(core::array::from_fn(|i| digits[8 + i]));
            pack::<Emu>(&mut output, 0, lo, hi);
        }
        for i in 0..13 {
            assert_eq!(output[i], input[i]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::emulation::Emu;
    use super::*;
    use crate::mul::tests::reference;
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    const GARBAGE: u64 = 0x5A5A_5A5A_5A5A_5A5A;

    /// `f` against the reference, with garbage in `out` and in exactly `scratch_len` of
    /// scratch, both at an offset that varies with the shape (so does the alignment), and
    /// canaries around both.
    fn check(
        f: &impl Fn(&mut [u64], &[u64], &[u64], &mut [u64]),
        a: &[u64],
        b: &[u64],
        what: &str,
    ) {
        let want = reference(a, b);
        let (n, ns) = (a.len() + b.len(), scratch_len(a.len(), b.len()));
        let o = 1 + n % 8;
        let mut out = vec![GARBAGE; n + 16];
        let mut scratch = vec![GARBAGE; ns + 16];
        f(&mut out[o..o + n], a, b, &mut scratch[o..o + ns]);
        assert!(out[o..o + n] == want, "{what}, {} x {}", a.len(), b.len());
        let canaries = out[..o].iter().chain(&out[o + n..]);
        let mut canaries = canaries.chain(&scratch[..o]).chain(&scratch[o + ns..]);
        assert!(
            canaries.all(|&x| x == GARBAGE),
            "canary, {what}, {} x {}",
            a.len(),
            b.len()
        );
    }

    fn limbs(rng: &mut Xoshiro256PlusPlus, n: usize) -> Vec<u64> {
        (0..n).map(|_| rng.next_u64()).collect()
    }

    /// Operand pairs that stress carries: all ones, long runs of ones, sparse, random.
    fn patterns(
        rng: &mut Xoshiro256PlusPlus,
        na: usize,
        nb: usize,
    ) -> Vec<(&'static str, Vec<u64>, Vec<u64>)> {
        let gap = |n: usize| {
            let mut x = vec![u64::MAX; n];
            x[n / 2] = 0;
            x
        };
        let sparse = |rng: &mut Xoshiro256PlusPlus, n: usize| {
            (0..n)
                .map(|_| {
                    if rng.next_u32().is_multiple_of(4) {
                        rng.next_u64()
                    } else {
                        0
                    }
                })
                .collect()
        };
        vec![
            ("ones", vec![u64::MAX; na], vec![u64::MAX; nb]),
            ("ones x rand", vec![u64::MAX; na], limbs(rng, nb)),
            ("gap", gap(na), vec![u64::MAX; nb]),
            (
                "alt",
                vec![0xAAAA_AAAA_AAAA_AAAA; na],
                vec![0x5555_5555_5555_5555; nb],
            ),
            ("sparse", sparse(rng, na), sparse(rng, nb)),
            ("rand", limbs(rng, na), limbs(rng, nb)),
            ("rand", limbs(rng, na), limbs(rng, nb)),
        ]
    }

    /// Shapes around every window and chunk boundary, through `f`.
    fn check_shapes(f: impl Fn(&mut [u64], &[u64], &[u64], &mut [u64])) {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(3);
        let nas = (8..=40).chain([46, 47, 48, 63, 64, 71, 72, 73, 89, 90, 100, 129, 200, 300]);
        for na in nas {
            // Include the default 72/73 boundary; the compact 46/47 cases use `na` above.
            for nb in [
                na,
                na + 1,
                na + 7,
                72,
                73,
                2 * na - 1,
                2 * na + 3,
                5 * na + 11,
                777,
            ] {
                if nb < na {
                    continue;
                }
                for (what, a, b) in patterns(&mut rng, na, nb) {
                    check(&f, &a, &b, what);
                }
            }
        }
        // Every chunk split and last-chunk window for either window limit.
        for na in [8, 16] {
            for nb in na..=1000 {
                check(&f, &limbs(&mut rng, na), &limbs(&mut rng, nb), "rand");
            }
        }
        // The longest `a`: column sums up to 2^64 - 6144 (all ones).
        for (na, nb) in [
            (MAX_LEN - 1, MAX_LEN - 1),
            (MAX_LEN, MAX_LEN),
            (MAX_LEN, 1700),
        ] {
            for (what, a, b) in patterns(&mut rng, na, nb) {
                check(&f, &a, &b, what);
            }
        }
    }

    #[test]
    fn emu_matches_reference() {
        check_shapes(|out, a, b, s| unsafe { mul_v::<Emu>(out, a, b, s) });
    }

    #[test]
    #[should_panic(expected = "a.len() <= MAX_LEN")]
    fn longer_than_max_len_panics() {
        let a = vec![1; MAX_LEN + 1];
        let mut out = vec![0; 2 * a.len()];
        let mut scratch = vec![0; scratch_len(a.len(), a.len())];
        unsafe { mul_v::<Emu>(&mut out, &a, &a, &mut scratch) };
    }

    /// Set `BASE58_REQUIRE_IFMA` to fail instead of skipping on CPUs without AVX-512 IFMA.
    #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
    #[test]
    fn avx512_matches_reference() {
        if !avx512::available() {
            assert!(
                std::env::var_os("BASE58_REQUIRE_IFMA").is_none(),
                "no AVX-512 IFMA"
            );
            eprintln!("skipped: no AVX-512 IFMA");
            return;
        }
        check_shapes(avx512::mul);
    }

    /// Columns whose two-step carry ends at 2^52 take the sequential pass (emulated, and on
    /// AVX-512 IFMA CPUs for real).
    #[test]
    fn finish_sequential_carry() {
        type Finish = unsafe fn(&mut [u64], &mut [u64]);
        let emu: Finish = finish::<Emu>;
        #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
        let fs = if avx512::available() {
            vec![emu, avx512::finish_tf]
        } else {
            vec![emu]
        };
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse2")))]
        let fs = [emu];
        for v in [true, false] {
            let mut rng = Xoshiro256PlusPlus::seed_from_u64(5);
            // c_1 = 2^52 - 1 and c_0 >> 52 = 1 give s_1 = 2^52; with s_2 mod 2^52 = 2^52 - 1,
            // u_2 = 2^52.
            // Columns below 2^60 up to 55 keep the value inside 52 limbs.
            let mut c: Vec<u64> = (0..64)
                .map(|k| if k < 56 { rng.next_u64() >> 4 } else { 0 })
                .collect();
            (c[0], c[1], c[2]) = (1 << 52, M52, M52);
            // The largest permitted column must also accept the incoming scalar carry.
            c[3] = u64::MAX - 6143;
            if v {
                // Plus a carry chain through a whole vector.
                c[8..24].fill(M52);
            }
            let mut want = vec![0u64; 52];
            let mut carry = 0u128;
            for (k, &x) in c.iter().enumerate() {
                carry += x as u128;
                let d = carry as u64 & M52;
                carry >>= 52;
                for (bit, w) in (52 * k..52 * k + 52).zip(0..) {
                    if bit < 64 * want.len() && d >> w & 1 == 1 {
                        want[bit / 64] |= 1 << (bit % 64);
                    }
                }
            }
            for (i, f) in fs.iter().enumerate() {
                let mut out = vec![GARBAGE; 52];
                unsafe { f(&mut out, &mut c.clone()) };
                assert_eq!(out, want, "finish {i}");
            }
        }
    }

    /// `Shape::new` keeps every chunk inside its window, and `scratch_len` fits in
    /// `mul::scratch_len` for every shape `mul::mul` can pass.
    #[test]
    fn shapes_fit() {
        for nb in 8..=1500 {
            let s = Shape::new(8, nb);
            let mb = digits(nb);
            assert!(s.chunks == 1 || s.gb < NW_MAX);
            assert!(s.nw_last <= NW_MAX && mb - 8 * (s.chunks - 1) * s.gb <= 8 * s.nw_last - 7);
            assert!(s.chunks == 1 || 8 * (s.chunks - 1) * s.gb < mb);
            for na in 8..=nb.min(MAX_LEN) {
                assert!(8 * digits(na).div_ceil(8) <= na + nb, "{na} x {nb}");
                assert!(
                    scratch_len(na, nb) <= crate::mul::scratch_len(nb),
                    "{na} x {nb}"
                );
            }
        }
    }
}
