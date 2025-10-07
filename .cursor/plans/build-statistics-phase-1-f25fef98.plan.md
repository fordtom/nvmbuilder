<!-- f25fef98-2ff0-41fa-966e-e111a4a22c29 37697ef8-96e2-4624-9c4a-d68fff5b3a67 -->
# Phase 1: Build Statistics Collection & Basic Reporting

## Overview

Add statistics collection throughout the build pipeline with a distributed architecture: stats structs live in the modules they measure (like the args pattern), and a separate printer module handles all output formatting. This creates a clean foundation for Phase 2 visualization features.

## Dependencies

Add to `Cargo.toml`:
```toml
comfy-table = "7.1"  # For tabular output formatting
```

## Architecture Changes

### Distributed Stats Collection

- `BlockStat` struct in `src/commands/mod.rs` - per-block statistics
- `BuildStats` struct in `src/commands/mod.rs` - aggregate statistics  
- `src/printer.rs` module (NEW) - all output formatting logic (summary, detailed tables, future JSON/graphical)

This separates data collection from presentation, making it easy to add new output formats in Phase 2.

### CLI Arguments Extension

Add to `src/output/args.rs` in `OutputArgs` struct:
```rust
#[arg(long, help = "Show detailed build statistics")]
pub stats: bool,

#[arg(long, help = "Suppress all output except errors")]
pub quiet: bool,
```

### Return Type Changes

- `build_block_single()`: Change from `Result<(), NvmError>` to `Result<BlockStat, NvmError>`
- `build_separate_blocks()`: Change to `Result<BuildStats, NvmError>`
- `build_single_file()`: Change to `Result<BuildStats, NvmError>`

## Implementation by Module

### 1. `src/output/mod.rs`

Modify `DataRange` struct to include size metadata:
```rust
pub struct DataRange {
    pub start_address: u32,
    pub bytestream: Vec<u8>,
    pub crc_address: u32,
    pub crc_bytestream: Vec<u8>,
    pub used_size: u32,        // NEW: actual data size before padding
    pub allocated_size: u32,   // NEW: block allocated size
}
```

Update `bytestream_to_datarange()`:
- Calculate `used_size` from initial bytestream length before any padding
- Extract `allocated_size` from `header.length`
- Populate new fields in returned `DataRange`
- No new function parameters needed - calculate internally

### 2. `src/commands/mod.rs`

Add stats structs at the top of the file:

```rust
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct BlockStat {
    pub name: String,
    pub start_address: u32,
    pub allocated_size: u32,
    pub used_size: u32,
    pub crc_address: u32,
    pub crc_value: u32,
}

#[derive(Debug)]
pub struct BuildStats {
    pub blocks_processed: usize,
    pub total_allocated: usize,
    pub total_used: usize,
    pub total_duration: Duration,
    pub block_stats: Vec<BlockStat>,
}

impl BuildStats {
    pub fn new() -> Self {
        Self {
            blocks_processed: 0,
            total_allocated: 0,
            total_used: 0,
            total_duration: Duration::from_secs(0),
            block_stats: Vec::new(),
        }
    }
    
    pub fn add_block(&mut self, stat: BlockStat) {
        self.blocks_processed += 1;
        self.total_allocated += stat.allocated_size as usize;
        self.total_used += stat.used_size as usize;
        self.block_stats.push(stat);
    }
    
    pub fn space_efficiency(&self) -> f64 {
        if self.total_allocated == 0 {
            0.0
        } else {
            (self.total_used as f64 / self.total_allocated as f64) * 100.0
        }
    }
}
```

Update `build_separate_blocks()`:
- Record start time with `Instant::now()`
- Collect `BlockStat` from parallel builds using rayon
- Aggregate into `BuildStats`
- Calculate total duration before returning
- Return `BuildStats`

Update `build_single_file()`:
- Same approach for combined mode
- Collect per-block stats
- Return `BuildStats`

### 3. `src/commands/generate.rs`

Modify `build_block_single()` to return `BlockStat`:

```rust
pub fn build_block_single(
    input: &BlockNames,
    data_sheet: &DataSheet,
    args: &Args,
) -> Result<BlockStat, NvmError>
```

Collect statistics from existing data (no timing):
1. Block name from `input.name`
2. Start address from `block.header.start_address + layout.settings.virtual_offset`
3. Allocated size from `data_range.allocated_size`
4. Used size from `data_range.used_size`
5. CRC address from `data_range.crc_address`
6. CRC value by converting `data_range.crc_bytestream` back to u32 (accounting for endianness from layout.settings)

Return `BlockStat` with collected data.

### 4. `src/printer.rs` (NEW)

Create output formatting module:

