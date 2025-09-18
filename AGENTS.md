# AGENTS Overview

Nvmbuilder is an embedded development tool that works with layout files (toml/yaml/json) and excel sheets to assemble, diff, export, sign (and more) static hex files for flashing to microcontrollers.

---

## Tools

Always run `cargo test` as a final check after making any changes.

Remember to use formatting tools to clean up your code once finished.

use `ast-grep` for semantic code search and better refactoring. Prefer codifying changes as ast-grep rules before sweeping edits to keep refactors reproducible.

## Working Guidelines

Make the minimum changes required to achieve your task; that does not mean skip parts or leave placeholders, but it means you should not add more than is asked for. You should instead allocate more time to planning so that you can provide a superior solution. If you are ever unsure of the goals or requirements of your task, you should pause your changes and provide the user with an update on your progress, and ask for clarification on the parts that aren't clear.

## Compatibility Policy

Do not maintain or even reference backwards compatibility unless explicitly required by the issue. We will break compatibility to deliver better functionality. Users are expected to adapt layouts/usage to adopt newer features and improvements.
