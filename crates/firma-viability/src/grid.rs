//! Bounded integer lattice with row-major indexing (manual §9.3, §15.2).

use std::ops::RangeInclusive;

/// Construction error for a [`Grid`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GridError {
    /// A grid needs at least one dimension.
    NoDimensions,
    /// A dimension had `start > end`.
    EmptyDimension(usize),
    /// The total point count overflowed `usize`.
    TooLarge,
}

impl std::fmt::Display for GridError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GridError::NoDimensions => f.write_str("grid has no dimensions"),
            GridError::EmptyDimension(d) => write!(f, "grid dimension {d} is empty (start > end)"),
            GridError::TooLarge => f.write_str("grid point count overflows usize"),
        }
    }
}

impl std::error::Error for GridError {}

/// A finite integer lattice: one inclusive `[lo, hi]` range per dimension.
/// Points are enumerated in row-major (last dimension varies fastest) order and
/// addressed by a `usize` index; this is the index a [`KernelSet`](crate::KernelSet)
/// bit refers to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid {
    dims: Vec<RangeInclusive<i64>>,
    /// `strides[d]` = number of index steps for +1 in dimension `d`.
    strides: Vec<usize>,
    size: usize,
}

impl Grid {
    /// Build a grid from per-dimension inclusive ranges.
    ///
    /// # Errors
    /// [`GridError`] if there are no dimensions, a dimension is empty, or the
    /// point count overflows.
    pub fn new(dims: Vec<RangeInclusive<i64>>) -> Result<Grid, GridError> {
        if dims.is_empty() {
            return Err(GridError::NoDimensions);
        }
        let mut extents = Vec::with_capacity(dims.len());
        for (d, r) in dims.iter().enumerate() {
            if r.start() > r.end() {
                return Err(GridError::EmptyDimension(d));
            }
            let ext = usize::try_from(r.end() - r.start())
                .ok()
                .and_then(|w| w.checked_add(1))
                .ok_or(GridError::TooLarge)?;
            extents.push(ext);
        }
        // Row-major strides: last dimension fastest.
        let mut strides = vec![1usize; dims.len()];
        for d in (0..dims.len() - 1).rev() {
            strides[d] = strides[d + 1]
                .checked_mul(extents[d + 1])
                .ok_or(GridError::TooLarge)?;
        }
        let size = strides[0]
            .checked_mul(extents[0])
            .ok_or(GridError::TooLarge)?;
        Ok(Grid {
            dims,
            strides,
            size,
        })
    }

    /// Number of dimensions.
    #[must_use]
    pub fn ndim(&self) -> usize {
        self.dims.len()
    }

    /// Total number of points.
    #[must_use]
    pub fn size(&self) -> usize {
        self.size
    }

    /// The coordinates of the point with row-major index `i`.
    ///
    /// # Panics
    /// If `i >= self.size()`.
    #[must_use]
    pub fn point(&self, i: usize) -> Vec<i64> {
        assert!(i < self.size, "grid index {i} out of range {}", self.size);
        let mut rem = i;
        let mut coords = Vec::with_capacity(self.dims.len());
        for d in 0..self.dims.len() {
            let c = rem / self.strides[d];
            rem %= self.strides[d];
            coords.push(self.dims[d].start() + c as i64);
        }
        coords
    }

    /// The row-major index of `coords`, or `None` if any coordinate is outside
    /// its dimension's range (a constraint violation, §15.2).
    #[must_use]
    pub fn index_of(&self, coords: &[i64]) -> Option<usize> {
        if coords.len() != self.dims.len() {
            return None;
        }
        let mut idx = 0usize;
        for (d, &c) in coords.iter().enumerate() {
            if !self.dims[d].contains(&c) {
                return None;
            }
            idx += (c - self.dims[d].start()) as usize * self.strides[d];
        }
        Some(idx)
    }
}
