//! Multiplication by number-theoretic transforms modulo three primes below 2^30, and the
//! divide-and-conquer decoders that use it.
//!
//! Each `u64` limb gives two 32-bit coefficients. A product is computed modulo each
//! prime by transform, pointwise product and inverse transform, then put together by
//! Chinese remaindering (`crt.rs`) and carried into limbs. A product that would fill
//! little of a power-of-two transform is computed modulo up to three shorter
//! polynomials instead, negacyclic and cyclic, and put together by polynomial Chinese
//! remaindering ([`Plan`], [`crt()`]).

use alloc::vec;
use alloc::vec::Vec;

#[cfg(any(test, feature = "__internals"))]
pub(crate) mod internals;

#[cfg(any(all(target_arch = "x86_64", target_feature = "sse2"), test))]
mod avx512;
#[cfg(test)]
mod cache_policy_tests;
mod crt;
#[cfg_attr(
    not(all(target_arch = "x86_64", target_feature = "sse2")),
    allow(
        dead_code,
        reason = "The kernels run only on x86-64 with IFMA, and in tests"
    )
)]
mod ifma;
#[cfg(all(test, miri))]
mod miri_tests;
#[cfg(all(
    target_arch = "aarch64",
    target_endian = "little",
    target_feature = "neon"
))]
mod neon;
mod portable;
#[cfg(kani)]
mod proofs;
#[cfg(any(
    test,
    feature = "__internals",
    all(
        target_arch = "aarch64",
        target_endian = "little",
        target_feature = "neon"
    ),
    all(target_arch = "x86_64", target_feature = "sse2")
))]
mod radix;
mod tables;
#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
mod wasm;
#[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
mod x86;

use core::ptr;

#[cfg(feature = "alloc")]
use crate::dc;
#[cfg(feature = "alloc")]
use crate::dc::Leaf;
use crate::dc::Multiplier;
use crate::encode::Leaves;
use crate::mul;
use ifma::{Kernels52, Tables52};
#[cfg(feature = "cache")]
use tables::cache::CACHE_MAX_LOG;
use tables::{MAX_LOG, PRIMES, Prime, Tables};

/// Products whose shorter operand has fewer limbs than this use Karatsuba. On an M5 Pro,
/// starting transforms at 256-1024 limbs with Toom-3 below them made whole decodes and
/// encodes 1.03-1.9x slower.
const THRESHOLD: usize = 128;

/// Products with a prepared power use transforms when both operands have at least this
/// many limbs. They need one forward transform fewer than other products.
const PREPARED_THRESHOLD: usize = 96;

/// [`THRESHOLD`] and [`PREPARED_THRESHOLD`] when Karatsuba has the AVX-512 IFMA leaf.
/// Tuned on whole decodes on Zen 5, where Karatsuba wins on prepared products of 1936
/// limbs and transforms win from 3430. A prepared threshold of 3072 made decodes up to 15%
/// slower. One Toom-3 level on products from 1536 limbs up to these thresholds changed
/// whole decodes and encodes by -3% to +3% depending on size; from 2304 limbs it gained
/// 1-3% only at 44-56 KiB.
const IFMA_THRESHOLD: usize = 3072;
const IFMA_PREPARED_THRESHOLD: usize = 2048;

/// The encoders' divide-and-conquer threshold in bytes and leaf size in chunks, tuned on
/// an M5 Pro against `encode_batched` in one binary. Smaller leaves were up to 4% faster
/// from 8 KiB up and 1-2% slower at 4-6 KiB.
#[cfg(any(not(feature = "cache"), test, feature = "__internals"))]
pub(crate) const ENCODE_THRESHOLD: usize = 4096;
const ENCODE_LEAF: usize = 32;

/// The leaf size of the caching encoders, which use [`Leaves::BEST`]. Matrix leaves pay off
/// much larger with NEON: in one binary with loops aligned to 64 bytes (M5 Pro), 768
/// chunks was 4-8% faster than 256 from 32 KiB to 1 MiB, and within 1% of 1024 there but
/// 3-4% faster at 12 and 24 KiB. With the x86 kernels (Zen 5) 96 was best: 256 within 5%,
/// 768 39% slower at 6 KiB and 58% at 8 KiB, where the whole input became one leaf.
const CACHED_LEAF: usize = match Leaves::BEST {
    Leaves::Horner => ENCODE_LEAF,
    Leaves::Matrix if cfg!(target_arch = "x86_64") => 96,
    Leaves::Matrix => 768,
};

/// Divide-and-conquer decoding with NTT multiplication and the feature-selected leaves.
/// With `cache`, twiddles through log18 and selected matrix-leaf tables are retained;
/// larger twiddles live for this call. Without `cache`, tables are owned and leaves
/// use Horner. Tables are built only when a product uses transforms.
#[inline(never)]
#[cfg(feature = "alloc")]
pub(crate) fn decode(
    input: &[u8],
    zeros: usize,
    output: &mut [u8],
) -> Result<usize, crate::common::DecodeError> {
    dc::decode_parts(input, zeros, output, &mut Ntt::best(), Leaf::BEST)
}

/// [`crate::dc::decode_value`] with the NTT multiplier of [`decode`].
#[cfg(all(feature = "check", feature = "alloc"))]
pub(crate) fn decode_value(
    input: &[u8],
    zeros: usize,
    capacity: usize,
) -> Result<alloc::vec::Vec<u64>, crate::common::DecodeError> {
    dc::decode_value(input, zeros, capacity, &mut Ntt::best(), Leaf::BEST)
}

/// Large encoding with NTT multiplication. NEON and x86 AVX-512 use a multiplication
/// tree in base 58^3 when transform padding and input size favor it. Other cases retain
/// the division tree and feature-selected leaves. Both routes retain twiddles
/// through log18 with `cache`; larger twiddles are owned per call. Powers,
/// prepared transforms and scratch remain local to the conversion.
#[inline(never)]
pub(crate) fn encode(
    zeros: usize,
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, crate::BufferTooSmall> {
    #[cfg(any(
        all(
            target_arch = "aarch64",
            target_endian = "little",
            target_feature = "neon"
        ),
        all(target_arch = "x86_64", target_feature = "sse2")
    ))]
    if let Some(result) = radix::try_encode(zeros, input, output) {
        return result;
    }
    let m = &mut Ntt::best();
    crate::encode::dc::encode_parts(zeros, input, output, m, CACHED_LEAF, Leaves::BEST)
}

#[cfg(all(feature = "check", feature = "alloc"))]
#[inline(never)]
pub(crate) fn encode_staged(
    zeros: usize,
    end: usize,
    output: &mut [u8],
) -> Result<usize, crate::BufferTooSmall> {
    #[cfg(any(
        all(
            target_arch = "aarch64",
            target_endian = "little",
            target_feature = "neon"
        ),
        all(target_arch = "x86_64", target_feature = "sse2")
    ))]
    if let Some(result) = radix::try_encode_staged(zeros, end, output) {
        return result;
    }
    crate::encode::dc::encode_staged(
        zeros,
        end,
        output,
        &mut Ntt::best(),
        CACHED_LEAF,
        Leaves::BEST,
    )
}

/// The [`Multiplier`] of the NTT decoders.
pub(crate) struct Ntt {
    /// Bypass the cache for owned-table comparisons in tests and benchmarks.
    #[cfg(all(feature = "cache", any(test, feature = "__internals")))]
    force_owned: bool,
    kernels: Kernels,
    /// The two-prime IFMA kernels, used instead of `kernels` when set.
    k52: Option<Kernels52>,
    /// Largest tables to use: longer products are split. [`MAX_LOG`] except in tests.
    max_log: u32,
    /// [`THRESHOLD`] and [`PREPARED_THRESHOLD`], or their IFMA values.
    threshold: usize,
    prepared_threshold: usize,
    /// Table size selected by `prepare`, without building a table yet.
    log: Option<u32>,
    owned: Option<Tables>,
    owned52: Option<Tables52>,
    #[cfg(feature = "cache")]
    shared: Option<&'static Tables>,
    #[cfg(feature = "cache")]
    shared52: Option<&'static Tables52>,
    /// Forward transforms of the prepared powers, by index.
    powers: Vec<Option<Prepared>>,
}

impl Ntt {
    /// With the thresholds for this CPU and the table policy selected by `cache`, and the
    /// three-prime `kernels`.
    pub(crate) fn new(kernels: Kernels) -> Self {
        let (threshold, prepared_threshold) = match mul::ifma_threshold() {
            0 => (THRESHOLD, PREPARED_THRESHOLD),
            _ => (IFMA_THRESHOLD, IFMA_PREPARED_THRESHOLD),
        };
        Ntt {
            #[cfg(all(feature = "cache", any(test, feature = "__internals")))]
            force_owned: false,
            kernels,
            k52: None,
            max_log: MAX_LOG,
            threshold,
            prepared_threshold,
            log: None,
            owned: None,
            owned52: None,
            #[cfg(feature = "cache")]
            shared: None,
            #[cfg(feature = "cache")]
            shared52: None,
            powers: Vec::new(),
        }
    }

    /// The fastest backend for this CPU: two primes with AVX-512 IFMA, else three.
    ///
    /// Two IFMA transforms replace three 32-bit Montgomery ones, at the cost of 16-byte
    /// twiddles and 8-byte residues. On Zen 5 (generic flags, fat LTO, public cdylibs,
    /// loops aligned to 64 in both builds, 2026-11-07), Auto decodes took 0.87-0.92x the
    /// three-prime-only time from 128 KiB to 4 MiB (0.87-0.95x with `cache`, from
    /// 64 KiB); radix-route encodes do not use this backend.
    pub(crate) fn best() -> Self {
        Ntt::new(Kernels::best()).with_ifma()
    }

    /// The two-prime IFMA kernels when this CPU has them.
    pub(crate) fn with_ifma(self) -> Self {
        Ntt {
            k52: Kernels52::best(),
            ..self
        }
    }

    /// Initialize tables only when a product will use transforms.
    fn ensure_tables(&mut self) {
        let log = self.log.expect("prepare runs first");
        if let Some(k) = self.k52 {
            #[cfg(feature = "cache")]
            if log <= CACHE_MAX_LOG && !self.owned_only() {
                self.shared52
                    .get_or_insert_with(|| Tables52::cached(log, &k));
                return;
            }
            self.owned52.get_or_insert_with(|| Tables52::new(log, &k));
            return;
        }
        #[cfg(feature = "cache")]
        if log <= CACHE_MAX_LOG && !self.owned_only() {
            self.shared.get_or_insert_with(|| Tables::cached(log));
            return;
        }
        self.owned.get_or_insert_with(|| Tables::new(log));
    }

    #[cfg(feature = "cache")]
    fn owned_only(&self) -> bool {
        #[cfg(any(test, feature = "__internals"))]
        return self.force_owned;
        #[cfg(not(any(test, feature = "__internals")))]
        false
    }

    /// Borrow the selected table with kernels and prepared powers after initialization.
    fn initialized_tables(&self) -> &Tables {
        #[cfg(feature = "cache")]
        if let Some(table) = self.shared {
            return table;
        }
        self.owned.as_ref().expect("ensure_tables runs first")
    }

    /// The three-prime backend after [`Ntt::ensure_tables`], or the two-prime one.
    fn backend(&self) -> Result<Two<'_>, Three<'_>> {
        match &self.k52 {
            Some(k) => {
                #[cfg(feature = "cache")]
                if let Some(t) = self.shared52 {
                    return Ok(Two { t, k });
                }
                let t = self.owned52.as_ref().expect("ensure_tables runs first");
                Ok(Two { t, k })
            }
            None => Err(Three {
                t: self.initialized_tables(),
                k: &self.kernels,
            }),
        }
    }

    /// `b`'s forward transforms for products of up to `len >= b.len()` limbs. `None` if
    /// `b` is below `prepared_threshold` or the tables are too short.
    fn prepare_operand(&mut self, b: &[u64], len: usize) -> Option<Prepared> {
        if b.len() < self.prepared_threshold {
            return None;
        }
        let log = log_len(len);
        if log > self.log.expect("prepare runs first") {
            return None;
        }
        self.ensure_tables();
        let data = match self.backend() {
            Ok(two) => Spectra::Two(spectra(two, b, log)),
            Err(three) => Spectra::Three(spectra(three, b, log)),
        };
        Some(Prepared {
            log,
            addr: b.as_ptr().addr(),
            len: b.len(),
            data,
        })
    }
}

