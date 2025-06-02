{ lib, rustPlatform, git }:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "jupiter-fsw";
  version = "0.0.1";

  src = ./..;

  cargoHash = "sha256-zOrvDUZ5gIlroafu7IntT6SmoBnpNg+k/vfVQw04TdI=";
})
