use std::collections::{HashMap, HashSet};

use crate::error::{ModelError, error};

/// A rectangular two-dimensional grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) symbols: Vec<char>,
    pub(crate) indices: HashMap<char, u8>,
    pub(crate) cells: Vec<u8>,
}

impl Grid {
    pub(crate) fn new(
        width: usize,
        height: usize,
        values: &str,
        origin: bool,
    ) -> Result<Self, ModelError> {
        let symbols: Vec<char> = values.chars().collect();
        if width == 0 || height == 0 || !(1..=8).contains(&symbols.len()) {
            return Err(error(
                "grid dimensions must be positive and values must contain 1 to 8 symbols",
            ));
        }
        let cell_count = width
            .checked_mul(height)
            .ok_or_else(|| error("grid dimensions exceed supported size"))?;
        if origin && symbols.len() < 2 {
            return Err(error("origin requires at least two grid values"));
        }
        if symbols.iter().copied().collect::<HashSet<_>>().len() != symbols.len() {
            return Err(error("grid values must be unique"));
        }
        let indices = symbols
            .iter()
            .enumerate()
            .map(|(index, symbol)| (*symbol, index as u8))
            .collect();
        let mut grid = Self {
            width,
            height,
            symbols,
            indices,
            cells: vec![0; cell_count],
        };
        if origin {
            let center = width / 2 + (height / 2) * width;
            grid.cells[center] = 1;
        }
        Ok(grid)
    }

    /// Returns grid width in cells.
    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    /// Returns grid height in cells.
    #[must_use]
    pub const fn height(&self) -> usize {
        self.height
    }

    /// Returns the grid as rows of symbols.
    #[must_use]
    pub fn rows(&self) -> Vec<String> {
        self.cells
            .chunks(self.width)
            .map(|row| {
                row.iter()
                    .map(|value| self.symbols[usize::from(*value)])
                    .collect()
            })
            .collect()
    }
}
