# AGENTS Overview

Nvmbuilder is an embedded development tool that works with layout files (toml/yaml/json) and excel sheets to assemble, diff, export, sign (and more) static hex files for flashing to microcontrollers.

---

## Tools

Always run `cargo test` as a final check after making any changes.

Remember to use formatting tools to clean up your code once finished.

use `ast-grep` for semantic code search and better refactoring. Prefer codifying changes as ast-grep rules before sweeping edits to keep refactors reproducible.

---

## Compatibility Policy

Do not maintain or even reference backwards compatibility unless explicitly required by the issue. We will break compatibility to deliver better functionality. Users are expected to adapt layouts/usage to adopt newer features and improvements.
