//! The table of powers `2^(32i)` in base 58^4, in panels of 8 rows, and its size classes.
//!
//! `2^(32i)` is not a multiple of 58, so every row starts at column 0, unlike the
//! decoder's table. Panel `p` stores `nb` blocks of 4 columns as `[nb][8][4]` words, zero
//! past each row's top column.

use std::sync::OnceLock;

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
    /// The table for `panels` panels.
    pub(super) fn new(panels: usize) -> Table {
        let mut row = vec![1u32];
        let mut meta = Vec::with_capacity(panels);
        let mut data = Vec::new();
        for _ in 0..panels {
            let rows: [Vec<u32>; ROWS] = std::array::from_fn(|_| {
                let r = row.clone();
                mul_word(&mut row);
                r
            });
            // The last row is the longest.
            let nb = rows[ROWS - 1].len().div_ceil(4);
            meta.push(Panel { nb, offset: data.len() });
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

/// `x = x * 2^32` in base 58^4.
fn mul_word(x: &mut Vec<u32>) {
    // `v < 58^4 * 2^32 + carry`, so the carry stays below 2^33.
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

pub(super) static TABLES: [OnceLock<Table>; CLASSES] = [const { OnceLock::new() }; CLASSES];

/// A table that holds `bytes` bytes. A smaller class is a prefix of a larger one, so the
/// largest class already built serves, which keeps one table in cache; otherwise the
/// smallest class that holds `bytes` is built and kept. Requires `bytes <= MAX_BYTES`.
pub(super) fn get(bytes: usize) -> &'static Table {
    let k = (0..CLASSES).find(|&k| bytes <= class_bytes(k)).expect("too many bytes");
    TABLES[k..]
        .iter()
        .rev()
        .find_map(OnceLock::get)
        .unwrap_or_else(|| TABLES[k].get_or_init(|| Table::new(class_bytes(k) / PANEL_BYTES)))
}

#[cfg(test)]
mod tests {
    use malachite_base::num::arithmetic::traits::PowerOf2;
    use malachite_base::num::conversion::traits::Digits;
    use malachite_nz::natural::Natural;

    use super::*;

    /// `2^(32 i)` in base 58^4, by malachite-nz (independent of the builder).
    fn power(i: usize) -> Vec<u32> {
        Natural::power_of_2(32 * i as u64).to_digits_asc(&(COL_BASE as u32))
    }

    /// The table of class `k` itself; [`get`] may return a larger one.
    fn class(k: usize) -> Table {
        Table::new(class_bytes(k) / PANEL_BYTES)
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
