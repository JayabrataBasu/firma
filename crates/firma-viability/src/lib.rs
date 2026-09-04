//! `firma-viability` — the viability kernel by backward iteration (manual §9.3).
//!
//! **Phase 1 scope (ADR 0011): the generic solver only.** This crate knows
//! nothing about firms, constraints, margins, or the FIRMA action core. It
//! operates on:
//!
//! * a [`Grid`] — a bounded integer lattice ("the box constraints", §15.2), and
//! * a [`Dynamics`] impl — an abstract admissible-transition relation.
//!
//! and computes
//!
//! ```text
//! K^(0) = grid
//! K^(n+1) = { x ∈ K^(n) : ∃ an admissible successor of x that lies in K^(n) }
//! Viab    = the fixed point (K^(n+1) == K^(n))
//! ```
//!
//! [`kernel`] returns the fixed point **and the full size sequence**
//! `|K^(0)| ≥ |K^(1)| ≥ …`, which is what VT-2 (§25.4) checks: monotone
//! decrease and termination at a fixed point.
//!
//! The Phase-2 domain layer (`margin`, `in_kernel`, `kernel_volume`, the FIRMA
//! `Dynamics` = deterministic core of §11) is built on top of this without
//! changing the `Dynamics` boundary — which is also the seam for the Phase-4
//! sampling-based approximator (§18.2).

#![forbid(unsafe_code)]

mod domain;
mod grid;

pub use domain::{firma_kernel, in_kernel, margin, volume, FirmaDynamics, DEFAULT_C_STEP};
pub use grid::{Grid, GridError};

/// An abstract discrete-time transition relation over integer points.
///
/// [`successors`](Dynamics::successors) returns every state reachable in one
/// admissible step from `point`. Successors that fall outside the [`Grid`] are
/// allowed in the returned list — the solver treats them as constraint
/// violations (§15.2: "Exceeding a bound is a violation, not clipping"), i.e.
/// permanently outside `K`.
pub trait Dynamics {
    /// Admissible one-step successors of `point`. `point` has one coordinate per
    /// grid dimension. May return points outside the grid.
    fn successors(&self, point: &[i64]) -> Vec<Vec<i64>>;
}

/// A subset of a [`Grid`], stored as a bitset indexed by the grid's row-major
/// point index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelSet {
    words: Vec<u64>,
    len: usize,
}

impl KernelSet {
    /// An empty set over a grid of `len` points.
    #[must_use]
    pub fn empty(len: usize) -> KernelSet {
        KernelSet {
            words: vec![0; len.div_ceil(64)],
            len,
        }
    }

    /// The full set (every grid point) over a grid of `len` points. This is the
    /// `K^(0)` of a *box-constrained* kernel; the Phase-2 domain layer supplies
    /// a smaller `K^(0)` via [`from_predicate`](Self::from_predicate) when the
    /// FIRMA constraints prune it (ADR 0021).
    #[must_use]
    pub fn full(len: usize) -> KernelSet {
        let mut s = KernelSet {
            words: vec![u64::MAX; len.div_ceil(64)],
            len,
        };
        // Clear the ragged tail bits so `count` is exact.
        for i in len..s.words.len() * 64 {
            s.clear(i);
        }
        s
    }

    /// The set of grid points satisfying `keep` — used to build `K^(0)` from
    /// the FIRMA constraint set (manual §9.3: `K^(0) = K(θ)`).
    #[must_use]
    pub fn from_predicate(grid: &Grid, keep: impl Fn(&[i64]) -> bool) -> KernelSet {
        let mut s = KernelSet::empty(grid.size());
        for i in 0..grid.size() {
            if keep(&grid.point(i)) {
                s.set(i);
            }
        }
        s
    }

    #[inline]
    fn get(&self, i: usize) -> bool {
        i < self.len && (self.words[i / 64] >> (i % 64)) & 1 == 1
    }

    #[inline]
    fn set(&mut self, i: usize) {
        self.words[i / 64] |= 1 << (i % 64);
    }

    #[inline]
    fn clear(&mut self, i: usize) {
        self.words[i / 64] &= !(1 << (i % 64));
    }

