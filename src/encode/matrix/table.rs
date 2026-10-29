//! The table of powers `2^(32i)` in base 58^4, in panels of 8 rows, and its size classes.
//!
//! `2^(32i)` is not a multiple of 58, so every row starts at column 0, unlike the
//! decoder's table. Panel `p` stores `nb` blocks of 4 columns as `[nb][8][4]` words, zero
//! past each row's top column.

use alloc::vec::Vec;

/// Input bytes per word.
pub(super) const WORD: usize = 4;
/// The base of a column. Below 2^24, which leaves headroom in `u64` column sums.
pub(super) const COL_BASE: u64 = 58u64.pow(4);
/// Rows per panel: the panel kernels take 8 words.
pub(super) const ROWS: usize = 8;
/// Input bytes per panel.
pub(super) const PANEL_BYTES: usize = WORD * ROWS;

/// Size classes: class `k` holds `512 << k` bytes.
const CLASSES: usize = 5;
const fn class_bytes(k: usize) -> usize {
    512 << k
}
// Exact padded word counts, checked against every generated class below.
const DATA_WORDS: [usize; CLASSES] = [11_968, 46_336, 182_144, 722_272, 2_876_480];
/// Most bytes the table covers.
pub(super) const MAX_BYTES: usize = class_bytes(CLASSES - 1);
/// Words for `MAX_BYTES` bytes, a whole number of panels.
pub(super) const MAX_WORDS: usize = MAX_BYTES / WORD;
/// Upper bound on [`Table::cols`]: a value below `2^(8 MAX_BYTES)` has at most
/// `8 MAX_BYTES / 23` columns (`log2(58^4) > 23`), plus block padding.
pub(super) const MAX_COLS: usize = MAX_BYTES * 8 / 23 + 8;

/// Largest product: `(2^32 - 1) * (58^4 - 1)`, the decoder's bound.
const MAX_PRODUCT: u128 = u32::MAX as u128 * (COL_BASE as u128 - 1);
/// Largest carry out of a column: the quotient of a `u64` by `58^4`, below 2^40.6.
pub(super) const MAX_CARRY: u64 = u64::MAX / COL_BASE;
/// Largest column value right after a carry pass: a remainder plus one carry.
pub(super) const MAX_AFTER_CARRY: u64 = COL_BASE - 1 + MAX_CARRY;
/// Panels between carry passes. Each panel adds 8 products to a column.
pub(super) const CARRY_EVERY: usize = 47;
const _: () = assert!(
    MAX_AFTER_CARRY as u128 + (ROWS * CARRY_EVERY) as u128 * MAX_PRODUCT <= u64::MAX as u128
);
// One more panel could overflow: the schedule is as long as the bound allows.
const _: () = assert!(
    MAX_AFTER_CARRY as u128 + (ROWS * (CARRY_EVERY + 1)) as u128 * MAX_PRODUCT > u64::MAX as u128
);
// The final pass finishes each division with one compare: see `normalize`.
const _: () = assert!(MAX_AFTER_CARRY / COL_BASE + 1 < COL_BASE);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Panel {
    /// Blocks of 4 columns, from column 0.
    pub(super) nb: usize,
    /// Offset of the panel's `32 * nb` words in [`Table::data`].
    pub(super) offset: usize,
}

pub(super) struct Table {
    pub(super) panels: Vec<Panel>,
    pub(super) data: Vec<u32>,
}

impl Table {
    /// The table for size class `k`.
    pub(super) fn new(k: usize) -> Table {
        let panels = class_bytes(k) / PANEL_BYTES;
        // Since 58^4 > 2^23, this many base-58^4 columns holds every power below
        // 2^(8 * class_bytes(k)). Reuse eight rows instead of allocating each clone.
        let stride = (8 * class_bytes(k)).div_ceil(23);
        let mut rows = vec![0u32; ROWS * stride];
        let mut lengths = [0; ROWS];
        rows[0] = 1;
        lengths[0] = 1;
        let mut meta = Vec::with_capacity(panels);
        let mut data = Vec::with_capacity(DATA_WORDS[k]);
        for p in 0..panels {
            for r in 0..ROWS {
                if p != 0 || r != 0 {
                    let previous = if r == 0 { ROWS - 1 } else { r - 1 };
                    let len = lengths[previous];
                    rows.copy_within(previous * stride..previous * stride + len, r * stride);
                    lengths[r] = mul_word(&mut rows[r * stride..(r + 1) * stride], len);
                }
            }
            // The last row is the longest.
            let nb = lengths[ROWS - 1].div_ceil(4);
            meta.push(Panel {
                nb,
                offset: data.len(),
            });
            for b in 0..nb {
                for (r, &len) in lengths.iter().enumerate() {
                    for w in 0..4 {
                        let j = 4 * b + w;
                        data.push(if j < len { rows[r * stride + j] } else { 0 });
                    }
                }
            }
        }
        Table { panels: meta, data }
    }