/// The forward transforms of `b`, `2^log` points per prime, one prime after the other.
fn spectra<B: Backend>(k: B, b: &[u64], log: u32) -> Vec<B::E> {
    let n = 1 << log;
    let mut data = vec![B::E::default(); B::PRIMES << log];
    for (j, f) in data.chunks_exact_mut(n).enumerate() {
        k.load(j, f, b);
        k.forward(j, f);
    }
    data
}

/// Cyclic forward transforms of a prepared power `b`, `2^log` points per prime, one prime
/// after the other.
///
/// They hold, mod q, every transform of `b` that a product of at most `2^log` points
/// takes. In bit-reversed order, the first `d` values are the `d`-point transform of
/// `b mod (x^d - 1)`, and values `d..2 d` the twisted one of `b mod (x^d + 1)`, for every
/// power of two `d < 2^log`: they are the frequencies `j 2^log / d` and
/// `(2 j + 1) 2^log / (2 d)`. The values are in `(-q, q)`, or `[0, 2p)` for two primes.
struct Prepared {
    log: u32,
    /// Address and length of the power, to check that `mul_power` got it.
    addr: usize,
    len: usize,
    data: Spectra,
}

/// The residues of a [`Prepared`] power, for the backend that made them.
enum Spectra {
    Three(Vec<i32>),
    Two(Vec<u64>),
}

impl Prepared {
    fn is(&self, b: &[u64]) -> bool {
        (self.addr, self.len) == (b.as_ptr().addr(), b.len())
    }
}

impl Multiplier for Ntt {
    fn prepare(&mut self, len: usize) {
        // Products longer than the tables serve would be split.
        self.log = Some(log_len(len).clamp(5, self.max_log));
        self.owned.take();
        self.owned52.take();
        #[cfg(feature = "cache")]
        {
            self.shared.take();
            self.shared52.take();
        }
        self.powers.clear();
    }

    /// With two primes, operands below both thresholds take Karatsuba alone, so they ask
    /// only its scratch rather than the transform bound, which is half again larger.
    /// `n / 2 + len(n / 2) <= len(n)` still holds across the step, as
    /// `mul::scratch_len <= scratch_len`.
    fn scratch_len(&self, n: usize) -> usize {
        match self.k52 {
            Some(_) if n < self.threshold.min(self.prepared_threshold) => mul::scratch_len(n),
            Some(_) => scratch_len_two(n),
            None => scratch_len(n),
        }
    }

    fn mul(&mut self, out: &mut [u64], a: &[u64], b: &[u64], scratch: &mut [u64]) {
        if a.len().min(b.len()) < self.threshold.max(1) {
            return mul::mul(out, a, b, scratch);
        }
        self.ensure_tables();
        let t = self.threshold;
        match self.backend() {
            Ok(k) => mul_with(out, a, b, scratch, k, t),
            Err(k) => mul_with(out, a, b, scratch, k, t),
        }
    }

    fn prepare_power(&mut self, i: usize, b: &[u64], len: usize) {
        let f = self.prepare_operand(b, len);
        if self.powers.len() <= i {
            if f.is_none() {
                return;
            }
            self.powers.resize_with(i + 1, || None);
        }
        self.powers[i] = f;
    }

    fn mul_power(&mut self, out: &mut [u64], a: &[u64], i: usize, b: &[u64], s: &mut [u64]) {
        match self.powers.get(i) {
            // `b` is the prepared power, and the product fits its transforms, so also the
            // tables. When `a` is `b`, the square takes `a`'s transforms from them too.
            Some(Some(f))
                if f.is(b)
                    && a.len() >= self.prepared_threshold
                    && log_len(a.len() + b.len()) <= f.log =>
            {
                // Preparing `f` initialized the tables, and `prepare` clears both.
                match self.backend() {
                    Ok(k) => transform(out, a, b, s, k, Some(f)),
                    Err(k) => transform(out, a, b, s, k, Some(f)),
                }
            }
            _ => self.mul(out, a, b, s),
        }
    }

    /// The power of two `L >= n` when its `2 L`-point cyclic transform, which the tables
    /// must serve, is shorter than the pieces of a full product, and the operands are long
    /// enough for transforms at all.
    fn wrap_len(&self, n: usize, full: usize) -> Option<usize> {
        if n < self.threshold {
            return None;
        }
        let l = n.next_power_of_two().max(4);
        let fits = log_len(l) <= self.log.expect("prepare runs first");
        (fits && 2 * l < Plan::new(full).width()).then_some(l)
    }

    fn wrapped_scratch_len(&self, l: usize, _n: usize) -> usize {
        // Folded operands, then four arrays of `2 L` `i32`s (three of `2 L` `u64`s with two
        // primes) after at most 7 limbs that align them ([`as_i32`]).
        match self.k52 {
            Some(_) => 2 * l + 6 * l + 7,
            None => 2 * l + 4 * l + 7,
        }
    }

    fn mul_wrapped(&mut self, out: &mut [u64], a: &[u64], b: &[u64], s: &mut [u64]) {
        self.ensure_tables();
        match self.backend() {
            Ok(k) => wrapped(out, a, b, s, k, None),
            Err(k) => wrapped(out, a, b, s, k, None),
        }
    }

    fn mul_power_wrapped(
        &mut self,
        out: &mut [u64],
        a: &[u64],
        i: usize,
        b: &[u64],
        s: &mut [u64],
    ) {
        self.ensure_tables();
        let f = match self.powers.get(i) {
            Some(Some(f)) if f.is(b) && log_len(out.len()) <= f.log => Some(f),
            _ => None,
        };
        match self.backend() {
            Ok(k) => wrapped(out, a, b, s, k, f),
            Err(k) => wrapped(out, a, b, s, k, f),
        }
    }
}

/// `out = a * b mod (β^L - 1)` for `L = out.len()`, a power of two whose `2 L` points `t`
/// serves, as some value in `[0, β^L - 1]`: one cyclic transform per prime, of the
/// operands folded modulo `β^L - 1`. A prepared `b`'s transforms come from `fb`: their
/// first `2 L` values are those of `b mod (x^(2 L) - 1)` (see [`Prepared`]).
///
/// Each coefficient of the cyclic product sums at most `2 L <= 2^23` products of 32-bit
/// coefficients, so it is below `2^87`, less than the primes' product, and Garner's CRT
/// recovers it.
fn wrapped<B: Backend>(
    out: &mut [u64],
    a: &[u64],
    b: &[u64],
    scratch: &mut [u64],
    k: B,
    fb: Option<&Prepared>,
) {
    let l = out.len();
    let n = 2 * l;
    debug_assert!(l.is_power_of_two() && n >= 8 && n.trailing_zeros() <= k.log());
    let (fa, scratch) = scratch.split_at_mut(l);
    let (fold_b, scratch) = scratch.split_at_mut(l);
    let a = if a.len() > l {
        mul::fold_wrapped(fa, a);
        &*fa
    } else {
        a
    };
    let b = if fb.is_none() && b.len() > l {
        mul::fold_wrapped(fold_b, b);
        &*fold_b
    } else {
        b
    };
    let (r, rest) = B::elems(scratch).split_at_mut(B::PRIMES * n);
    let tmp = &mut rest[..n];
    for (j, r) in r.chunks_exact_mut(n).enumerate() {
        k.load(j, r, a);
        k.forward(j, r);
        if let Some(f) = fb {
            k.pointwise(j, r, &B::spectrum(f, j)[..n]);
        } else {
            k.load(j, tmp, b);
            k.forward(j, tmp);
            k.pointwise(j, r, tmp);
        }
        k.inverse(j, r, tmp);
    }
    k.finish(out, r, n, n, true);
}

/// The residue arithmetic of a transform backend: three primes below `2^30` in `i32`
/// lanes ([`Three`]), or two below `2^50` in `u64` lanes with AVX-512 IFMA ([`Two`]).
/// Prime `j`'s residues are passed with its index.
trait Backend: Copy {
    type E: Copy + Default;
    const PRIMES: usize;
    /// `log2` of the points the tables serve.
    fn log(self) -> u32;
    /// The scratch limbs from the first 64-byte boundary, at most 7 limbs in, as residues.
    fn elems(s: &mut [u64]) -> &mut [Self::E];
    /// Prime `j`'s transforms in `f`, which this backend prepared.
    fn spectrum(f: &Prepared, j: usize) -> &[Self::E];
    fn load(self, j: usize, dst: &mut [Self::E], limbs: &[u64]);
    /// [`Backend::load`], coefficient `i` times `w^i` for `w` a primitive `2 m`-th root,
    /// `m = dst.len()`.
    fn load_twist(self, j: usize, dst: &mut [Self::E], limbs: &[u64]);
    fn load_add(self, j: usize, dst: &mut [Self::E], limbs: &[u64], sub: bool);
    /// `a[i] w^i` for `w` a primitive `2 a.len()`-th root.
    fn twist(self, j: usize, a: &mut [Self::E]);
    /// `a[i] w^-i`, undoing [`Backend::twist`] after an inverse transform.
    fn untwist(self, j: usize, a: &mut [Self::E], tmp: &mut [Self::E]);
    /// `dst[i] = s[i] mod q` for `s[i] < 2^63`.
    fn narrow(self, j: usize, dst: &mut [Self::E], s: &[u64]);
    fn forward(self, j: usize, a: &mut [Self::E]);
    fn inverse(self, j: usize, a: &mut [Self::E], tmp: &mut [Self::E]);
    fn pointwise(self, j: usize, a: &mut [Self::E], b: &[Self::E]);
    fn square(self, j: usize, a: &mut [Self::E]);
    /// [`Kernels::pair`], with the roots of `x^m + 1` from the tables.
    fn pair(self, j: usize, r: &mut [Self::E], m: usize, tmp: &mut [Self::E]);
    fn fold_sub(self, j: usize, dst: &mut [Self::E], src: &[Self::E], start: usize, neg: bool);
    fn halves(self, j: usize, lo: &mut [Self::E], hi: &mut [Self::E]);
    /// Garner on the first `len` coefficients of each prime's `width`, then the carry
    /// into `out` ([`crt::carry`], or [`crt::carry_wrapped`] with `wrapped`).
    fn finish(self, out: &mut [u64], r: &mut [Self::E], width: usize, len: usize, wrapped: bool);
}

/// Three primes below `2^30`, [`Kernels`] on `i32` residues.
#[derive(Clone, Copy)]
struct Three<'a> {
    t: &'a Tables,
    k: &'a Kernels,
}

impl Backend for Three<'_> {
    type E = i32;
    const PRIMES: usize = 3;
    fn log(self) -> u32 {
        self.t.log
    }
    fn elems(s: &mut [u64]) -> &mut [i32] {
        as_i32(s)
    }
    fn spectrum(f: &Prepared, j: usize) -> &[i32] {
        let Spectra::Three(d) = &f.data else {
            unreachable!("prepared by the other backend")
        };
        &d[j << f.log..(j + 1) << f.log]
    }
    fn load(self, j: usize, dst: &mut [i32], limbs: &[u64]) {
        (self.k.load)(dst, limbs, PRIMES[j]);
    }
    fn load_twist(self, j: usize, dst: &mut [i32], limbs: &[u64]) {
        let m = dst.len();
        (self.k.load_twist)(dst, limbs, PRIMES[j], &self.t.tw[j].fw()[m..2 * m]);
    }
    fn load_add(self, j: usize, dst: &mut [i32], limbs: &[u64], sub: bool) {
        (self.k.load_add)(dst, limbs, PRIMES[j], sub);
    }
    fn twist(self, j: usize, a: &mut [i32]) {
        let m = a.len();
        (self.k.twist)(a, &self.t.tw[j].fw()[m..2 * m], PRIMES[j]);
    }
    fn untwist(self, j: usize, a: &mut [i32], tmp: &mut [i32]) {
        let m = a.len();
        let iw = &mut tmp[..m];
        tables::invert_roots(iw, &self.t.tw[j].fw()[m..2 * m]);
        (self.k.twist)(a, iw, PRIMES[j]);
    }
    fn narrow(self, j: usize, dst: &mut [i32], s: &[u64]) {
        narrow(dst, s, PRIMES[j]);
    }
    fn forward(self, j: usize, a: &mut [i32]) {
        (self.k.forward)(a, PRIMES[j], self.t.tw[j].fw());
    }
    fn inverse(self, j: usize, a: &mut [i32], tmp: &mut [i32]) {
        let iw = self.t.tw[j].inverse(&mut tmp[..a.len()]);
        (self.k.inverse)(a, PRIMES[j], iw);
    }
    fn pointwise(self, j: usize, a: &mut [i32], b: &[i32]) {
        (self.k.pointwise)(a, b, PRIMES[j]);
    }
    fn square(self, j: usize, a: &mut [i32]) {
        (self.k.square)(a, PRIMES[j]);
    }
    fn pair(self, j: usize, r: &mut [i32], m: usize, tmp: &mut [i32]) {
        let iw = &mut tmp[..m];
        tables::invert_roots(iw, &self.t.tw[j].fw()[m..2 * m]);
        (self.k.pair)(r, m, iw, PRIMES[j]);
    }
    fn fold_sub(self, j: usize, dst: &mut [i32], src: &[i32], start: usize, neg: bool) {
        (self.k.fold_sub)(dst, src, start, neg, PRIMES[j]);
    }
    fn halves(self, j: usize, lo: &mut [i32], hi: &mut [i32]) {
        (self.k.halves)(lo, hi, PRIMES[j]);
    }
    fn finish(self, out: &mut [u64], r: &mut [i32], width: usize, len: usize, wrapped: bool) {
        let (r1, rest) = r.split_at_mut(width);
        let (r2, r3) = rest.split_at_mut(width);
        let r3 = &mut r3[..width];
        (self.k.garner)([&mut r1[..len], &mut r2[..len], &mut r3[..len]]);
        if wrapped {
            crt::carry_wrapped(out, [r1, r2, r3]);
        } else {
            crt::carry(out, [r1, r2, r3]);
        }
    }
}

