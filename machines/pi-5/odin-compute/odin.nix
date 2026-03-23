{ rustPlatform, pkg-config, libudev-zero, systemd, libgpiod, makeWrapper,  src }:

rustPlatform.buildRustPackage {
  pname = "odin-compute";
  version = "0.1.1";

  inherit src;

  nativeBuildInputs = [ pkg-config makeWrapper ];
  buildInputs = [ systemd libgpiod libudev-zero ];

  buildAndTestSubdir = "machines/pi-5/odin-compute";
  cargoHash = "sha256-N0N3SPfofK4pfurJb60zew731LqvQeflF2XK1fJwIGU=";

  doCheck = false;
}