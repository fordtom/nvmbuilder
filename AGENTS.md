# AGENTS Overview

This document houses the canonical "Overview" rule text for project agents.

> Placeholder: The "overview" rule content was not found in this repository when this file was created. Paste the authoritative Overview rule text here.

---

## Note: ast-grep is available in the dev shell

`ast-grep` from nixpkgs is installed in the Nix dev shell. Use it for semantic code search and safe, rule-driven refactors.

- Quick check: `nix develop -c ast-grep --version | cat`
- Example scan: `ast-grep scan -p 'fn main' .`

Prefer codifying changes as ast-grep rules before sweeping edits to keep refactors reproducible.

