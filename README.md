## nvmbuilder

Build flash blocks from a layout file (TOML/YAML/JSON) and an Excel workbook, then emit Intel HEX files.

### Usage

```bash
nvmbuilder <BLOCK@FILE>... -x <XLSX> \
  [--main-sheet <NAME>] [-v <VARIANT>] [-d] [-o <DIR>] \
  [--prefix <STR>] [--suffix <STR>] [--record-width N]

```

- **BLOCK@FILE**: one or more block specs in the form `name@layout_file` (positional). `layout_file` may be `.toml`, `.yaml`/`.yml`, or `.json`.
- **-x, --xlsx FILE**: path to Excel workbook containing values/variants [required]
- **--main-sheet NAME**: main sheet name in Excel (default: `Main`)
- **-v, --variant NAME**: column in the workbook to use for variants (optional)
- **-d, --debug**: prefer the Debug column when present (optional)
- **-o, --out DIR**: output directory for `.hex` files (default: `out`)
- **--prefix STR**: Optional string prepended to block name in output filename
- **--suffix STR**: Optional string appended to block name in output filename
- **--record-width N**: number of bytes per HEX data record (default: 32; range 1..=64)
- The following are configured in the layout `settings` instead of CLI flags:
  - **byte_swap**: swap bytes in place across the payload and CRC (for TI)
  - **pad_to_end**: pad the output HEX to the full block length

The order of preference for value selection is debug -> variant -> default. Ensure you always have default filled. Strings in the excel can point to different sheets as a way of providing arrays.

### Examples

Examples live in the `examples/` directory.

```bash
# Single block from TOML layout
nvmbuilder block@examples/block.toml -x examples/data.xlsx -o out

# Multiple blocks with a variant and debug values
nvmbuilder blockA@examples/block.toml blockB@examples/block.toml -x examples/data.xlsx -v VarA -d -o out

# Using YAML or JSON layouts
nvmbuilder block@examples/block.yaml -x examples/data.xlsx -o out
nvmbuilder block@examples/block.json -x examples/data.xlsx -o out
```

Outputs are written to the chosen directory as `{prefix_}{block}{_suffix}.hex`.
