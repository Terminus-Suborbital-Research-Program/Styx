{
  description = "Dev shell for TERMINUS's rust crates";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils, ... }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        devShell = with pkgs; mkShell {
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
            libudev-zero
            ansible
            pkg-config
          ];

          RUST_SRC_PATH = rustPlatform.rustLibSrc;
          
          shellHook = ''
              export PATH=$PATH:''${CARGO_HOME:-~/.cargo}/bin
              export PATH=$PATH:''${RUSTUP_HOME:-~/.rustup}/toolchains/$RUSTC_VERSION-x86_64-unknown-linux-gnu/bin/
              rustup default stable
              rustup target add thumbv8m.main-none-eabihf
              rustup target add thumbv6m-none-eabi
            '';
        };

        programs.vscode = {
          enable = true;
          extensions = with pkgs.vscodeExtensions; [
            rust-lang.rust-analyzer
          ];
        };
      }
    );
}
