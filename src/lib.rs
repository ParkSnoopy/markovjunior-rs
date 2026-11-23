//! A small `MarkovJunior` rewrite engine.

mod convolution;
mod error;
mod grid;
mod model;
mod node;
mod path;
mod rule;
mod vox;

pub use error::ModelError;
pub use grid::Grid;
pub use model::Model;