/// Two primes below `2^50`, [`Kernels52`] on `u64` residues.
#[derive(Clone, Copy)]
struct Two<'a> {
    t: &'a Tables52,
    k: &'a Kernels52,
}

impl Backend for Two<'_> {
    type E = u64;
    const PRIMES: usize = 2;
    fn log(self) -> u32 {
        self.t.log
    }
    fn elems(s: &mut [u64]) -> &mut [u64] {
        let skip = s.as_ptr().addr().wrapping_neg() % 64 / 8;
        &mut s[skip..]
    }
    fn spectrum(f: &Prepared, j: usize) -> &[u64] {
        let Spectra::Two(d) = &f.data else {
            unreachable!("prepared by the other backend")
        };
        &d[j << f.log..(j + 1) << f.log]
    }
    fn load(self, _: usize, dst: &mut [u64], limbs: &[u64]) {
        (self.k.load)(dst, limbs);
    }
    fn load_twist(self, j: usize, dst: &mut [u64], limbs: &[u64]) {
        (self.k.load_twist)(dst, limbs, &self.t.tw[j]);
    }
    fn load_add(self, j: usize, dst: &mut [u64], limbs: &[u64], sub: bool) {
        (self.k.load_add)(dst, limbs, self.t.tw[j].prime(), sub);
    }
    fn twist(self, j: usize, a: &mut [u64]) {
        (self.k.twist)(a, &self.t.tw[j]);
    }
    fn untwist(self, j: usize, a: &mut [u64], _: &mut [u64]) {
        (self.k.untwist)(a, a.len(), 0, &self.t.tw[j]);
    }
    fn narrow(self, j: usize, dst: &mut [u64], s: &[u64]) {
        (self.k.narrow)(dst, s, self.t.tw[j].prime());
    }
    fn forward(self, j: usize, a: &mut [u64]) {
        (self.k.forward)(a, &self.t.tw[j]);
    }
    fn inverse(self, j: usize, a: &mut [u64], _: &mut [u64]) {
        (self.k.inverse)(a, &self.t.tw[j]);
    }
    fn pointwise(self, j: usize, a: &mut [u64], b: &[u64]) {
        (self.k.pointwise)(a, b, self.t.tw[j].prime());
    }
    fn square(self, j: usize, a: &mut [u64]) {
        (self.k.square)(a, self.t.tw[j].prime());
    }
    /// As `portable::pair`, in passes: blocks `1..m/n` of the first residue are untwisted
    /// and subtracted from the second, then block 0 is untwisted and halved with it.
    fn pair(self, j: usize, r: &mut [u64], m: usize, _: &mut [u64]) {
        let (tw, k) = (&self.t.tw[j], self.k);
        let n = r.len() - m;
        assert!(n < m && m.is_multiple_of(n));
        let (e, s) = r.split_at_mut(m);
        let (e0, mid) = e.split_at_mut(n);
        for (i, blk) in mid.chunks_exact_mut(n).enumerate() {
            (k.untwist)(blk, m, (i + 1) * n, tw);
            (k.fold_sub)(s, blk, 0, false, tw.prime());
        }
        (k.untwist)(e0, m, 0, tw);
        (k.halves)(e0, s, tw.prime());
    }
    fn fold_sub(self, j: usize, dst: &mut [u64], src: &[u64], start: usize, neg: bool) {
        (self.k.fold_sub)(dst, src, start, neg, self.t.tw[j].prime());
    }
    fn halves(self, j: usize, lo: &mut [u64], hi: &mut [u64]) {
        (self.k.halves)(lo, hi, self.t.tw[j].prime());
    }
    fn finish(self, out: &mut [u64], r: &mut [u64], width: usize, len: usize, wrapped: bool) {
        let (r1, r2) = r.split_at_mut(width);
        let r2 = &mut r2[..width];
        (self.k.garner)(&mut r1[..len], &mut r2[..len]);
        let c = ifma::carry_out(out, r1, r2);
        if wrapped {
            crt::wrap_carry(out, c);
        } else {
            debug_assert_eq!(c, 0, "product does not fit");
        }
    }
}

/// The type of [`Kernels::load_twist`].
type LoadTwist = fn(dst: &mut [i32], limbs: &[u64], p: Prime, w: &[i32]);

/// One backend's kernels. Each works on one prime's residues, which stay in `(-q, q)`.
#[derive(Clone, Copy)]
pub(crate) struct Kernels {
    /// Writes the 32-bit halves of `limbs`, least significant first, to `dst`, reduced
    /// into `(-q, q)`, and zeroes the rest of `dst`.
    pub load: fn(dst: &mut [i32], limbs: &[u64], p: Prime),
    /// Forward transform: natural order in, bit-reversed order out. `a.len()` is a power
    /// of two, at least 8. `fw` contains at least `a.len()` forward roots from
    /// [`tables::Twiddles::fw`].
    pub forward: fn(a: &mut [i32], p: Prime, fw: &[i32]),
    /// Inverse transform: bit-reversed order in, natural order out, times `a.len()`.
    /// `iw` holds at least `a.len()` inverse roots generated by [`tables::Twiddles::inverse`].
    pub inverse: fn(a: &mut [i32], p: Prime, iw: &[i32]),
    /// `a[i] = a[i] * b[i] / a.len()`.
    pub pointwise: fn(a: &mut [i32], b: &[i32], p: Prime),
    /// [`Kernels::pointwise`] of `a` with itself: `a[i] = a[i]^2 / a.len()`.
    pub square: fn(a: &mut [i32], p: Prime),
    /// In place: `r1` to `[0, q1)`, `r2` to `v2`, `r3` to `v3` (see `crt.rs`). The
    /// lengths are equal and a multiple of 4.
    pub garner: fn(r: [&mut [i32]; 3]),
    /// [`Kernels::load`], with coefficient `i` times the root `w[i]` stands for
    /// (Montgomery form, as in [`tables::Twiddles`]). `w.len() >= 2 * limbs.len()`.
    pub load_twist: LoadTwist,
    /// Adds the 32-bit halves of `limbs` to the start of `dst`, or with `sub` subtracts
    /// them.
    pub load_add: fn(dst: &mut [i32], limbs: &[u64], p: Prime, sub: bool),
    /// `a[i]` times the root `w[i]` stands for. `a.len()` is a multiple of 4,
    /// at most `w.len()`.
    pub twist: fn(a: &mut [i32], w: &[i32], p: Prime),
    /// Subtracts `x^start src` from `dst`, modulo `x^d + 1` (`neg`) or `x^d - 1`, for
    /// `d = dst.len()`. `start`, `src.len()` and `d` are multiples of 4.
    pub fold_sub: fn(dst: &mut [i32], src: &[i32], start: usize, neg: bool, p: Prime),
    /// `(lo[i], hi[i]) = ((hi[i] + lo[i]) / 2, (hi[i] - lo[i]) / 2)`. The lengths are
    /// equal and a multiple of 4.
    pub halves: fn(lo: &mut [i32], hi: &mut [i32], p: Prime),
    /// [`crt()`] for one negacyclic piece of `m` coefficients, which is still twisted
    /// (`iw[i]` undoes it), fused with undoing the twist. `r.len() - m` is a
    /// multiple of 4 below `m`.
    pub pair: fn(r: &mut [i32], m: usize, iw: &[i32], p: Prime),
}

impl Kernels {
    /// The fastest kernels for this CPU: NEON on aarch64; SIMD128 on Wasm built with it;
    /// on `x86_64` those of `avx512.rs`
    /// with AVX-512F, the rest the portable ones compiled for AVX-512 or AVX2 when
    /// available.
    pub fn best() -> Self {
        core::cfg_select! {
            all(target_arch = "aarch64", target_endian = "little", target_feature = "neon") => {
                Kernels::neon()
            }
            all(target_arch = "x86_64", target_feature = "sse2") => {
                avx512::kernels().or_else(x86::kernels).unwrap_or_else(Kernels::portable)
            }
            all(target_arch = "wasm32", target_feature = "simd128") => Kernels::wasm(),
            _ => Kernels::portable(),
        }
    }
}

/// Scratch limbs [`mul_with`] needs when no operand is longer than `n` limbs.
///
/// - Transform: let `p = (2 (a + b)).next_multiple_of(8) <= 4 n + 6` and `t` its
///   largest power of two. A cyclic plan needs `2 L` limbs: `L = p`, or `p > 7 L / 8`,
///   except for `L <= 256` where the minimum cyclic rest prevents splitting;
///   those small cyclic plans also fit `12 n + 128` directly.
///   One negacyclic piece needs `2 t + 7 c / 2` limbs, including both sums arrays,
///   with `c <= t / 2` and `p > t + c / 2` when `c > 64`: at most `3 p` limbs.
///   When `c = 64`, the excess over `3 p` is at most 72 limbs (`t >= 128`).
///   Two pieces need `11 t / 4 + 7 c / 2`, with `c <= t / 4` and `p > 3 t / 2`,
///   below `3 p`. Adding rounding and at most 7 alignment limbs ([`as_i32`]) fits
///   `12 n + 128` in every case. Squares reuse the temporary for large inverse roots.
/// - Within the transform limit, use the workspace's running maximum through
///   `Plan::new(2 n)`: `2 L` for cyclic plans, `2 t + 7 c / 2` for one piece,
///   and the preceding one-piece maximum `15 t / 4` for two pieces. This covers
///   every smaller product without the drop between one and two pieces.
///   With 128 spare limbs and the linear cap, the result is between
///   `8 n + 128` and `12 n + 128`. Larger inputs keep the linear bound.
/// - Blocks (`2 a <= b`): `2 a + scratch_len(a) <= 14 a + 128 <= 7 n + 128`.
/// - Split (`b / 2 < a <= b`): at most `b + 1` limbs, then products of at most
///   `ceil(b / 2)` limbs: at most `7 b + 135 <= 8 b + 128` since `b >= 9`
///   (`a + b > 16`, because the tables serve at least 32 points).
/// - DC: `n / 2 + scratch_len(n / 2) <= 6.5 n + 128 <= 8 n + 128`.
/// - Below the threshold: [`mul::scratch_len`], which the result is never below.
pub(crate) fn scratch_len(n: usize) -> usize {
    let linear = 12 * n + 128;
    let bound = if n != 0 && n <= (1usize << (MAX_LOG - 2)) {
        let plan = Plan::new(2 * n);
        let prefix = match plan.k {
            0 => 2 * plan.cyc,
            1 => 2 * plan.nega[0] + 7 * (plan.cyc / 2),
            _ => 15 * (plan.nega[0] / 4),
        };
        (prefix + 128).min(linear)
    } else {
        linear
    };
    mul::scratch_len(n).max(bound)
}

