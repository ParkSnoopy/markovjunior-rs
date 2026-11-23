use rand::Rng;
use roxmltree::Document;

use crate::{
    error::{ModelError, error},
    grid::Grid,
    node::Node,
};

/// A loaded `MarkovJunior` model.
#[derive(Debug)]
pub struct Model {
    grid: Grid,
    root: Node,
}

impl Model {
    /// Loads a model XML document with the requested grid dimensions.
    ///
    /// # Errors
    ///
    /// Returns an error when the XML, grid, or supported node data is invalid.
    pub fn from_xml(xml: &str, width: usize, height: usize) -> Result<Self, ModelError> {
        Self::from_xml_3d(xml, width, height, 1)
    }

    /// Loads a model on a volume. A depth of one selects 2D semantics.
    ///
    /// # Errors
    /// Returns an error for invalid XML, dimensions, or unsupported nodes.
    pub fn from_xml_3d(
        xml: &str,
        width: usize,
        height: usize,
        depth: usize,
    ) -> Result<Self, ModelError> {
        let document =
            Document::parse(xml).map_err(|parse_error| error(parse_error.to_string()))?;
        let root = document.root_element();
        let grid = Grid::new(root, width, height, depth)?;
        let root_node = Node::load(root, &grid, [true; 8])?;
        Ok(Self {
            grid,
            root: root_node,
        })
    }

    /// Runs the model for at most `steps` rewrite turns.
    pub fn run<R: Rng>(&mut self, steps: usize, rng: &mut R) -> usize {
        let mut completed = 0;
        while completed < steps && self.root.tick(&mut self.grid, rng) {
            completed += 1;
        }
        completed
    }

    /// Returns the final grid.
    #[must_use]
    pub const fn grid(&self) -> &Grid {
        &self.grid
    }
}
