use std::fmt;

/// An error raised while loading or running a model.
#[derive(Debug)]
pub struct ModelError(pub(crate) String);

impl fmt::Display for ModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ModelError {}

pub fn error(message: impl Into<String>) -> ModelError {
    ModelError(message.into())
}