/// [`scratch_len`] for the two-prime backend: its residues take `8` bytes for the `4` of
/// the three-prime residues, so a transform with `width` points and a temporary of at
/// most `t <= width` takes `2 width + t` limbs for `(3 width + t) / 2`, at most 1.5 times
/// as many. The other terms and the at most 7 alignment limbs are the same, so 1.5 times
/// the bound covers every case, and keeps `n / 2 + len(n / 2) <= len(n)`.
pub(crate) fn scratch_len_two(n: usize) -> usize {
    (3 * scratch_len(n)).div_ceil(2)
}

/// `log2` of the transform length for a product of `len` limbs: at least `2 len` points
/// (two 32-bit coefficients per limb), and at least 8.
fn log_len(len: usize) -> u32 {
    (2 * len).max(8).next_power_of_two().trailing_zeros()
}

/// `out = a * b`. Requires `out.len() == a.len() + b.len()`, tables of `log >= 5`, and
/// `scratch.len() >= scratch_len(max(a.len(), b.len()))` ([`scratch_len_two`] for
/// [`Two`]); `scratch` may hold anything.
///
/// Uses Karatsuba when the shorter operand is below `threshold` limbs, and splits
/// products too long for the tables.
fn mul_with<B: Backend>(
    out: &mut [u64],
    a: &[u64],
    b: &[u64],
    scratch: &mut [u64],
    k: B,
    threshold: usize,
) {
    debug_assert_eq!(out.len(), a.len() + b.len());
    debug_assert!(k.log() >= 5);
    let (a, b) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    if a.len() < threshold.max(1) {
        mul::mul(out, a, b, scratch);
    } else if 2 * a.len() <= b.len() {
        mul::mul_blocks(out, a, b, scratch, |o, x, y, s| {
            mul_with(o, x, y, s, k, threshold)
        });
    } else if log_len(a.len() + b.len()) > k.log() {
        split(out, a, b, scratch, k, threshold);
    } else {
        transform(out, a, b, scratch, k, None);
    }
}

/// `out = a * b` from the four products of the halves of `a` and of `b`, each a quarter
/// of the work. Requires `b.len() / 2 < a.len() <= b.len()`. When `a` and `b` are the
/// same slice, so are the operands of the two diagonal products, so those square too.
fn split<B: Backend>(
    out: &mut [u64],
    a: &[u64],
    b: &[u64],
    scratch: &mut [u64],
    k: B,
    threshold: usize,
) {
    let (ha, hb) = (a.len() / 2, b.len() / 2);
    let (tmp, scratch) = scratch.split_at_mut(a.len() - ha + b.len() - hb);
    out.fill(0);
    for (i, x) in [(0, &a[..ha]), (ha, &a[ha..])] {
        for (j, y) in [(0, &b[..hb]), (hb, &b[hb..])] {
            let tmp = &mut tmp[..x.len() + y.len()];
            mul_with(tmp, x, y, scratch, k, threshold);
            // The partial sums never exceed `a * b`, so nothing carries out.
            let carry = mul::add_assign(&mut out[i + j..], tmp);
            debug_assert!(!carry);
        }
    }
}

/// `out = a * b` by transforms of up to `L = 2^log_len(a.len() + b.len())` points, which
/// the tables must serve: one cyclic transform of `L` points per prime, or the shorter
/// pieces of a [`Plan`]. When `a` and `b` are the same slice, squares: one forward
/// transform per prime instead of two. So does a prepared `b`, whose transforms `fb`
/// holds; it needs `log_len(a.len() + b.len()) <= fb.log`. Squaring a prepared `b` takes
/// no forward transform.
fn transform<B: Backend>(
    out: &mut [u64],
    a: &[u64],
    b: &[u64],
    scratch: &mut [u64],
    k: B,
    fb: Option<&Prepared>,
) {
    let plan = Plan::new(a.len() + b.len());
    if !plan.nega().is_empty() {
        return pieces(out, a, b, scratch, k, &plan, fb);
    }
    let n = plan.cyc;
    let (r, rest) = B::elems(scratch).split_at_mut(B::PRIMES * n);
    let tmp = &mut rest[..n];
    for (j, r) in r.chunks_exact_mut(n).enumerate() {
        match fb {
            Some(f) if ptr::eq(a, b) => r.copy_from_slice(&B::spectrum(f, j)[..n]),
            _ => {
                k.load(j, r, a);
                k.forward(j, r);
            }
        }
        if ptr::eq(a, b) {
            k.square(j, r);
        } else if let Some(f) = fb {
            k.pointwise(j, r, &B::spectrum(f, j)[..n]);
        } else {
            k.load(j, tmp, b);
            k.forward(j, tmp);
            k.pointwise(j, r, tmp);
        }
        k.inverse(j, r, tmp);
    }
    // Only the first `2 * out.len() <= n` coefficients can be non-zero.
    let m = (2 * out.len()).next_multiple_of(4);
    k.finish(out, r, n, m, false);
}

/// The transforms for a product of `len` limbs: modulo `x^m + 1` for each `m` in
/// [`Plan::nega`] (descending), and modulo `x^cyc - 1` with `cyc` below all of them.
/// The moduli are coprime: `x^m + 1` vanishes exactly at the elements of order `2 m`,
/// and those of `x^cyc - 1` have order at most `cyc`.
///
/// With `p = 2 len` coefficients rounded up to a multiple of 8 and `t` the largest power
/// of two not above it: `x^t + 1` and a cyclic rest when that rest is shorter than `t`
/// (`p <= 3 t / 2`); else `x^t + 1`, `x^(t/2) + 1` and a rest shorter than `t / 2`
/// (`p <= 7 t / 4`); else one cyclic transform of `L = 2 t` points. More pieces cost
/// more passes than the padding they save (measured). The rest is a power of two of at
/// least 64 points: folding an operand into fewer, and summing the larger piece's blocks
/// into them in `pair` or [`crt()`], are long chains of dependent additions. An 8-point
/// rest made products just above a power of two about 10% slower (measured).
struct Plan {
    nega: [usize; 2],
    k: usize,
    cyc: usize,
}

impl Plan {
    fn new(len: usize) -> Self {
        let p = (2 * len).next_multiple_of(8);
        let top = 1 << p.ilog2();
        let cyc = |r: usize| r.next_power_of_two().max(64);
        if p > top && cyc(p - top) < top {
            return Plan {
                nega: [top, 0],
                k: 1,
                cyc: cyc(p - top),
            };
        }
        if p > top + top / 2 && cyc(p - top - top / 2) < top / 2 {
            return Plan {
                nega: [top, top / 2],
                k: 2,
                cyc: cyc(p - top - top / 2),
            };
        }
        Plan {
            nega: [0; 2],
            k: 0,
            cyc: p.next_power_of_two(),
        }
    }

    fn nega(&self) -> &[usize] {
        &self.nega[..self.k]
    }

    /// Coefficients the pieces cover: at least `2 len`, at most `L`.
    fn width(&self) -> usize {
        self.nega().iter().sum::<usize>() + self.cyc
    }
}

/// `out = a * b` from the pieces of `plan`, which has some `nega`: for each prime, `c mod
/// (x^m + 1)` for each `m` there and `c mod (x^cyc - 1)`, with `c` the product of the
/// coefficient polynomials, put together by [`crt()`]. Squares, and takes a prepared
/// `b`'s transforms from `fb`, as [`transform`] does.
///
/// Modulo `x^m + 1` is a cyclic transform of `m` points after multiplying coefficient
/// `i` by `w^i`, for `w` a primitive `2 m`-th root of unity: the twiddles of the top
/// stage of a `2 m`-point transform, so the tables must serve `2 m <= L` points.
///
/// An operand has fewer than `2 m` coefficients for the first `m` in `nega`, so [`fold`]
/// adds at most three blocks to a negacyclic piece. It can span thousands of blocks of
/// the cyclic rest, which [`sums`] folds once for all primes.
fn pieces<B: Backend>(
    out: &mut [u64],
    a: &[u64],
    b: &[u64],
    scratch: &mut [u64],
    k: B,
    plan: &Plan,
    fb: Option<&Prepared>,
) {
    let width = plan.width();
    debug_assert!(2 * out.len() <= width);
    let (sa, scratch) = scratch.split_at_mut(plan.cyc);
    let (sb, scratch) = scratch.split_at_mut(plan.cyc);
    let sa = sums(sa, a);
    let sb = if ptr::eq(a, b) || fb.is_some() {
        None
    } else {
        sums(sb, b)
    };
    let (r, tmp) = B::elems(scratch).split_at_mut(B::PRIMES * width);
    for (j, r) in r.chunks_exact_mut(width).enumerate() {
        let f = fb.map(|f| B::spectrum(f, j));
        // `x = a b` modulo `x^d + 1` (twisted, `nega`) or `x^d - 1`, for `d = x.len()`:
        // `b`'s transform is at `d..2 d` or `..d` in `f` (see [`Prepared`]).
        let mut product = |x: &mut [B::E], nega: bool| {
            // An operand's coefficients for `x`, from its sums for the cyclic rest.
            let load = |x: &mut [B::E], v: &[u64], s: Option<&[u64]>| match s {
                Some(s) if !nega => k.narrow(j, x, s),
                _ => fold(k, j, x, v, nega),
            };
            let d = x.len();
            let at = if nega { d..2 * d } else { 0..d };
            match f {
                Some(f) if ptr::eq(a, b) => x.copy_from_slice(&f[at.clone()]),
                _ => {
                    load(x, a, sa);
                    k.forward(j, x);
                }
            }
            if ptr::eq(a, b) {
                k.square(j, x);
            } else if let Some(f) = f {
                k.pointwise(j, x, &f[at]);
            } else {
                let y = &mut tmp[..x.len()];
                load(y, b, sb);
                k.forward(j, y);
                k.pointwise(j, x, y);
            }
            k.inverse(j, x, tmp);
            // With one piece, `pair` undoes the twist after the cyclic rest is ready.
            if plan.k > 1 && nega {
                k.untwist(j, x, tmp);
            }
        };
        let mut rest = &mut *r;
        for &m in plan.nega() {
            let x;
            (x, rest) = rest.split_at_mut(m);
            product(x, true);
        }
        product(rest, false);
        match *plan.nega() {
            [m] => k.pair(j, r, m, tmp),
            ref nega => crt(k, j, r, nega),
        }
    }
    // `2 out.len() <= width`, a multiple of 8.
    let len = (2 * out.len()).next_multiple_of(4);
    k.finish(out, r, width, len, false);
}

/// Writes the coefficients of `limbs` to `dst`: modulo `x^d + 1` and times `w^i` at `i`,
/// with `w` a primitive `2 d`-th root (`nega`), or modulo `x^d - 1`, for `d = dst.len()`,
/// which is even.
fn fold<B: Backend>(k: B, j: usize, dst: &mut [B::E], limbs: &[u64], nega: bool) {
    let half = dst.len() / 2;
    let (head, tail) = limbs.split_at(limbs.len().min(half));
    if nega && tail.is_empty() {
        return k.load_twist(j, dst, head);
    }
    k.load(j, dst, head);
    // Block `i + 1` is multiplied by `x^(d (i + 1))`: `(-1)^(i + 1)` modulo `x^d + 1`.
    for (i, blk) in tail.chunks(half).enumerate() {
        k.load_add(j, dst, blk, nega && i % 2 == 0);
    }
    if nega {
        k.twist(j, dst);
    }
}

/// Operands of at most this many blocks are folded per prime ([`fold`]). A [`narrow`] pass
/// takes about five times the instructions per coefficient of a `load_add` pass (NEON:
/// LLVM multiplies its 64-bit lanes in scalar registers), more than a few calls cost:
/// summing from two blocks made the 144 KiB encode 2% slower on the M5 Pro, where most
/// cyclic rests have 2 to 5.
const SUMS_BLOCKS: usize = 8;

/// The coefficients of `limbs` modulo `x^d - 1`, for `d = s.len()`, which is even, summed
/// exactly in `s`: each sum is below `2^32` times the number of blocks of `d`. `None` if
/// `limbs` spans at most [`SUMS_BLOCKS`] blocks.
fn sums<'s>(s: &'s mut [u64], limbs: &[u64]) -> Option<&'s [u64]> {
    let half = s.len() / 2;
    if limbs.len() <= SUMS_BLOCKS * half {
        return None;
    }
    s.fill(0);
    for blk in limbs.chunks(half) {
        for (s, &l) in s.as_chunks_mut::<2>().0.iter_mut().zip(blk) {
            s[0] += l & 0xFFFF_FFFF;
            s[1] += l >> 32;
        }
    }
    Some(s)
}

