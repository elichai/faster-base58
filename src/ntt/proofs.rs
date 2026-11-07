//! Full-width scalar NTT obligations. These do not model SIMD instructions.

use super::{
    Plan, portable,
    tables::{MAX_LOG, PRIMES},
};

/// `x ± mont(y, w)` for any twiddle `|w| <= q / 2`, after reducing `x` when `rx`.
fn butterfly(index: usize, x: i32, y: i32, rx: bool) -> [i64; 2] {
    let p = PRIMES[index];
    let w: i32 = kani::any();
    kani::assume(-(p.q / 2) <= w && w <= p.q / 2);
    let x = if rx { p.reduce(x) } else { x } as i64;
    let t = p.mont(y, w, p.wq(w)) as i64;
    [x + t, x - t]
}

macro_rules! prime_proofs {
    ($reduce:ident, $normalize:ident, $load:ident, $scale:ident, $rx:ident, $lazy:ident, $index:expr) => {
        /// A stage of `inverse` that reduces `x` (`portable::reduces_x`) takes values
        /// below `2 q` and stores values below `1.25 q`.
        #[kani::proof]
        fn $rx() {
            let q = PRIMES[$index].q;
            let (x, y): (i32, i32) = (kani::any(), kani::any());
            kani::assume(-2 * q < x && x < 2 * q && -2 * q < y && y < 2 * q);
            let bound = q as i64 + q as i64 / 4;
            let [s, d] = butterfly($index, x, y, true);
            assert!(-bound < s && s < bound && -bound < d && d < bound);
        }

        /// A stage of `inverse` that does not reduce takes values below `1.25 q` and
        /// stores values below `1.9 q`, which is below `2 q` and `2^31`.
        #[kani::proof]
        fn $lazy() {
            let q = PRIMES[$index].q;
            let bound = q + q / 4;
            let (x, y): (i32, i32) = (kani::any(), kani::any());
            kani::assume(-bound < x && x < bound && -bound < y && y < bound);
            let out = 19 * q as i64 / 10;
            let [s, d] = butterfly($index, x, y, false);
            assert!(-out < s && s < out && -out < d && d < out);
        }

        #[kani::proof]
        fn $reduce() {
            let p = PRIMES[$index];
            let x: i32 = kani::any();
            let r = p.reduce(x);
            let bound = p.q / 2 + 32;
            assert!((-bound..=bound).contains(&r));
            assert_eq!((x as i64 - r as i64) % p.q as i64, 0);
        }

        #[kani::proof]
        fn $normalize() {
            let p = PRIMES[$index];
            let x: i32 = kani::any();
            kani::assume(-p.q < x && x < p.q);
            let r = p.normalize(x);
            assert!(0 <= r && r < p.q);
            assert!(r == x || r as i64 == x as i64 + p.q as i64);
        }

        #[kani::proof]
        fn $load() {
            let p = PRIMES[$index];
            let x: u32 = kani::any();
            let r = portable::load_one(x, p);
            assert!(-p.q < r && r < p.q);
            assert_eq!(p.normalize(r) as u32, x % p.q as u32);
        }

        /// All 24 legal lengths for this configured prime, including 1 and `2^MAX_LOG`.
        #[kani::proof]
        fn $scale() {
            let p = PRIMES[$index];
            let log: u32 = kani::any();
            kani::assume(log <= MAX_LOG);
            let n = 1usize << log;
            let (k, kq) = p.scale(n);

            let q = p.q as u128;
            let residue = ((1u128 << 64) / n as u128) % q;
            let want = if 2 * residue > q {
                residue as i128 - q as i128
            } else {
                residue as i128
            };
            assert_eq!(k as i128, want);
            assert!((k as i64).abs() <= (p.q / 2) as i64);
            assert_eq!((kq as u32).wrapping_mul(p.q as u32), k as u32);
        }
    };
}

prime_proofs!(
    reduce_q1,
    normalize_q1,
    load_q1,
    scale_q1,
    inverse_reduce_x_q1,
    inverse_lazy_q1,
    0
);
prime_proofs!(
    reduce_q2,
    normalize_q2,
    load_q2,
    scale_q2,
    inverse_reduce_x_q2,
    inverse_lazy_q2,
    1
);
prime_proofs!(
    reduce_q3,
    normalize_q3,
    load_q3,
    scale_q3,
    inverse_reduce_x_q3,
    inverse_lazy_q3,
    2
);

/// Every product shape served by MAX_LOG=23 has enough coefficients and valid pieces.
#[kani::proof]
#[kani::unwind(4)]
fn polynomial_plan_bounds() {
    let len: u32 = kani::any();
    kani::assume(1 <= len && len <= 1 << 22);
    let len = len as usize;
    let plan = Plan::new(len);
    assert!(plan.width() >= 2 * len);
    assert!(plan.width() <= (2 * len).max(8).next_power_of_two());
    assert!(plan.cyc.is_power_of_two());
    for &n in plan.nega() {
        assert!(n.is_power_of_two());
        assert!(plan.cyc < n);
        assert!(n.is_multiple_of(plan.cyc));
    }
    if plan.k == 2 {
        assert_eq!(plan.nega[0], 2 * plan.nega[1]);
    }
}
