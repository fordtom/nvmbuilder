use thiserror::Error;

use crate::layout::errors::LayoutError;
use crate::output::errors::OutputError;
use crate::variant::errors::VariantError;

#[derive(Debug, Error)]
pub enum NvmError {
    #[error(transparent)]
    Layout(#[from] LayoutError),

    #[error(transparent)]
    Variant(#[from] VariantError),

    #[error(transparent)]
    Output(#[from] OutputError),
}
