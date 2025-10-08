use thiserror::Error;

#[derive(Debug, Error)]
pub enum LayoutError {
    #[error("File error: {0}.")]
    FileError(String),

    #[error("Failed to extract {0}.")]
    FailedToExtract(String),

    #[error("Block not found: {0}.")]
    BlockNotFound(String),

    #[error("Recursion failed: {0}.")]
    RecursionFailed(String),

    #[error("Data value export failed: {0}.")]
    DataValueExportFailed(String),

    #[error("Bytestream assembly failed: {0}.")]
    BytestreamAssemblyFailed(String),

    #[error("Array error: {0}.")]
    ArrayError(String),
}
