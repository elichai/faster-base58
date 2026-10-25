//! The table of powers `58^(4i)` in base 2^32, in panels of 8 rows, and its size classes.
//!
//! `58^(4i) = 2^(4i) * 29^(4i)` and 29 is odd, so the lowest non-zero word of row `i` is
//! `i / 8`: every row of panel `p` starts at column `p`. Panel `p` stores `nb` blocks of
//! 4 columns as `[nb][8][4]` words, zero outside each row's non-zero words.

/// Characters per group: `58^4 < 2^24` leaves headroom in `u64` column sums.
pub(crate) const GROUP: usize = 4;
pub(crate) const GROUP_BASE: u64 = 58u64.pow(GROUP as u32);
/// Rows per panel.
pub(crate) const ROWS: usize = 8;
/// Characters per panel.
pub(crate) const PANEL_CHARS: usize = GROUP * ROWS;

/// Size classes: class `k` holds `704 << k` characters.
const CLASSES: usize = 4;
const fn class_chars(k: usize) -> usize {
    704 << k
}
// Exact padded word counts, checked against every generated class below.
const DATA_WORDS: [usize; CLASSES] = [10_208, 39_296, 153_824, 608_608];
/// Most characters the table covers.
pub(crate) const MAX_CHARS: usize = class_chars(CLASSES - 1);
/// Most groups for `MAX_CHARS` characters, padded to whole panels.
pub(crate) const MAX_GROUPS: usize = MAX_CHARS / GROUP;
/// Upper bound on [`Table::cols`]: the value has fewer than `6 * MAX_CHARS` bits
/// (`log2(58) < 6`), plus block padding, the carry column and rounding up to even.
pub(crate) const MAX_COLS: usize = MAX_CHARS * 6 / 32 + 8;

/// Largest product: `(58^4 - 1) * (2^32 - 1)`.
const MAX_PRODUCT: u128 = (GROUP_BASE as u128 - 1) * u32::MAX as u128;
/// Largest column value right after a carry pass: a word below 2^32, or one carry.
const MAX_AFTER_CARRY: u128 = 2 * u32::MAX as u128;
/// Panels between carry passes. Each panel adds 8 products to a column.
pub(crate) const CARRY_EVERY: usize = 47;
const _: () = assert!(
    MAX_AFTER_CARRY + (ROWS * CARRY_EVERY) as u128 * MAX_PRODUCT <= u64::MAX as u128
);
// One more panel could overflow: the schedule is as long as the bound allows.
const _: () = assert!(
    MAX_AFTER_CARRY + (ROWS * (CARRY_EVERY + 1)) as u128 * MAX_PRODUCT > u64::MAX as u128
);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Panel {
    /// Blocks of 4 columns, starting at the panel's index.
    pub(crate) nb: usize,
    /// Offset of the panel's `32 * nb` words in [`Table::data`].
    pub(crate) offset: usize,
}

pub(crate) struct Table {
    pub(crate) panels: Vec<Panel>,
    pub(crate) data: Vec<u32>,
}

impl Table {
    /// The table for size class `k`.
    pub(crate) fn new(k: usize) -> Table {
        let panels = class_chars(k) / PANEL_CHARS;
        let mut row = vec![1u32];
        let mut meta = Vec::with_capacity(panels);
        let mut data = Vec::with_capacity(DATA_WORDS[k]);
        for p in 0..panels {
            let rows: [Vec<u32>; ROWS] = std::array::from_fn(|_| {
                let r = row.clone();
                mul_small(&mut row, GROUP_BASE);
                r
            });
            // The last row is the longest. Its top word is non-zero.
            let nb = (rows[ROWS - 1].len() - p).div_ceil(4);
            meta.push(Panel { nb, offset: data.len() });
            for b in 0..nb {
                for r in &rows {
                    for w in 0..4 {
                        data.push(r.get(p + 4 * b + w).copied().unwrap_or(0));
                    }
                }
            }
        }
        Table { panels: meta, data }
    }

    /// Columns the column sums of the first `panels` panels need: up to the last
    /// panel's end, one carry column, rounded up to even. Requires `panels >= 1`.
    pub(crate) fn cols(&self, panels: usize) -> usize {
        let last = self.panels[panels - 1];
        (panels - 1 + 4 * last.nb + 1).next_multiple_of(2)
    }

    /// The words of panel `p`.
    pub(crate) fn words(&self, p: usize) -> &[u32] {
        let Panel { nb, offset } = self.panels[p];
        &self.data[offset..offset + 32 * nb]
    }
}

/// `x = x * m`, for `m < 2^32`.
fn mul_small(x: &mut Vec<u32>, m: u64) {
    let mut carry = 0;
    for w in x.iter_mut() {
        let v = *w as u64 * m + carry;
        *w = v as u32;
        carry = v >> 32;
    }
    if carry != 0 {
        x.push(carry as u32);
    }
}

/// Process-wide storage and lookup, omitted when caching is disabled.
#[cfg(feature = "cache")]
pub(super) mod cached {
    use super::*;
    use std::sync::OnceLock;

    pub(in crate::matrix) static TABLES: [OnceLock<Table>; CLASSES] = [const { OnceLock::new() }; CLASSES];

