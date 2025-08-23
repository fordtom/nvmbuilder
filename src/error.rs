use thiserror::Error;

#[derive(Debug, Error)]
pub enum NvmError {
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

    #[error("Excel column not found: {0}.")]
    ColumnNotFound(String),

    #[error("Excel retrieval error: {0}.")]
    RetrievalError(String),

    #[error("Array error: {0}.")]
    ArrayError(String),

    #[error("Misc error: {0}.")]
    MiscError(String),

    #[error("Hex output error: {0}.")]
    HexOutputError(String),
}