/// `dst[i] = s[i] mod q`, in `(-q, q)`. Requires `s[i] < 2^63`.
fn narrow(dst: &mut [i32], s: &[u64], p: Prime) {
    // `2^64 mod q` in Montgomery form: `mont(h, w, wq) = h 2^32 mod q`.
    let w = p.to_mont(p.r32 as u32);
    let wq = p.wq(w);
    for (d, &x) in dst.iter_mut().zip(s) {
        // `x = h 2^32 + l` with `h < 2^31`: `mont` gives below `0.75 q` and `load_one` at
        // most `q / 2 + 32`, so the sum is below `2^31`.
        *d = p.reduce(p.mont((x >> 32) as i32, w, wq) + portable::load_one(x as u32, p));
    }
}

/// Polynomial CRT, in place: `r` holds `c mod (x^m + 1)` for each `m` in `nega`, in that
/// order, then `c mod (x^n - 1)` for the rest, as inverse transforms leave them, where
/// `nega` is descending and `n` is below its last entry. `c` has degree below `r.len()`;
/// on return `r` holds it.
///
/// With `m` the first length and `d = r.len() - m < m`, write `c = lo + x^d mid +
/// x^m hi`, with `d` coefficients in `lo` and in `hi`. Modulo `x^m + 1` it is `lo - hi`,
/// then `mid`. Modulo each later piece `x^m = 1`, so it is `lo + hi + x^d mid`.
/// Subtracting `x^d mid` from the later residues leaves those of `lo + hi`, which the
/// recursion rebuilds in `r[m..]`. Then `lo` and `hi` are half its sum with and
/// difference from `lo - hi`.
fn crt<B: Backend>(k: B, j: usize, r: &mut [B::E], nega: &[usize]) {
    let Some((&m, later)) = nega.split_first() else {
        return;
    };
    let d = r.len() - m;
    let (e, rest) = r.split_at_mut(m);
    let (lo, mid) = e.split_at_mut(d);
    let mut tail = &mut *rest;
    for &n in later {
        let x;
        (x, tail) = tail.split_at_mut(n);
        k.fold_sub(j, x, mid, d, true);
    }
    k.fold_sub(j, tail, mid, d, false);
    crt(k, j, rest, later);
    k.halves(j, lo, rest);
}

/// The scratch limbs from the first 64-byte boundary, at most 7 limbs in, as twice as
/// many `i32`s: vector loads and stores that cross a cache line are slower.
fn as_i32(s: &mut [u64]) -> &mut [i32] {
    let skip = s.as_ptr().addr().wrapping_neg() % 64 / 8;
    let s = &mut s[skip..];
    // SAFETY: `u64` is at least as aligned as `i32`, the new slice covers the same
    // bytes, every bit pattern is a valid `i32`, and the borrow of `s` moves into it.
    unsafe { core::slice::from_raw_parts_mut(s.as_mut_ptr().cast(), 2 * s.len()) }
}

#[cfg(test)]
mod tests {
    use super::internals::{
        cached::decode_ntt_cached, decode_ntt_portable, decode_ntt_with, owned::decode_ntt,
    };
    use super::*;
    use crate::common::CHUNK;
    use crate::dc::DC_THRESHOLD;
    use crate::tests::{Decode, decode_vec, random_string};
    use crate::{dc, encode};
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    /// An [`Ntt`] with per-call tables and the thresholds for CPUs without the IFMA leaf,
    /// which the shapes in the tests below are chosen around.
    fn scalar_ntt(kernels: Kernels) -> Ntt {
        let (threshold, prepared_threshold) = (THRESHOLD, PREPARED_THRESHOLD);
        Ntt {
            threshold,
            prepared_threshold,
            ..Ntt::new_owned(kernels)
        }
    }

    /// [`mul_with`] against Karatsuba, with exactly `scratch_len` garbage-filled scratch.
    fn check(a: &[u64], b: &[u64], t: &Tables, k: &Kernels, threshold: usize) {
        let mut want = vec![0; a.len() + b.len()];
        mul::mul(
            &mut want,
            a,
            b,
            &mut vec![0; mul::scratch_len(a.len().max(b.len()))],
        );
        let mut got = vec![0xA5A5_A5A5_A5A5_A5A5; a.len() + b.len()];
        let mut scratch = vec![0x5A5A_5A5A_5A5A_5A5A; scratch_len(a.len().max(b.len()))];
        mul_with(&mut got, a, b, &mut scratch, Three { t, k }, threshold);
        assert!(
            got == want,
            "{} x {} limbs, tables 2^{}",
            a.len(),
            b.len(),
            t.log
        );
    }

    /// [`check`] for the two-prime backend, with exactly `scratch_len_two` scratch.
    fn check_two(a: &[u64], b: &[u64], t: &Tables52, k: &Kernels52, threshold: usize) {
        let mut want = vec![0; a.len() + b.len()];
        let n = a.len().max(b.len());
        mul::mul(&mut want, a, b, &mut vec![0; mul::scratch_len(n)]);
        let mut got = vec![0xA5A5_A5A5_A5A5_A5A5; a.len() + b.len()];
        let mut scratch = vec![0x5A5A_5A5A_5A5A_5A5A; scratch_len_two(n)];
        mul_with(&mut got, a, b, &mut scratch, Two { t, k }, threshold);
        let (na, nb) = (a.len(), b.len());
        assert!(
            got == want,
            "{na} x {nb} limbs, two primes, tables 2^{}",
            t.log
        );
    }

    /// The native two-prime kernels, or `None` with a skip line for `what`. Set
    /// `BASE58_REQUIRE_IFMA` to fail instead of skipping on CPUs without IFMA.
    fn native_kernels52(what: &str) -> Option<Kernels52> {
        let k = Kernels52::best();
        if k.is_none() {
            assert!(
                std::env::var_os("BASE58_REQUIRE_IFMA").is_none(),
                "no AVX-512 IFMA: {what}"
            );
            std::eprintln!("skipped: no AVX-512 IFMA ({what})");
        }
        k
    }

    /// The two-prime kernels: emulated, and native on CPUs with IFMA.
    fn kernels52() -> Vec<Kernels52> {
        let native = native_kernels52("native two-prime kernels");
        [Some(ifma::tests::emu()), native]
            .into_iter()
            .flatten()
            .collect()
    }

    /// [`scalar_ntt`] with each three-prime kernel set and each of [`kernels52`].
    fn multipliers() -> Vec<Ntt> {
        let mut v = vec![scalar_ntt(Kernels::portable()), scalar_ntt(Kernels::best())];
        for k in kernels52() {
            v.push(Ntt {
                k52: Some(k),
                ..scalar_ntt(Kernels::portable())
            });
        }
        v
    }

    /// Random, all-ones (the largest coefficients) and zero operands.
    fn operands(rng: &mut Xoshiro256PlusPlus, na: usize, nb: usize) -> [(Vec<u64>, Vec<u64>); 3] {
        let random = |rng: &mut Xoshiro256PlusPlus, n| (0..n).map(|_| rng.next_u64()).collect();
        [
            (random(rng, na), random(rng, nb)),
            (vec![u64::MAX; na], vec![u64::MAX; nb]),
            (vec![0; na], random(rng, nb)),
        ]
    }

    /// Every small shape, all through transforms. Squares pass one slice twice.
    fn small_shapes(check: impl Fn(&[u64], &[u64], usize)) {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(30);
        for na in 1..=64 {
            for nb in 1..=64 {
                for (a, b) in operands(&mut rng, na, nb) {
                    check(&a, &b, 1);
                    if na == nb {
                        check(&a, &a, 1);
                    }
                }
            }
        }
    }

    /// Shapes around the threshold, powers of two and unbalanced products, and the
    /// squares of each operand (one slice twice).
    fn large_shapes(check: impl Fn(&[u64], &[u64], usize)) {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(31);
        let square = [127, 128, 129, 255, 256, 257, 1000, 1023, 1024, 1025];
        let shapes = square.map(|n| (n, n)).into_iter().chain([
            (1, 1000),
            (7, 5000),
            (300, 700),
            (400, 700),
            (128, 4095),
            (2047, 2049),
            // Three pieces, with the longer operand wrapping modulo the first.
            (150, 290),
            (300, 590),
            // Cyclic rests of 16 and 64 coefficients that both operands span many times.
            (1020, 1034),
            (1100, 2000),
        ]);
        for (na, nb) in shapes {
            for (a, b) in operands(&mut rng, na, nb) {
                check(&a, &b, THRESHOLD);
                check(&b, &a, 1);
                check(&a, &a, 1);
                check(&b, &b, 1);
            }
        }
    }

    /// A product check: operands and threshold.
    type Check<'a> = &'a dyn Fn(&[u64], &[u64], usize);
    /// A multiplication with scratch.
    type MulFn<'a> = Box<dyn Fn(&mut [u64], &[u64], &[u64], &mut [u64]) + 'a>;
    /// A backend's scratch bound.
    type ScratchLen = fn(usize) -> usize;

    fn shapes(log: u32, k: Kernels, f: fn(Check)) {
        let t = Tables::new(log);
        f(&|a, b, threshold| check(a, b, &t, &k, threshold));
    }

    fn shapes_two(log: u32, f: fn(Check)) {
        for k in kernels52() {
            let t = Tables52::new(log, &k);
            f(&|a, b, threshold| check_two(a, b, &t, &k, threshold));
        }
    }

    #[test]
    fn small_shapes_portable() {
        shapes(8, Kernels::portable(), |c| small_shapes(c));
    }

    #[test]
    fn large_shapes_portable() {
        shapes(14, Kernels::portable(), |c| large_shapes(c));
    }

    #[test]
    fn small_shapes_best() {
        shapes(8, Kernels::best(), |c| small_shapes(c));
    }

    #[test]
    fn large_shapes_best() {
        shapes(14, Kernels::best(), |c| large_shapes(c));
    }

    #[test]
    fn small_shapes_two_primes() {
        shapes_two(8, |c| small_shapes(c));
    }

    #[test]
    fn large_shapes_two_primes() {
        shapes_two(14, |c| large_shapes(c));
    }

