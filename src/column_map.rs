use roxmltree::Node;

use crate::{
    error::{ModelError, error},
    grid::Grid,
};

/// A terminal 1:1 footprint map: each source cell supplies one vertical column.
#[derive(Debug)]
pub struct ColumnMap {
    output: Grid,
    columns: Vec<Vec<u8>>,
}

impl ColumnMap {
    pub fn load(xml: Node<'_, '_>, source: &Grid) -> Result<Self, ModelError> {
        if source.depth != 1
            || xml.parent_element() != Some(xml.document().root_element())
            || !xml.document().root_element().has_tag_name("sequence")
            || xml.next_sibling_element().is_some()
        {
            return Err(error("column map must end a top-level 2D sequence"));
        }
        let scale: Vec<_> = xml
            .attribute("scale")
            .unwrap_or_default()
            .split_whitespace()
            .collect();
        let ["1", "1", height] = scale.as_slice() else {
            return Err(error("column map requires scale='1 1 N'"));
        };
        let depth = height
            .parse::<usize>()
            .map_err(|_| error("map height must be a positive integer"))?;
        let output = Grid::new(xml, source.width, source.height, depth)?;
        let mut columns = vec![vec![0; depth]; source.symbols.len()];
        for rule in xml.children().filter(Node::is_element) {
            if !rule.has_tag_name("rule") {
                return Err(error("column map accepts only inline rules"));
            }
            let symbol = rule
                .attribute("in")
                .and_then(|s| s.parse::<char>().ok())
                .ok_or_else(|| error("map input must be a single symbol"))?;
            let mask = source
                .waves
                .get(&symbol)
                .ok_or_else(|| error("unknown map input symbol"))?;
            let layers: Vec<_> = rule
                .attribute("out")
                .unwrap_or_default()
                .split_whitespace()
                .collect();
            if layers.is_empty() || layers.len() > depth {
                return Err(error("map output must fit its column"));
            }
            for (z, layer) in layers.iter().rev().enumerate() {
                if *layer == "*" {
                    continue;
                }
                let value = layer
                    .parse::<char>()
                    .ok()
                    .and_then(|c| output.indices.get(&c))
                    .ok_or_else(|| error("map layer must be one declared output symbol"))?;
                for (i, col) in columns.iter_mut().enumerate() {
                    if mask & (1 << i) != 0 {
                        col[z] = *value;
                    }
                }
            }
        }
        Ok(Self { output, columns })
    }

    pub fn apply(mut self, source: &Grid) -> Grid {
        for (layer, cells) in self.output.cells.chunks_mut(source.cells.len()).enumerate() {
            for (cell, &input) in cells.iter_mut().zip(&source.cells) {
                *cell = self.columns[usize::from(input)][layer];
            }
        }
        self.output
    }
}
