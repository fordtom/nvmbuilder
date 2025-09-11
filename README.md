## nvmbuilder

Build flash blocks from a layout file (TOML/YAML/JSON) and an Excel workbook, then emit Intel HEX files.

### Usage

```bash
nvmbuilder <BLOCK>... -l <LAYOUT> -x <XLSX> \
  [-v <VARIANT>] [-d] [-o <DIR>] \
  [--prefix <STR>] [--suffix <STR>] [--record-width N] [--pad-to-end]

```

- **BLOCK**: one or more block names to build (positional)
- **-l, --layout FILE**: path to layout (`.toml`, `.yaml`/`.yml`, or `.json`) [required]
- **-x, --xlsx FILE**: path to Excel workbook containing values/variants [required]
- **-v, --variant NAME**: column in the workbook to use for variants (optional)
- **-d, --debug**: prefer the Debug column when present (optional)
- **-o, --out DIR**: output directory for `.hex` files (default: `out`)
- Offset is configured in the layout file as a top-level `offset` field.
- **--prefix STR**: Optional string prepended to block name in output filename
- **--suffix STR**: Optional string appended to block name in output filename
- **--record-width N**: number of bytes per HEX data record (default: 32; range 1..=64)
 - **--pad-to-end**: pad the output HEX to the full block length (default: off)

The order of preference for value selection is debug -> variant -> default. Ensure you always have default filled. Strings in the excel can point to different sheets as a way of providing arrays.

### Examples

Examples live in the `examples/` directory.

```bash
# Single block from TOML layout
nvmbuilder block -l examples/block.toml -x examples/data.xlsx -o out

# Multiple blocks with a variant and debug values
nvmbuilder blockA blockB -l examples/block.toml -x examples/data.xlsx -v VarA -d -o out

# Using YAML or JSON layouts
nvmbuilder block -l examples/block.yaml -x examples/data.xlsx -o out
nvmbuilder block -l examples/block.json -x examples/data.xlsx -o out
```

Outputs are written to the chosen directory as `{prefix}_{block}_{suffix}.hex`, omitting empty parts and their underscores. Examples: `block.hex`, `PRE_block.hex`, `block_SUF.hex`, `PRE_block_SUF.hex`.