    /// Tables smaller than the products force the split path, recursively, for products
    /// and squares.
    #[test]
    fn split_matches_karatsuba() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(32);
        let t = Tables::new(5);
        for (na, nb) in [(5, 12), (9, 9), (16, 17), (40, 60), (100, 101), (33, 300)] {
            for (a, b) in operands(&mut rng, na, nb) {
                check(&a, &b, &t, &Kernels::portable(), 1);
                check(&a, &a, &t, &Kernels::portable(), 1);
                check(&b, &b, &t, &Kernels::portable(), 1);
            }
        }
    }

    /// Tables capped below the longest product send a real decode through [`split`], as
    /// decodes above about 16 MiB do with `MAX_LOG`.
    #[test]
    fn decode_through_split() {
        let s = random_string(&mut Xoshiro256PlusPlus::seed_from_u64(38), 100_001);
        let mut ntt = Ntt {
            max_log: 10,
            ..scalar_ntt(Kernels::best())
        };
        let mut out = vec![0; s.len()];
        let n = dc::decode_with(&s, &mut out, &mut ntt).unwrap();
        assert_eq!(Ok(out[..n].to_vec()), decode_vec(crate::dc::decode, &s));
    }

    /// The threshold path hands the scratch to Karatsuba.
    #[test]
    fn scratch_covers_karatsuba() {
        for n in 0..100_000 {
            assert!(scratch_len(n) >= mul::scratch_len(n), "n {n}");
        }
    }

    #[test]
    fn scratch_budget_and_plan_bounds() {
        assert!(scratch_len(1 << 18) * 8 <= 16 * 1024 * 1024 + 1024);
        assert!(scratch_len(286_720) <= 2_556_032);
        let mut prefix = 0;
        for len in 1..=1 << (MAX_LOG - 1) {
            let plan = Plan::new(len);
            let needed = if plan.nega().is_empty() {
                2 * plan.cyc + 7
            } else {
                2 * plan.cyc + (3 * plan.width() + plan.nega[0]) / 2 + 7
            };
            prefix = prefix.max(needed);
            let n = len.div_ceil(2);
            assert!(prefix <= scratch_len(n), "product {len}, scratch {prefix}");
            assert!(n / 2 + scratch_len(n / 2) <= scratch_len(n));
            if n >= 9 {
                assert!(n + 1 + scratch_len(n.div_ceil(2)) <= scratch_len(n));
            }
        }
    }

    #[test]
    fn no_tables_below_transform_thresholds() {
        let input = vec![0xA5; 8192];
        let mut encoded = vec![0; crate::encode::max_encoded_len(input.len())];
        let len = crate::encode_malachite(&input, &mut encoded).unwrap();
        let text = core::str::from_utf8(&encoded[..len]).unwrap();
        for cached in [false, true] {
            let mut ntt = Ntt {
                threshold: usize::MAX,
                prepared_threshold: usize::MAX,
                ..Ntt::with_cache(cached, Kernels::best())
            };
            let mut out = vec![0; input.len()];
            assert_eq!(dc::decode_with(text, &mut out, &mut ntt), Ok(input.len()));
            assert_eq!(out, input);
            assert!(ntt.owned.is_none(), "decode cached={cached}");
            #[cfg(feature = "cache")]
            assert!(ntt.shared.is_none(), "decode cached={cached}");

            let mut ntt = Ntt {
                threshold: usize::MAX,
                prepared_threshold: usize::MAX,
                ..Ntt::with_cache(cached, Kernels::best())
            };
            let mut out = vec![0; encoded.len()];
            assert_eq!(
                encode::encode_tuned(&input, &mut out, &mut ntt, 0, 32, Leaves::Horner),
                Ok(len)
            );
            assert_eq!(out[..len], encoded[..len]);
            assert!(ntt.owned.is_none(), "encode cached={cached}");
            #[cfg(feature = "cache")]
            assert!(ntt.shared.is_none(), "encode cached={cached}");
        }
    }

    #[test]
    fn reused_multiplier_grows_tables() {
        let mut ntt = scalar_ntt(Kernels::best());
        for len in [6000, 30_000] {
            let input = "z".repeat(len);
            let want = decode_vec(crate::decode_malachite, &input).unwrap();
            let mut out = vec![0; want.len()];
            assert_eq!(dc::decode_with(&input, &mut out, &mut ntt), Ok(want.len()));
            assert_eq!(out, want);
        }
    }

    /// Exactly `scratch_len` limbs serve at any alignment, and [`as_i32`] aligns them:
    /// one transform of 8 points and one of 512, two pieces and three.
    #[test]
    fn scratch_at_any_alignment() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(42);
        let (t, k) = (Tables::new(12), Kernels::best());
        for n in [1, 34, 128, 129, 200, 321] {
            let (a, b) = operands(&mut rng, n, n)[0].clone();
            let mut want = vec![0; 2 * n];
            mul::mul(&mut want, &a, &b, &mut vec![0; mul::scratch_len(n)]);
            let mut scratch = vec![0x5A5A_5A5A_5A5A_5A5A; scratch_len(n) + 7];
            for skip in 0..8 {
                let s = &mut scratch[skip..][..scratch_len(n)];
                assert_eq!(as_i32(s).as_ptr().addr() % 64, 0);
                let mut got = vec![0; 2 * n];
                mul_with(&mut got, &a, &b, s, Three { t: &t, k: &k }, 1);
                assert!(got == want, "{n} limbs, scratch {skip} limbs in");
            }
        }
    }

    /// One cyclic transform of `b` holds, mod q, every piece transform a [`Plan`] of at
    /// most its length takes from `b`: `[0, d)` is the one modulo `x^d - 1` and `[d, 2 d)`
    /// the twisted one modulo `x^d + 1`. [`Prepared`] relies on this.
    #[test]
    fn pieces_are_slices_of_one_transform() {
        use crate::ntt::tables::tests::modq;
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(39);
        let (t, k) = (Tables::new(12), Kernels::portable());
        for (j, (p, tw)) in PRIMES.into_iter().zip(&t.tw).enumerate() {
            let reduced = |x: &[i32]| x.iter().map(|&v| modq(v as i128, p.q)).collect::<Vec<_>>();
            for n in (4..=12).map(|log| 1 << log) {
                for len in [n / 8, n / 3, n / 2] {
                    let b: Vec<u64> = (0..len).map(|_| rng.next_u64()).collect();
                    let mut full = vec![0; n];
                    (k.load)(&mut full, &b, p);
                    (k.forward)(&mut full, p, tw.fw());
                    let pieces = (3..n.ilog2()).map(|log| 1 << log);
                    for (d, nega) in pieces.flat_map(|d| [(d, true), (d, false)]) {
                        let mut x = vec![0; d];
                        fold(Three { t: &t, k: &k }, j, &mut x, &b, nega);
                        (k.forward)(&mut x, p, tw.fw());
                        let at = if nega { d } else { 0 };
                        let msg = format!("n {n}, {len} limbs, d {d}, negacyclic {nega}");
                        assert_eq!(reduced(&full[at..at + d]), reduced(&x), "{msg}");
                    }
                }
            }
        }
    }

    /// [`sums`] then [`narrow`] against [`fold`] modulo `x^d - 1`, for each prime: operands
    /// from just over [`SUMS_BLOCKS`] blocks of `d` coefficients to thousands, ending
    /// inside a block or at its end, random and all-ones (the largest sums). Values in
    /// `(-q, q)`.
    #[test]
    fn sums_match_fold() {
        use crate::ntt::tables::tests::modq;
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(43);
        let k = Kernels::portable();
        let reduced = |x: &[i32], q| x.iter().map(|&v| modq(v as i128, q)).collect::<Vec<_>>();
        for d in [8, 16, 64, 256] {
            let short = vec![u64::MAX; SUMS_BLOCKS * d / 2];
            assert!(sums(&mut vec![0; d], &short).is_none(), "d {d}");
            for len in [SUMS_BLOCKS * d / 2 + 1, (SUMS_BLOCKS + 2) * d / 2, 20_001] {
                let random: Vec<u64> = (0..len).map(|_| rng.next_u64()).collect();
                for a in [random, vec![u64::MAX; len]] {
                    let mut s = vec![0; d];
                    let s = sums(&mut s, &a).unwrap();
                    for (j, p) in PRIMES.into_iter().enumerate() {
                        let (mut got, mut want) = (vec![0; d], vec![0; d]);
                        narrow(&mut got, s, p);
                        fold(
                            Three {
                                t: &Tables::new(5),
                                k: &k,
                            },
                            j,
                            &mut want,
                            &a,
                            false,
                        );
                        assert_eq!(
                            reduced(&got, p.q),
                            reduced(&want, p.q),
                            "d {d}, {len} limbs"
                        );
                        assert!(got.iter().all(|v| v.abs() < p.q), "d {d}, {len} limbs");
                    }
                }
            }
        }
    }

    /// [`narrow`] at the ends of its range, `[0, 2^63)`.
    #[test]
    fn narrow_extremes() {
        use crate::ntt::tables::tests::modq;
        let s = [
            0,
            1,
            u32::MAX as u64,
            1 << 32,
            (1 << 63) - (1 << 32),
            (1 << 63) - 1,
        ];
        for p in PRIMES {
            let mut got = [0; 6];
            narrow(&mut got, &s, p);
            for (&g, &x) in got.iter().zip(&s) {
                assert_eq!(modq(g as i128, p.q), modq(x as i128, p.q), "{x}");
                assert!(g.abs() < p.q, "{x}");
            }
        }
    }

    /// [`Multiplier::prepare_power`] then [`Multiplier::mul_power`] against Karatsuba,
    /// with exactly `scratch_len` garbage-filled scratch.
    fn check_power(ntt: &mut Ntt, i: usize, a: &[u64], b: &[u64], len: usize) {
        let mut want = vec![0; a.len() + b.len()];
        mul::mul(
            &mut want,
            a,
            b,
            &mut vec![0; mul::scratch_len(a.len().max(b.len()))],
        );
        ntt.prepare_power(i, b, len);
        let mut got = vec![0xA5A5_A5A5_A5A5_A5A5; a.len() + b.len()];
        let mut scratch = vec![0x5A5A_5A5A_5A5A_5A5A; ntt.scratch_len(a.len().max(b.len()))];
        ntt.mul_power(&mut got, a, i, b, &mut scratch);
        assert!(got == want, "{} x {} limbs, len {len}", a.len(), b.len());
    }

    /// The inverse transform at the largest input magnitudes, for odd and even `log2(n)`:
    /// no `i32` overflow (the dev profile checks the portable kernels), outputs in
    /// `(-q, q)`, and `forward(inverse(x)) = n x mod q`.
    #[test]
    fn inverse_extremes() {
        use crate::ntt::tables::tests::modq;
        let t = Tables::new(14);
        for k in [Kernels::portable(), Kernels::best()] {
            for (p, tw) in PRIMES.into_iter().zip(&t.tw) {
                let m = p.q - 1;
                for n in (3..=14).map(|log| 1 << log) {
                    let mut iw = vec![0; n];
                    let iw = tw.inverse(&mut iw);
                    let mixed: Vec<i32> = (0..n).map(|i| if i % 3 == 0 { m } else { -m }).collect();
                    for x in [vec![m; n], vec![-m; n], mixed] {
                        let mut a = x.clone();
                        (k.inverse)(&mut a, p, iw);
                        assert!(a.iter().all(|v| v.abs() < p.q), "n {n}");
                        (k.forward)(&mut a, p, tw.fw());
                        for (&got, &x) in a.iter().zip(&x) {
                            let want = modq(x as i128 * n as i128, p.q);
                            assert_eq!(modq(got as i128, p.q), want, "n {n}");
                        }
                    }
                }
            }
        }
    }

    /// Products with a prepared power transform only `a`: one forward transform per piece
    /// and prime, for one cyclic transform, two pieces and three.
    #[test]
    fn prepared_powers_skip_transforms() {
        use core::cell::Cell;
        thread_local!(static FORWARDS: Cell<usize> = const { Cell::new(0) });
        let counting = Kernels {
            forward: |a, p, t| {
                FORWARDS.with(|c| c.set(c.get() + 1));
                portable::forward(a, p, t)
            },
            ..Kernels::portable()
        };
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(41);
        let mut ntt = scalar_ntt(counting);
        ntt.prepare(4000);
        let shapes = [
            (1031, 858, 1889, 1),
            (600, 500, 4000, 2),
            (800, 900, 2000, 3),
        ];
        for (i, (na, nb, len, pieces)) in shapes.into_iter().enumerate() {
            let (a, b) = operands(&mut rng, na, nb)[0].clone();
            ntt.prepare_power(i, &b, len);
            FORWARDS.with(|c| c.set(0));
            let mut out = vec![0; na + nb];
            ntt.mul_power(&mut out, &a, i, &b, &mut vec![0; scratch_len(na.max(nb))]);
            assert_eq!(FORWARDS.with(Cell::get), 3 * pieces, "{na} x {nb} limbs");
        }
        // Squares of a prepared power take no forward transform: one cyclic transform,
        // two pieces and three.
        for (i, n) in [128, 129, 200].into_iter().enumerate() {
            let b: Vec<u64> = (0..n).map(|_| rng.next_u64()).collect();
            ntt.prepare_power(shapes.len() + i, &b, 2 * n);
            FORWARDS.with(|c| c.set(0));
            let mut out = vec![0; 2 * n];
            let slot = shapes.len() + i;
            ntt.mul_power(&mut out, &b, slot, &b, &mut vec![0; scratch_len(n)]);
            assert_eq!(FORWARDS.with(Cell::get), 0, "{n} limbs");
            let mut want = vec![0; 2 * n];
            mul::mul(&mut want, &b, &b, &mut vec![0; mul::scratch_len(n)]);
            assert_eq!(out, want, "{n} limbs");
        }
        let slots = ntt.powers.len();
        ntt.prepared_threshold = usize::MAX;
        ntt.prepare_power(1, &[], 0);
        assert!(
            ntt.powers[1].is_none(),
            "skipping must clear an existing spectrum"
        );
        ntt.prepare_power(slots + 5, &[], 0);
        assert_eq!(ntt.powers.len(), slots, "skipped powers need no new slots");
    }

    /// Prepared powers: one cyclic transform as stored, a prefix of a longer one, one and
    /// two negacyclic pieces, operands wrapping modulo the cyclic piece, a power or an
    /// operand below the threshold, a `len` too short for the product, and an operand
    /// other than the prepared power. Each gives the exact product.
    #[test]
    fn prepared_powers_match_karatsuba() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(40);
        // (a, b, len), with `b <= len` as `dc` promises.
        let shapes = [
            // One 4096-point transform; then the prefix of an 8192-point one.
            (1031, 858, 1889),
            (1031, 858, 3778),
            // x^2048 + 1 and x^256 - 1, from a stored 8192-point transform.
            (600, 500, 4000),
            // x^2048 + 1, x^1024 + 1 and x^512 - 1.
            (800, 900, 2000),
            // x^4096 + 1 and x^512 - 1, with `b`, then `a`, wrapping modulo the latter.
            (2000, 300, 2300),
            (300, 2000, 4000),
            // Below the threshold, and a product too long for the stored transform.
            (5, 858, 1889),
            (130, 109, 239),
            (1500, 858, 1716),
        ];
        for mut ntt in multipliers() {
            ntt.prepare(4000);
            let mut i = 0;
            for (na, nb, len) in shapes {
                for (a, b) in operands(&mut rng, na, nb) {
                    check_power(&mut ntt, i, &a, &b, len);
                    i += 1;
                }
            }
            // An operand other than the prepared power, under its index.
            let (a, b, c) = (vec![u64::MAX; 1031], vec![u64::MAX; 858], vec![1; 858]);
            ntt.prepare_power(i, &b, 1889);
            let mut got = vec![0; a.len() + c.len()];
            ntt.mul_power(&mut got, &a, i, &c, &mut vec![0; ntt.scratch_len(a.len())]);
            let mut want = vec![0; a.len() + c.len()];
            mul::mul(&mut want, &a, &c, &mut vec![0; mul::scratch_len(a.len())]);
            assert!(got == want, "operand other than the prepared power");
        }
    }

    /// `mul_wrapped` and `mul_power_wrapped` against the full product folded modulo
    /// `β^L - 1`: operands shorter than, as long as and longer than `L` (folded first), a
    /// prepared `b` whose stored transform is longer than `2 L` points, both kernel sets.
    #[test]
    fn wrapped_matches_folded_product() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(42);
        // (L, a, b)
        let shapes = [
            (4, 1, 1),
            (4, 9, 3),
            (8, 8, 8),
            (64, 63, 2),
            (256, 300, 200),
            (512, 1100, 512),
            (2048, 2500, 1100),
            (2048, 1, 2048),
        ];
        // Some value in [0, β^L - 1]: both ends stand for zero.
        let canon = |mut v: Vec<u64>| {
            if v.iter().all(|&x| x == u64::MAX) {
                v.fill(0);
            }
            v
        };
        for mut ntt in multipliers() {
            ntt.prepare(8192);
            let mut i = 0;
            for (l, na, nb) in shapes {
                for (a, b) in operands(&mut rng, na, nb) {
                    let mut full = vec![0; na + nb];
                    mul::mul(
                        &mut full,
                        &a,
                        &b,
                        &mut vec![0; mul::scratch_len(na.max(nb))],
                    );
                    let mut want = vec![0; l];
                    mul::fold_wrapped(&mut want, &full);
                    let want = canon(want);
                    // Exactly `len` limbs, 8 bytes past a 64-byte boundary: [`as_i32`]
                    // skips 7 of them.
                    let len = ntt.wrapped_scratch_len(l, na.max(nb));
                    let mut buf = vec![0x5A5A_5A5A_5A5A_5A5A; len + 7];
                    let skip = 8usize.wrapping_sub(buf.as_ptr().addr()) % 64 / 8;
                    let s = &mut buf[skip..][..len];
                    let mut got = vec![0xA5A5_A5A5_A5A5_A5A5; l];
                    ntt.mul_wrapped(&mut got, &a, &b, s);
                    assert!(canon(got) == want, "{l}: {na} x {nb}");
                    // An 8192-point stored transform: its prefix serves 2 L points.
                    ntt.prepare_power(i, &b, 4096);
                    assert_eq!(
                        ntt.powers.get(i).is_some_and(Option::is_some),
                        nb >= ntt.prepared_threshold
                    );
                    let mut got = vec![0xA5A5_A5A5_A5A5_A5A5; l];
                    s.fill(0x5A5A_5A5A_5A5A_5A5A);
                    ntt.mul_power_wrapped(&mut got, &a, i, &b, s);
                    assert!(canon(got) == want, "{l}: {na} x {nb}, prepared");
                    i += 1;
                }
            }
        }
    }

    /// One slice twice never calls `pointwise`, and two equal slices never call
    /// `square`: through one transform, two pieces and three.
    #[test]
    fn squares_take_square_path() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(37);
        let t = Tables::new(14);
        let no_pointwise = Kernels {
            pointwise: |_, _, _| panic!("pointwise"),
            ..Kernels::best()
        };
        let no_square = Kernels {
            square: |_, _| panic!("square"),
            ..Kernels::best()
        };
        for (n, nega) in [(128, 0), (129, 1), (200, 2)] {
            assert_eq!(Plan::new(2 * n).nega().len(), nega);
            let a: Vec<u64> = (0..n).map(|_| rng.next_u64()).collect();
            check(&a, &a, &t, &no_pointwise, 1);
            check(&a, &a.clone(), &t, &no_square, 1);
        }
    }

    /// Large inputs, where bs58 is too slow to be the reference; `decode` (Karatsuba)
    /// is checked against bs58 and `decode_batched`. The thresholds of CPUs with and
    /// without the IFMA leaf are both checked on every CPU.
    #[test]
    fn decoders_match_karatsuba() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(33);
        let decoders: [(&str, Decode); 5] = [
            ("ntt", decode_ntt),
            ("ntt-cached", decode_ntt_cached),
            ("ntt-portable", decode_ntt_portable),
            ("scalar thresholds", |s, o| {
                decode_ntt_with(s, o, true, THRESHOLD, PREPARED_THRESHOLD)
            }),
            ("IFMA thresholds", |s, o| {
                decode_ntt_with(s, o, true, IFMA_THRESHOLD, IFMA_PREPARED_THRESHOLD)
            }),
        ];
        let t = DC_THRESHOLD * CHUNK;
        for len in [t + 1, 2 * t + 1, 10_000, 40_960, 100_001, 1_000_000] {
            // 58^len - 1 maximises carries.
            let max = "z".repeat(len);
            let min = format!("2{}", "1".repeat(len - 1));
            let zeros = format!("111{max}");
            for s in [random_string(&mut rng, len), max, min, zeros] {
                let want = decode_vec(crate::dc::decode, &s);
                for (name, f) in decoders {
                    assert!(decode_vec(f, &s) == want, "{name}, len {len}");
                }
            }
        }
    }

    /// `prepare` sizes the tables for the longest product a decode makes: never smaller,
    /// and at the benchmark sizes (32 KiB, 128 KiB, 1 MiB decoded) not larger. Uses
    /// per-call tables so the cache stays cold for `cached_tables_across_threads`.
    #[test]
    fn tables_fit_longest_product() {
        struct Recording {
            ntt: Ntt,
            prepared: u32,
            used: u32,
            /// Points of all prepared power transforms (each 3 primes × 4 bytes).
            power_points: usize,
        }

        impl Multiplier for Recording {
            fn prepare(&mut self, len: usize) {
                self.ntt.prepare(len);
                self.prepared = self.ntt.log.unwrap();
            }

            fn scratch_len(&self, n: usize) -> usize {
                self.ntt.scratch_len(n)
            }

            fn mul(&mut self, out: &mut [u64], a: &[u64], b: &[u64], scratch: &mut [u64]) {
                self.used = self.used.max(log_len(a.len() + b.len()));
                self.ntt.mul(out, a, b, scratch);
            }

            // The power's transforms are for products of `len` limbs.
            fn prepare_power(&mut self, i: usize, b: &[u64], len: usize) {
                self.used = self.used.max(log_len(len));
                self.ntt.prepare_power(i, b, len);
                if let Some(Some(f)) = self.ntt.powers.get(i) {
                    self.power_points += 1 << f.log;
                }
            }

            fn mul_power(
                &mut self,
                out: &mut [u64],
                a: &[u64],
                i: usize,
                b: &[u64],
                s: &mut [u64],
            ) {
                self.used = self.used.max(log_len(a.len() + b.len()));
                self.ntt.mul_power(out, a, i, b, s);
            }
        }

        let mut rng = Xoshiro256PlusPlus::seed_from_u64(35);
        let chars = |bytes: usize| (8.0 * bytes as f64 / 58f64.log2()) as usize;
        let exact = [32 << 10, 128 << 10, 1 << 20].map(chars);
        let others = (0..24).map(|i| (5_200.0 * 1.25f64.powi(i)) as usize);
        for len in exact.into_iter().chain(others) {
            let s = random_string(&mut rng, len);
            let ntt = Ntt::new_owned(Kernels::best());
            let mut m = Recording {
                ntt,
                prepared: 0,
                used: 0,
                power_points: 0,
            };
            dc::decode_with(&s, &mut vec![0; len], &mut m).unwrap();
            assert!(m.prepared >= m.used, "len {len}: tables too small");
            // Prepared powers take less than 12 L bytes while the longest product
            // fits the tables.
            assert!(
                m.power_points < 1 << m.prepared,
                "len {len}: prepared powers too large"
            );
            if exact.contains(&len) {
                assert_eq!(m.prepared, m.used, "len {len}: tables too large");
            }
        }
    }

    /// Threads racing to use the largest cacheable table all get one correct table.
    #[cfg(feature = "cache")]
    #[test]
    fn cached_tables_across_threads() {
        const LOG: u32 = CACHE_MAX_LOG;
        let start = std::sync::Barrier::new(8);
        let got: Vec<&Tables> = std::thread::scope(|scope| {
            let threads: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        start.wait();
                        Tables::cached(LOG)
                    })
                })
                .collect();
            threads.into_iter().map(|t| t.join().unwrap()).collect()
        });
        assert!(
            got.iter().all(|&t| core::ptr::eq(t, got[0])),
            "two tables built"
        );
        assert_eq!(got[0].log, LOG);
        for (g, w) in got[0].tw.iter().zip(&Tables::new(LOG).tw) {
            assert!(g.fw() == w.fw() && g.iw() == w.iw());
        }
    }

    /// All-ones operands give the largest coefficients a 2^20-point transform can see.
    /// One slice twice squares; a copy takes the general path.
    #[test]
    fn large_all_ones() {
        let n = 1 << 18;
        assert_eq!(log_len(2 * n), 20);
        let (a, t) = (vec![u64::MAX; n], Tables::new(20));
        check(&a, &a, &t, &Kernels::portable(), THRESHOLD);
        check(&a, &a.clone(), &t, &Kernels::portable(), THRESHOLD);
    }

    /// Transforms of `2^MAX_LOG` points, the most the tables allow, on all-ones operands
    /// (the largest coefficients), for each kind of [`Plan`], as products and, for equal
    /// lengths, as squares. The closed form
    /// `(X^a - 1)(X^b - 1) = (X^(a+b) - 1) - (X^a - 1) - (X^b - 1)`, for `X = 2^64`,
    /// replaces a reference product. Takes seconds and about 1 GB, so it is ignored by
    /// default: run it with `cargo test -- --ignored`.
    #[test]
    #[ignore = "Exercises maximum-size transforms; run separately in the maximum_transform CI job"]
    fn largest_transforms() {
        let t = Tables::new(MAX_LOG);
        let m = 1 << 20;
        let shapes = [
            // One cyclic transform of 2^23 points, full.
            (2 * m, 2 * m),
            // x^(2^22) + 1, and x^64 - 1 or x^(2^21) - 1.
            (m, m + 4),
            (3 * m / 2, 3 * m / 2),
            // x^(2^22) + 1, x^(2^21) + 1 and a cyclic rest; then with `b` wrapping
            // modulo x^(2^22) + 1.
            (3 * m / 2, 3 * m / 2 + 4),
            (m + 1000, 2 * m + 1000),
            // One cyclic transform of 2^23 points, a tenth of it padding.
            (9 * m / 5, 9 * m / 5),
        ];
        // Two primes only with native IFMA: emulated, these would take minutes.
        let k52 = native_kernels52("largest_transforms, two primes");
        let t52 = k52.as_ref().map(|k| Tables52::new(MAX_LOG, k));
        // Each backend with exactly its own scratch bound.
        let mut backends: Vec<(&str, MulFn, ScratchLen)> = vec![];
        for k in [Kernels::portable(), Kernels::best()] {
            let t = &t;
            backends.push((
                "three primes",
                Box::new(move |o, a, b, s| mul_with(o, a, b, s, Three { t, k: &k }, THRESHOLD)),
                scratch_len,
            ));
        }
        if let (Some(k), Some(t)) = (&k52, &t52) {
            backends.push((
                "two primes",
                Box::new(move |o, a, b, s| mul_with(o, a, b, s, Two { t, k }, THRESHOLD)),
                scratch_len_two,
            ));
        }
        for (name, mul_with, scratch_len) in &backends {
            for (na, nb) in shapes {
                assert_eq!(log_len(na + nb), MAX_LOG);
                let (a, b) = (vec![u64::MAX; na], vec![u64::MAX; nb]);
                let mut want = vec![u64::MAX; na + nb];
                let borrow = mul::sub_assign(&mut want, &a) | mul::sub_assign(&mut want, &b);
                assert!(!borrow);
                let mut got = vec![0; na + nb];
                let mut scratch = vec![0; scratch_len(nb)];
                mul_with(&mut got, &a, &b, &mut scratch);
                assert!(got == want, "{name}: {na} x {nb} limbs");
                if na == nb {
                    // One slice twice: the square path.
                    got.fill(0);
                    mul_with(&mut got, &a, &a, &mut scratch);
                    assert!(got == want, "{name}: {na} limbs squared");
                }
            }
        }
    }

    /// The same through [`pieces`]: two (`2^20 + 2^18` points) and three
    /// (`2^20 + 2^19 + 2^16`).
    #[test]
    fn large_all_ones_pieces() {
        let t = Tables::new(21);
        let shapes: [(usize, &[usize], usize); 2] = [
            (327_680, &[1 << 20], 1 << 18),
            (409_600, &[1 << 20, 1 << 19], 1 << 16),
        ];
        for (n, nega, cyc) in shapes {
            let plan = Plan::new(2 * n);
            assert_eq!((log_len(2 * n), plan.nega(), plan.cyc), (21, nega, cyc));
            let a = vec![u64::MAX; n];
            check(&a, &a, &t, &Kernels::best(), THRESHOLD);
            check(&a, &a.clone(), &t, &Kernels::best(), THRESHOLD);
        }
    }

    /// The pieces cover the product, are coprime and fit the tables, and pad little.
    #[test]
    fn plan_covers_product() {
        for len in 1..=1 << 14 {
            let (plan, l) = (Plan::new(len), 1 << log_len(len));
            let w = plan.width();
            assert!(
                w >= 2 * len && w <= l && (plan.nega().is_empty() || w < l),
                "len {len}"
            );
            // Descending powers of two, at least 8, the cyclic one last and smallest, and
            // at least 64 next to negacyclic ones.
            let mut all = plan.nega().to_vec();
            all.push(plan.cyc);
            assert!(
                all.iter().all(|&x| x.is_power_of_two() && x >= 8),
                "len {len}"
            );
            assert!(all.windows(2).all(|x| x[0] > x[1]), "len {len}");
            assert!(plan.nega().is_empty() || plan.cyc >= 64, "len {len}");
            assert!(plan.nega().iter().all(|&m| 2 * m <= l), "len {len}");
            // Rounding to three bits adds less than a quarter. Up to 128 limbs, which only
            // tests multiply by transforms, the 64-point rest can add more.
            let pieces = !plan.nega().is_empty() && len > 128;
            assert!(4 * w < 5 * (2 * len + 8) || !pieces, "len {len}");
        }
    }

    /// Products of every length from a few limbs below `2^k` and `1.5·2^k` limbs to 36
    /// above, and squares of half of each even length: every small cyclic piece, for two
    /// pieces and three, and the step from the smallest piece to the next.
    #[test]
    fn rests_around_powers_of_two() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(43);
        let (t, k) = (Tables::new(14), Kernels::best());
        for top in [128, 1024, 4096] {
            for len in (top - 4..=top + 36).chain(3 * top / 2 - 4..=3 * top / 2 + 36) {
                let na = len * 11 / 20;
                for (a, b) in &operands(&mut rng, na, len - na)[..2] {
                    check(a, b, &t, &k, 1);
                }
                if len % 2 == 0 {
                    for (a, _) in &operands(&mut rng, len / 2, 1)[..2] {
                        check(a, a, &t, &k, 1);
                    }
                }
            }
        }
    }

    /// [`crt`] against reducing random polynomials directly, for every shape of plan.
    #[test]
    fn crt_reconstructs_polynomial() {
        use crate::ntt::tables::tests::{modq, samples};
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(36);
        let shapes: [(&[usize], usize); 6] = [
            (&[16], 8),
            (&[64], 8),
            (&[64], 32),
            (&[1024], 256),
            (&[64, 32], 8),
            (&[1024, 128], 32),
        ];
        for k in [Kernels::portable(), Kernels::best()] {
            for (p, (nega, cyc)) in PRIMES.into_iter().flat_map(|p| shapes.map(|s| (p, s))) {
                let len = nega.iter().sum::<usize>() + cyc;
                let c = &samples(&mut rng, p.q, len)[..len];
                let mut r = vec![0i64; len];
                let mut at = 0;
                for &m in nega.iter().chain([&cyc]) {
                    for (i, &x) in c.iter().enumerate() {
                        let neg = at < len - cyc && (i / m) % 2 == 1;
                        r[at + i % m] += if neg { -x as i64 } else { x as i64 };
                    }
                    at += m;
                }
                let r: Vec<i32> = r.iter().map(|&x| (x % p.q as i64) as i32).collect();
                let mut got = r.clone();
                let j = PRIMES.iter().position(|x| x.q == p.q).unwrap();
                crt(
                    Three {
                        t: &Tables::new(5),
                        k: &k,
                    },
                    j,
                    &mut got,
                    nega,
                );
                let mut all = vec![got];
                if let &[m] = nega {
                    // `pair` takes the negacyclic residue times `w^i` at `i`.
                    let (t, q) = (Tables::new(m.ilog2() + 1), p.q as i64);
                    let tw = &t.tw[PRIMES.iter().position(|x| x.q == p.q).unwrap()];
                    let mut r = r.clone();
                    let one = p.to_mont(1) as i64;
                    for (i, x) in r[..m].iter_mut().enumerate() {
                        // `fw[m + i]` is `w^i` in Montgomery form; `one` strips the `2^32`.
                        let w = tw.fw()[m + i] as i64 * inverse(one, q) % q;
                        *x = (*x as i64 * w % q) as i32;
                    }
                    let mut iw = vec![0; m];
                    tables::invert_roots(&mut iw, &tw.fw()[m..2 * m]);
                    (k.pair)(&mut r, m, &iw, p);
                    all.push(r);
                }
                for got in all {
                    for (i, (&got, &want)) in got.iter().zip(c).enumerate() {
                        assert_eq!(
                            modq(got as i128, p.q),
                            modq(want as i128, p.q),
                            "{nega:?} {i}"
                        );
                        assert!(got.abs() < p.q);
                    }
                }
            }
        }
    }

    /// `1 / x mod q`.
    fn inverse(x: i64, q: i64) -> i64 {
        crate::ntt::tables::tests::pow(x.rem_euclid(q) as u64, q as u64 - 2, q as u64) as i64
    }

    /// [`split_matches_karatsuba`] with two primes.
    #[test]
    fn two_primes_split_matches_karatsuba() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(32);
        for k in kernels52() {
            let t = Tables52::new(5, &k);
            for (na, nb) in [(5, 12), (9, 9), (16, 17), (40, 60), (100, 101), (33, 300)] {
                for (a, b) in operands(&mut rng, na, nb) {
                    check_two(&a, &b, &t, &k, 1);
                    check_two(&a, &a, &t, &k, 1);
                    check_two(&b, &b, &t, &k, 1);
                }
            }
        }
    }

    /// [`scratch_at_any_alignment`] with two primes: exactly `scratch_len_two` limbs.
    #[test]
    fn two_primes_scratch_at_any_alignment() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(42);
        for k in kernels52() {
            let t = Tables52::new(12, &k);
            for n in [1, 34, 128, 129, 200, 321] {
                let (a, b) = operands(&mut rng, n, n)[0].clone();
                let mut want = vec![0; 2 * n];
                mul::mul(&mut want, &a, &b, &mut vec![0; mul::scratch_len(n)]);
                let len = scratch_len_two(n);
                let mut scratch = vec![0x5A5A_5A5A_5A5A_5A5A; len + 7];
                for skip in 0..8 {
                    let s = &mut scratch[skip..][..len];
                    assert_eq!(Two::elems(s).as_ptr().addr() % 64, 0);
                    let mut got = vec![0; 2 * n];
                    mul_with(&mut got, &a, &b, s, Two { t: &t, k: &k }, 1);
                    assert!(got == want, "{n} limbs, scratch {skip} limbs in");
                }
            }
        }
    }

    /// [`squares_take_square_path`] with two primes.
    #[test]
    fn two_primes_squares_take_square_path() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(37);
        for k in kernels52() {
            let t = Tables52::new(14, &k);
            let no_pointwise = Kernels52 {
                pointwise: |_, _, _| panic!("pointwise"),
                ..k
            };
            let no_square = Kernels52 {
                square: |_, _| panic!("square"),
                ..k
            };
            for (n, nega) in [(128, 0), (129, 1), (200, 2)] {
                assert_eq!(Plan::new(2 * n).nega().len(), nega);
                let a: Vec<u64> = (0..n).map(|_| rng.next_u64()).collect();
                check_two(&a, &a, &t, &no_pointwise, 1);
                check_two(&a, &a.clone(), &t, &no_square, 1);
            }
        }
    }

    /// Decoding and division-tree encoding end to end through the two-prime backend, with
    /// the thresholds of [`scalar_ntt`] so moderate lengths take transforms, prepared
    /// powers and wrapped products. Lengths in characters (decode) and bytes (encode).
    #[test]
    fn two_primes_convert_end_to_end() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(44);
        for k in kernels52() {
            for len in [DC_THRESHOLD * CHUNK + 1, 10_000, 40_960] {
                let max = "z".repeat(len);
                for s in [random_string(&mut rng, len), max] {
                    let want = decode_vec(crate::dc::decode, &s).unwrap();
                    let mut ntt = Ntt {
                        k52: Some(k),
                        ..scalar_ntt(Kernels::portable())
                    };
                    let mut out = vec![0; s.len()];
                    let n = dc::decode_with(&s, &mut out, &mut ntt).unwrap();
                    assert!(out[..n] == want[..], "decode, len {len}");
                    assert!(ntt.owned.is_none() && ntt.owned52.is_some());

                    let mut ntt = Ntt {
                        k52: Some(k),
                        ..scalar_ntt(Kernels::portable())
                    };
                    let mut text = vec![0; crate::encode::max_encoded_len(want.len())];
                    let n = encode::encode_tuned(&want, &mut text, &mut ntt, 0, 32, Leaves::Horner)
                        .unwrap();
                    assert!(text[..n] == *s.as_bytes(), "encode, len {len}");
                    assert!(ntt.owned.is_none() && ntt.owned52.is_some());
                }
            }
        }
    }

    /// [`cached_tables_across_threads`] for the two-prime store, from emulated kernels:
    /// one table, equal to an owned one.
    #[cfg(feature = "cache")]
    #[test]
    fn two_primes_cached_tables_across_threads() {
        const LOG: u32 = CACHE_MAX_LOG;
        let k = ifma::tests::emu();
        let start = std::sync::Barrier::new(8);
        let got: Vec<&Tables52> = std::thread::scope(|scope| {
            let threads: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        start.wait();
                        Tables52::cached(LOG, &k)
                    })
                })
                .collect();
            threads.into_iter().map(|t| t.join().unwrap()).collect()
        });
        assert!(
            got.iter().all(|&t| core::ptr::eq(t, got[0])),
            "two tables built"
        );
        assert_eq!(got[0].log, LOG);
        let owned = Tables52::new(LOG, &k);
        for (g, w) in got[0].tw.iter().zip(&owned.tw) {
            assert!(g.roots_eq(w));
        }
    }
}
