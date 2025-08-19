{
  description = "Dev shell with Rust toolchain and Alejandra, pinned to nixpkgs 25.05 stable";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rustc
            cargo
            rustfmt
            clippy
            alejandra
          ];
          shellHook = ''
            echo "Entering dev shell (Rust + Alejandra) from nixpkgs 25.05 — system: ${system}"
          '';
        };

        formatter = pkgs.alejandra;
      }
    );
}