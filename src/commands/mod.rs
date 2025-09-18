pub mod generate;

use crate::args::Args;
use crate::error::NvmError;
use crate::variant::DataSheet;
use rayon::prelude::*;

pub fn build_separate_blocks(args: &Args, data_sheet: &DataSheet) -> Result<(), NvmError> {
    args.layout
        .blocks
        .par_iter()
        .try_for_each(|input| generate::build_block_single(input, data_sheet, args))
}