    /// A table that holds `chars` characters. A smaller class is a prefix of a larger one, so
    /// any larger class already built serves; otherwise the smallest class that holds
    /// `chars` is built and kept. Requires `chars <= MAX_CHARS`.
    pub(crate) fn get(chars: usize) -> &'static Table {
        let k = (0..CLASSES).find(|&k| chars <= class_chars(k)).expect("too many characters");
        TABLES[k..]
            .iter()
            .find_map(OnceLock::get)
            .unwrap_or_else(|| TABLES[k].get_or_init(|| Table::new(k)))
    }
}

#[cfg(feature = "cache")]
pub(crate) use cached::get;

/// Static header bytes and class rows, without initializing a table.
#[cfg(feature = "__internals")]
pub fn cache_memory() -> (usize, [crate::internals::MatrixCacheClass; CLASSES]) {
    #[cfg(feature = "cache")]
    {
        let rows = std::array::from_fn(|k| match cached::TABLES[k].get() {
            Some(t) => (
                class_chars(k), true, t.panels.len(),
                t.panels.capacity() * size_of::<Panel>(), t.data.len(),
                t.data.capacity() * size_of::<u32>(),
            ),
            None => (class_chars(k), false, 0, 0, 0, 0),
        });
        (size_of_val(&cached::TABLES), rows)
    }
    #[cfg(not(feature = "cache"))]
    (0, std::array::from_fn(|k| (class_chars(k), false, 0, 0, 0, 0)))
}

/// Without process-wide caching, this conversion owns the smallest suitable table.
#[cfg(not(feature = "cache"))]
pub(crate) fn get(chars: usize) -> Table {
    let k = (0..CLASSES).find(|&k| chars <= class_chars(k)).expect("too many characters");
    Table::new(k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "cache"))]
    #[test]
    fn tables_are_owned_without_cache() {
        let (a, b) = (get(704), get(704));
        assert_eq!(a.data, b.data);
        assert_ne!(a.data.as_ptr(), b.data.as_ptr());
        #[cfg(feature = "__internals")]
        {
            let (header, rows) = cache_memory();
            assert_eq!(header, 0);
            assert!(rows.iter().all(|r| !r.1 && r.2 == 0 && r.3 == 0 && r.4 == 0 && r.5 == 0));
        }
    }

    /// `58^e` in base 2^32, by repeated multiplication by 58 (independent of the builder).
    fn power(e: usize) -> Vec<u32> {
        let mut x = vec![1u32];
        for _ in 0..e {
            mul_small(&mut x, 58);
        }
        x
    }

    /// The table of class `k` itself; [`get`] may return a larger one.
    fn class(k: usize) -> Table {
        Table::new(k)
    }

    #[test]
    fn rows_are_powers() {
        let t = class(0);
        let mut want = vec![1u32];
        for p in 0..t.panels.len() {
            let Panel { nb, .. } = t.panels[p];
            let words = t.words(p);
            for r in 0..ROWS {
                let i = ROWS * p + r;
                if i.is_multiple_of(37) {
                    assert_eq!(want, power(GROUP * i), "row {i}");
                }
                // Below the panel's first column the row is zero.
                assert!(want[..p].iter().all(|&w| w == 0), "row {i}");
                for b in 0..nb {
                    for w in 0..4 {
                        let j = p + 4 * b + w;
                        let got = words[(ROWS * b + r) * 4 + w];
                        assert_eq!(got, want.get(j).copied().unwrap_or(0), "row {i} col {j}");
                    }
                }
                assert!(want.len() <= p + 4 * nb, "row {i} is cut off");
                mul_small(&mut want, GROUP_BASE);
            }
        }
    }

    #[test]
    fn metadata() {
        for k in 0..CLASSES {
            let t = class(k);
            assert_eq!(t.panels.len(), class_chars(k) / PANEL_CHARS);
            let mut offset = 0;
            let mut end = 0;
            for (p, panel) in t.panels.iter().enumerate() {
                assert_eq!(panel.offset, offset);
                offset += 32 * panel.nb;
                // Panel ends never decrease, so the last panel's end bounds all columns.
                assert!(p + 4 * panel.nb >= end);
                end = p + 4 * panel.nb;
            }
            assert_eq!(t.data.len(), offset);
            assert!(t.cols(t.panels.len()) <= MAX_COLS);
        }
    }

    #[test]
    fn storage_has_no_spare_capacity() {
        let sizes: Vec<_> = (0..CLASSES).map(|k| {
            let t = class(k);
            assert_eq!(t.data.len(), DATA_WORDS[k]);
            assert_eq!(t.panels.capacity(), t.panels.len());
            (class_chars(k), t.data.len() * 4, t.data.capacity() * 4)
        }).collect();
        assert!(sizes.iter().all(|row| row.1 == row.2), "class/data/capacity bytes: {sizes:?}");
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

    /// Whatever class [`get`] returns holds the characters, within [`MAX_COLS`].
    #[test]
    fn get_holds_the_characters() {
        for chars in [1, 704, 705, 1408, 2000, 2816, 2817, MAX_CHARS] {
            let (t, panels) = (get(chars), chars.div_ceil(PANEL_CHARS));
            assert!(t.panels.len() >= panels, "{chars} characters");
            assert!(t.cols(panels) <= MAX_COLS, "{chars} characters");
        }
    }

    #[test]
    #[should_panic(expected = "too many characters")]
    fn too_many_characters() {
        get(MAX_CHARS + 1);
    }
}
