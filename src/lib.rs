//! A small `MarkovJunior` rewrite engine.

mod error;
mod grid;
mod model;
mod node;
mod rule;

pub use error::ModelError;
pub use grid::Grid;
pub use model::Model;