    /// Number of points in the set.
    #[must_use]
    pub fn count(&self) -> usize {
        self.words.iter().map(|w| w.count_ones() as usize).sum()
    }

    /// Whether the grid point with row-major index `i` is in the set.
    #[must_use]
    pub fn contains_index(&self, i: usize) -> bool {
        self.get(i)
    }

    /// Row-major indices of the points in the set, ascending.
    pub fn indices(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.len).filter(move |i| self.get(*i))
    }
}

/// The result of a [`kernel`] computation.
#[derive(Debug, Clone)]
pub struct KernelReport {
    /// The viability kernel — the fixed point of the backward iteration.
    pub kernel: KernelSet,
    /// `sizes[n] == |K^(n)|`. Length is `iterations + 1`. Guaranteed
    /// non-increasing, with the last two entries equal (the fixed point).
    pub sizes: Vec<usize>,
    /// Number of backward-iteration steps taken to reach the fixed point.
    pub iterations: usize,
    /// Total grid points, for volume.
    pub grid_size: usize,
}

impl KernelReport {
    /// `|Viab(K)| / |grid|` (manual §9.3: "volume = |K|/|grid|").
    #[must_use]
    pub fn volume(&self) -> f64 {
        if self.grid_size == 0 {
            0.0
        } else {
            self.kernel.count() as f64 / self.grid_size as f64
        }
    }

    /// True iff `sizes` is non-increasing and terminates in a repeated value —
    /// the VT-2 property (§25.4). The solver always produces this; the method
    /// exists so a test can assert it explicitly against the reported sequence.
    #[must_use]
    pub fn is_monotone_to_fixed_point(&self) -> bool {
        if self.sizes.len() < 2 {
            return false;
        }
        let non_increasing = self.sizes.windows(2).all(|w| w[0] >= w[1]);
        let last = self.sizes.len() - 1;
        non_increasing && self.sizes[last] == self.sizes[last - 1]
    }
}

/// Compute the viability kernel of `dynamics` on `grid` by backward iteration
/// from the full grid as `K^(0)` (manual §9.3). Equivalent to
/// [`kernel_from`] with `initial = KernelSet::full(grid.size())`.
#[must_use]
pub fn kernel<D: Dynamics>(grid: &Grid, dynamics: &D) -> KernelReport {
    kernel_from(grid, dynamics, KernelSet::full(grid.size()))
}

/// Compute the viability kernel of `dynamics` on `grid` by backward iteration
/// starting from `initial` as `K^(0)` (manual §9.3):
/// `K^(n+1) = { x ∈ K^(n) : ∃ an admissible successor of x in K^(n) }`.
///
/// The Phase-2 FIRMA layer passes `K^(0) = K(θ)` — the grid points satisfying
/// every constraint — here (ADR 0021), rather than the whole grid. The
/// backward-iteration algorithm itself is unchanged from Phase 1 (ADR 0011);
/// only the starting set is parameterised.
///
/// Terminates because the sequence is monotone decreasing on a finite set
/// (§9.3, theorem `[E]`).
///
/// # Panics
/// If `initial`'s length does not match `grid.size()`.
#[must_use]
pub fn kernel_from<D: Dynamics>(grid: &Grid, dynamics: &D, initial: KernelSet) -> KernelReport {
    let n = grid.size();
    assert_eq!(initial.len, n, "initial set length must match grid size");
    let mut current = initial;
    let mut sizes = vec![current.count()];
    let mut iterations = 0;

    loop {
        let mut next = KernelSet::empty(n);
        for i in current.indices() {
            let point = grid.point(i);
            let survives = dynamics.successors(&point).into_iter().any(|s| {
                grid.index_of(&s)
                    .is_some_and(|si| current.contains_index(si))
            });
            if survives {
                next.set(i);
            }
        }
        iterations += 1;
        sizes.push(next.count());
        if next == current {
            return KernelReport {
                kernel: next,
                sizes,
                iterations,
                grid_size: n,
            };
        }
        current = next;
    }
}
