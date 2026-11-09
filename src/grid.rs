use std::collections::{HashMap, HashSet};

use roxmltree::Node as XmlNode;

use crate::error::{ModelError, error};

/// A rectangular two-dimensional grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) symbols: Vec<char>,
    pub(crate) indices: HashMap<char, u8>,
    pub(crate) waves: HashMap<char, u16>,
    pub(crate) cells: Vec<u8>,
}

impl Grid {
    pub(crate) fn new(
        xml: XmlNode<'_, '_>,
        width: usize,
        height: usize,
    ) -> Result<Self, ModelError> {
        let values = xml
            .attribute("values")
            .ok_or_else(|| error("root node is missing 'values'"))?;
        let origin = match xml.attribute("origin").unwrap_or("false") {
            value if value.eq_ignore_ascii_case("true") => true,
            value if value.eq_ignore_ascii_case("false") => false,
            _ => return Err(error("origin must be true or false")),
        };
        let symbols: Vec<char> = values.chars().filter(|symbol| *symbol != ' ').collect();
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
        if symbols.contains(&'*') {
            return Err(error("grid values cannot use reserved wildcard symbol '*'"));
        }

        let indices = symbols
            .iter()
            .enumerate()
            .map(|(index, symbol)| (*symbol, index as u8))
            .collect();
        let mut waves: HashMap<char, u16> = symbols
            .iter()
            .enumerate()
            .map(|(index, symbol)| (*symbol, 1_u16 << index))
            .collect();
        waves.insert('*', (1_u16 << symbols.len()) - 1);
        load_unions(xml, &indices, &mut waves)?;
        let mut grid = Self {
            width,
            height,
            symbols,
            indices,
            waves,
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

fn load_unions(
    xml: XmlNode<'_, '_>,
    indices: &HashMap<char, u8>,
    waves: &mut HashMap<char, u16>,
) -> Result<(), ModelError> {
    for child in xml.children() {
        match child.tag_name().name() {
            "markov" | "sequence" | "union" => {
                if child.has_tag_name("union") {
                    let symbol_text = child
                        .attribute("symbol")
                        .ok_or_else(|| error("union is missing 'symbol'"))?;
                    let mut symbol_chars = symbol_text.chars();
                    let symbol = symbol_chars
                        .next()
                        .ok_or_else(|| error("union symbol must contain one character"))?;
                    if symbol_chars.next().is_some() {
                        return Err(error("union symbol must contain one character"));
                    }
                    if waves.contains_key(&symbol) {
                        return Err(error(format!("union symbol '{symbol}' is already defined")));
                    }
                    let values = child
                        .attribute("values")
                        .ok_or_else(|| error("union is missing 'values'"))?;
                    let mut wave = 0;
                    for value in values.chars() {
                        let Some(index) = indices.get(&value) else {
                            return Err(error(format!(
                                "union value '{value}' is not declared by grid values"
                            )));
                        };
                        wave |= 1_u16 << index;
                    }
                    waves.insert(symbol, wave);
                }
                load_unions(child, indices, waves)?;
            }
            _ => {}
        }
    }
    Ok(())
}