    /// Columns the sums of the first `panels` panels need: the last panel's blocks and two
    /// more. With `L` columns in the top row, the value is below `2^32 * 58^(4L)`, and
    /// `2^32 < 58^8`. Requires `panels >= 1`.
    pub(super) fn cols(&self, panels: usize) -> usize {
        4 * self.panels[panels - 1].nb + 2
    }

    /// The words of panel `p`.
    pub(super) fn words(&self, p: usize) -> &[u32] {
        let Panel { nb, offset } = self.panels[p];
        &self.data[offset..offset + 32 * nb]
    }
}

/// `x[..len] *= 2^32` in base 58^4, returning its length. The slice must hold the product.
fn mul_word(x: &mut [u32], mut len: usize) -> usize {
    // `v < 58^4 * 2^32 + carry`, so the carry stays below 2^33.
    let mut carry = 0;
    for c in &mut x[..len] {
        let v = ((*c as u64) << 32) + carry;
        *c = (v % COL_BASE) as u32;
        carry = v / COL_BASE;
    }
    while carry != 0 {
        x[len] = (carry % COL_BASE) as u32;
        len += 1;
        carry /= COL_BASE;
    }
    len
}

/// Process-wide storage and lookup, omitted when caching is disabled.
#[cfg(feature = "cache")]
pub(super) mod cached {
    use super::*;
    use std::sync::OnceLock;

    pub(in crate::encode::matrix) static TABLES: [OnceLock<Table>; CLASSES] =
        [const { OnceLock::new() }; CLASSES];

    /// A table that holds `bytes` bytes. Reuse the largest suitable initialized class;
    /// otherwise build the smallest class that holds `bytes`. Previously built smaller
    /// classes stay allocated. Requires `bytes <= MAX_BYTES`.
    pub(in crate::encode::matrix) fn get(bytes: usize) -> &'static Table {
        let k = (0..CLASSES)
            .find(|&k| bytes <= class_bytes(k))
            .expect("too many bytes");
        TABLES[k..]
            .iter()
            .rev()
            .find_map(OnceLock::get)
            .unwrap_or_else(|| TABLES[k].get_or_init(|| Table::new(k)))
    }
}

#[cfg(feature = "cache")]
pub(super) use cached::get;

/// Static header bytes and class rows, without initializing a table.
#[cfg(feature = "__internals")]
#[must_use]
pub fn cache_memory() -> (usize, [crate::internals::MatrixCacheClass; CLASSES]) {
    #[cfg(feature = "cache")]
    {
        let rows = core::array::from_fn(|k| match cached::TABLES[k].get() {
            Some(t) => (
                class_bytes(k),
                true,
                t.panels.len(),
                t.panels.capacity() * size_of::<Panel>(),
                t.data.len(),
                t.data.capacity() * size_of::<u32>(),
            ),
            None => (class_bytes(k), false, 0, 0, 0, 0),
        });
        (size_of_val(&cached::TABLES), rows)
    }
    #[cfg(not(feature = "cache"))]
    (
        0,
        core::array::from_fn(|k| (class_bytes(k), false, 0, 0, 0, 0)),
    )
}

/// Without process-wide caching, this conversion owns the smallest suitable table.
#[cfg(not(feature = "cache"))]
pub(super) fn get(bytes: usize) -> Table {
    let k = (0..CLASSES)
        .find(|&k| bytes <= class_bytes(k))
        .expect("too many bytes");
    Table::new(k)
}

#[cfg(test)]
mod tests {
    use malachite_base::num::arithmetic::traits::PowerOf2;
    use malachite_base::num::conversion::traits::Digits;
    use malachite_nz::natural::Natural;

    use super::*;

    #[cfg(not(feature = "cache"))]
    #[test]
    fn tables_are_owned_without_cache() {
        let (a, b) = (get(512), get(512));
        assert_eq!(a.data, b.data);
        assert_ne!(a.data.as_ptr(), b.data.as_ptr());
        #[cfg(feature = "__internals")]
        {
            let (header, rows) = cache_memory();
            assert_eq!(header, 0);
            assert!(
                rows.iter()
                    .all(|r| !r.1 && r.2 == 0 && r.3 == 0 && r.4 == 0 && r.5 == 0)
            );
        }
    }

    /// `2^(32 i)` in base 58^4, by malachite-nz (independent of the builder).
    fn power(i: usize) -> Vec<u32> {
        Natural::power_of_2(32 * i as u64).to_digits_asc(&(COL_BASE as u32))
    }

    /// The table of class `k` itself; [`get`] may return a larger one.
    fn class(k: usize) -> Table {
        Table::new(k)
    }

