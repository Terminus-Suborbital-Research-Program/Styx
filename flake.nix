{
  description = "Dev shell for TERMINUS's rust crates";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, utils, ... }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        jupiter-fsw = pkgs.callPackage ./jupiter-fsw/jupiter.nix { };
      in {
        packages = { inherit jupiter-fsw; };

        devShell = with pkgs;
          mkShell {
            buildInputs = [
              darwin.apple_sdk.frameworks.Security
              libiconv
              rustup
              gcc
              cargo
              rustc
              rustfmt
              rustPackages.clippy
              rust-analyzer
              probe-rs-tools
              cargo-make
              lazygit
              clippy
              rust-analyzer
              ravedude
              libudev-zero
              ansible
              pkg-config
              picotool
              cargo-machete
              gource
              pkgsCross.avr.buildPackages.gcc
            ];

            RUST_SRC_PATH = rustPlatform.rustLibSrc;

            shellHook = ''
              export PATH=$PATH:''${CARGO_HOME:-~/.cargo}/bin
              export PATH=$PATH:''${RUSTUP_HOME:-~/.rustup}/toolchains/$RUSTC_VERSION-x86_64-unknown-linux-gnu/bin/
            '';
          };

      });
}
