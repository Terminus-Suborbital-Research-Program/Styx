{
  description = "Dev flake for TERMINUS's rust crates + Jupiter-FSW service";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, utils, ... }:
    let
      systems = utils.lib.eachDefaultSystem (system:
        let
          pkgs = import nixpkgs { inherit system; };
          jupiter-fsw = pkgs.callPackage ./jupiter-fsw/jupiter.nix { };
        in {
          packages = { inherit jupiter-fsw; };
          devShell.default = pkgs.mkShell {
            buildInputs = with pkgs; [
              libiconv
              rustup
              libgpiod
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
          };
        });
    in {
      packages = systems.packages;
      devShells = systems.devShell;
    };
}
