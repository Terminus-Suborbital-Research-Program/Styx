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
            picotool
          ];

          RUST_SRC_PATH = rustPlatform.rustLibSrc;
          
          shellHook = ''
              export PATH=$PATH:''${CARGO_HOME:-~/.cargo}/bin
              export PATH=$PATH:''${RUSTUP_HOME:-~/.rustup}/toolchains/$RUSTC_VERSION-x86_64-unknown-linux-gnu/bin/
              rustup default stable
              rustup target add thumbv8m.main-none-eabihf
            '';
        };

        programs.vscode = {
          enable = true;
          extensions = with pkgs.vscodeExtensions; [
            rust-lang.rust-analyzer
          ] ++ pkgs.vscode-utils.extensionsFromVscodeMarketplace [
	    {
	      name = "probe-rs-debugger";
	      publisher = "probe-rs";
	      version = "0.21.2";
	      sha256 = "0x82727qdrz2vf279n2vrzsi4bbyal8w3w8aqm0h9jmxd05y7f9x";
	    }
	  ];
        };
      }
    );
}
