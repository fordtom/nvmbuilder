use thiserror::Error;

#[derive(Debug, Error)]
pub enum LayoutError {
    #[error("File error: {0}.")]
    FileError(String),

    #[error("Block not found: {0}.")]
    BlockNotFound(String),

    #[error("Data value export failed: {0}.")]
    DataValueExportFailed(String),

    #[error("Invalid block argument: {0}.")]
    InvalidBlockArgument(String),

    #[error(transparent)]
    Variant(#[from] crate::variant::errors::VariantError),
}
