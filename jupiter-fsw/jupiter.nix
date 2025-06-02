{ lib, rustPlatform }:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "jupiter-fsw";
  version = "0.0.1";

  cargoHash = lib.fakeHash;
})
