{
  description = "A devShell example";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        pkgs = import nixpkgs { inherit system overlays; };
      in {

        devShells.default = pkgs.mkShell {
          buildInputs = [ rust ] ++ (with pkgs; [
            pkg-config
            probe-rs-tools
            cargo-make
            ravedude
            lazygit
            libudev-zero
            pkg-config
            picotool
            fish
            pkgsCross.avr.buildPackages.gcc
          ]);
        };

        programs.vscode = {
          enable = true;
          extensions = with pkgs.vscodeExtensions;
            [ rust-lang.rust-analyzer ]
            ++ pkgs.vscode-utils.extensionsFromVscodeMarketplace [{
              name = "probe-rs-debugger";
              publisher = "probe-rs";
              version = "0.21.2";
              sha256 = "0x82727qdrz2vf279n2vrzsi4bbyal8w3w8aqm0h9jmxd05y7f9x";
            }];
        };
      });
}