    // The old builder keeps independent, exact-length row Vecs. This catches stale
    // workspace words, wrong row rotation and differences at class boundaries.
    fn reference(k: usize) -> Table {
        let panels = class_bytes(k) / PANEL_BYTES;
        let mut row = vec![1u32];
        let mut meta = Vec::with_capacity(panels);
        let mut data = Vec::with_capacity(DATA_WORDS[k]);
        for _ in 0..panels {
            let rows: [Vec<u32>; ROWS] = core::array::from_fn(|_| {
                let r = row.clone();
                reference_mul_word(&mut row);
                r
            });
            let nb = rows[ROWS - 1].len().div_ceil(4);
            meta.push(Panel {
                nb,
                offset: data.len(),
            });
            for b in 0..nb {
                for r in &rows {
                    for w in 0..4 {
                        data.push(r.get(4 * b + w).copied().unwrap_or(0));
                    }
                }
            }
        }
        Table { panels: meta, data }
    }

    fn reference_mul_word(x: &mut Vec<u32>) {
        let mut carry = 0;
        for c in x.iter_mut() {
            let v = ((*c as u64) << 32) + carry;
            *c = (v % COL_BASE) as u32;
            carry = v / COL_BASE;
        }
        while carry != 0 {
            x.push((carry % COL_BASE) as u32);
            carry /= COL_BASE;
        }
    }

    #[test]
    fn reused_rows_match_reference_for_every_class() {
        for k in 0..CLASSES {
            let (got, want) = (class(k), reference(k));
            assert_eq!(got.panels, want.panels, "class {k}");
            assert_eq!(got.data, want.data, "class {k}");
        }
    }

    #[test]
    fn row_workspace_bound_and_canaries() {
        for k in 0..CLASSES {
            let stride = (8 * class_bytes(k)).div_ceil(23);
            let mut guarded = vec![u32::MAX; stride + 2];
            guarded[1..1 + stride].fill(0);
            guarded[1] = 1;
            let mut len = 1;
            for _ in 1..class_bytes(k) / WORD {
                len = super::mul_word(&mut guarded[1..1 + stride], len);
                assert!(len <= stride);
            }
            assert_eq!(&guarded[1..1 + len], power(class_bytes(k) / WORD - 1));
            assert_eq!(guarded[0], u32::MAX);
            assert_eq!(guarded[stride + 1], u32::MAX);
        }
    }

    #[test]
    fn rows_are_powers() {
        let t = class(CLASSES - 1);
        for p in 0..t.panels.len() {
            let Panel { nb, .. } = t.panels[p];
            let words = t.words(p);
            for r in 0..ROWS {
                let i = ROWS * p + r;
                if !(i < 50 || i.is_multiple_of(37) || i == MAX_WORDS - 1) {
                    continue;
                }
                let want = power(i);
                assert!(want.len() <= 4 * nb, "row {i} is cut off");
                for j in 0..4 * nb {
                    let got = words[(ROWS * (j / 4) + r) * 4 + j % 4];
                    assert_eq!(got, want.get(j).copied().unwrap_or(0), "row {i} col {j}");
                }
            }
        }
    }

    #[test]
    fn metadata() {
        for k in 0..CLASSES {
            let t = class(k);
            assert_eq!(t.panels.len(), class_bytes(k) / PANEL_BYTES);
            let mut offset = 0;
            for (p, panel) in t.panels.iter().enumerate() {
                assert_eq!(panel.offset, offset);
                offset += 32 * panel.nb;
                // Panel ends increase, so a carry pass's last column is one that no panel
                // and no earlier pass has written. `convert` relies on this.
                assert!(p == 0 || panel.nb > t.panels[p - 1].nb, "panel {p}");
            }
            assert_eq!(t.data.len(), offset);
            assert!(t.cols(t.panels.len()) <= MAX_COLS);
        }
    }

    #[test]
    fn storage_has_no_spare_capacity() {
        let sizes: Vec<_> = (0..CLASSES)
            .map(|k| {
                let t = class(k);
                assert_eq!(t.data.len(), DATA_WORDS[k]);
                assert_eq!(t.panels.capacity(), t.panels.len());
                (class_bytes(k), t.data.len() * 4, t.data.capacity() * 4)
            })
            .collect();
        assert!(
            sizes.iter().all(|row| row.1 == row.2),
            "class/data/capacity bytes: {sizes:?}"
        );
    }

    /// [`get`] relies on this to serve a request from a larger class.
    #[test]
    fn classes_are_prefixes() {
        let big = class(CLASSES - 1);
        for k in 0..CLASSES - 1 {
            let small = class(k);
            let n = small.panels.len();
            assert_eq!(small.panels[..], big.panels[..n]);
            assert_eq!(small.data[..], big.data[..small.data.len()]);
        }
    }

    /// Whatever class [`get`] returns holds the bytes, within [`MAX_COLS`].
    #[test]
    fn get_holds_the_bytes() {
        for bytes in [1, 512, 513, 1024, 2000, 2048, 2049, 4096, 4097, MAX_BYTES] {
            let (t, panels) = (get(bytes), bytes.div_ceil(PANEL_BYTES));
            assert!(t.panels.len() >= panels, "{bytes} bytes");
            assert!(t.cols(panels) <= MAX_COLS, "{bytes} bytes");
        }
    }

    #[test]
    #[should_panic(expected = "too many bytes")]
    fn too_many_bytes() {
        get(MAX_BYTES + 1);
    }
}