```rust
use crate::commands::{BlockStat, BuildStats};
use comfy_table::{Table, Cell, Attribute, ContentArrangement};

pub fn print_summary(stats: &BuildStats) {
    println!(
        "✓ Built {} blocks in {}ms ({:.1}% efficiency)",
        stats.blocks_processed,
        stats.total_duration.as_millis(),
        stats.space_efficiency()
    );
}

pub fn print_detailed(stats: &BuildStats) {
    // Build Summary section with comfy-table
    // Block Details table with columns: Block, Address Range, Used/Alloc, Efficiency, CRC Value
}

// Helper functions:
fn format_bytes(bytes: usize) -> String { /* with thousand separators */ }
fn format_address_range(start: u32, allocated: u32) -> String { /* 0x1000-0x10FF */ }
fn format_efficiency(used: u32, allocated: u32) -> String { /* 76.2% */ }
```

Functions:
- `print_summary()` - Brief one-line summary (default mode)
- `print_detailed()` - Detailed tables using comfy-table (--stats mode)
- Helper formatters for bytes, addresses, percentages

### 5. `src/main.rs`

Modify main flow:
1. Record overall start time with `Instant::now()`
2. Receive `BuildStats` from build functions
3. Set `stats.total_duration` to elapsed time
4. Display output based on flags:
   - If `args.output.quiet`: print nothing on success
   - If `args.output.stats`: call `printer::print_detailed(&stats)`
   - Otherwise: call `printer::print_summary(&stats)`
5. Always print errors regardless of quiet flag

Add at top:
```rust
mod printer;
```

### 6. `src/output/args.rs`

Add two new boolean fields to `OutputArgs` struct for verbosity control.

### 7. `src/lib.rs`

Add printer module:
```rust
pub mod printer;
```

## Output Format

Default output:
```
✓ Built 3 blocks in 127ms (81.2% efficiency)
```

With `--stats`:
```
Build Summary
═══════════════════════════════════════
Build Time         │ 127ms
Blocks Processed   │ 3
Total Allocated    │ 1,536 bytes  
Total Used         │ 1,247 bytes
Space Efficiency   │ 81.2%

Block Details  
═══════════════════════════════════════
Block     │ Address Range    │ Used/Alloc │ Efficiency │ CRC Value
motor     │ 0x1000-0x10FF    │ 195/256    │ 76.2%      │ 0x12345678
sensor    │ 0x2000-0x20FF    │ 132/256    │ 51.6%      │ 0x9ABCDEF0
config    │ 0x3000-0x33FF    │ 920/1024   │ 89.8%      │ 0x55AA55AA
```

## Testing

### Update Existing Tests

**`tests/smoke.rs`:**
- Update to handle new return types (`BlockStat` and `BuildStats`)
- Verify builds succeed without validating stat contents

**`tests/common/mod.rs`:**
- Update helper functions for new return types

### New Test File: `tests/statistics.rs`

Test cases:

1. `test_block_stat_collection` - Verify BlockStat contains expected values
2. `test_build_stats_aggregation` - Verify BuildStats aggregates correctly
3. `test_space_efficiency_calculation` - Test efficiency percentage calculation
4. `test_combined_mode_stats` - Verify stats work with --combined flag

Output testing can be done manually during development.

## Implementation Order

1. Add comfy-table dependency to Cargo.toml
2. Add CLI flags to `src/output/args.rs`
3. Add stats structs to `src/commands/mod.rs`
4. Update `DataRange` struct in `src/output/mod.rs`
5. Update `bytestream_to_datarange()` to calculate and populate size fields
6. Modify `build_block_single()` to collect and return BlockStat
7. Update `build_separate_blocks()` and `build_single_file()` to return BuildStats
8. Create `src/printer.rs` with formatting functions
9. Modify `src/main.rs` to track timing and display stats
10. Update existing tests
11. Create `tests/statistics.rs` with test coverage
12. Run `cargo fmt` and `cargo test`

## Success Criteria

- All existing tests pass
- Default output shows one-line summary with block count, time, efficiency
- `--stats` shows detailed two-section table
- `--quiet` suppresses all non-error output
- Stats collection uses existing data (no performance overhead)
- Clean separation: data collection in modules, formatting in printer
- Foundation ready for Phase 2 (JSON output, graphical visualization)

### To-dos

- [ ] Add comfy-table dependency to Cargo.toml
- [ ] Add --stats and --quiet flags to OutputArgs
- [ ] Add BlockStat and BuildStats structs to src/commands/mod.rs
- [ ] Add used_size and allocated_size fields to DataRange struct
- [ ] Update bytestream_to_datarange() to calculate size fields internally
- [ ] Modify build_block_single() to collect and return BlockStat
- [ ] Update build_separate_blocks() and build_single_file() to return BuildStats
- [ ] Create src/printer.rs with print_summary() and print_detailed()
- [ ] Modify main.rs to track timing and conditionally display statistics
- [ ] Update tests/smoke.rs and tests/common/mod.rs for new return types
- [ ] Create tests/statistics.rs with test coverage
- [ ] Run cargo fmt and cargo test, verify all tests pass