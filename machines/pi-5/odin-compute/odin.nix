{ rustPlatform, pkg-config, libudev-zero, systemd, libgpiod, makeWrapper, basler-pylon, src }:

rustPlatform.buildRustPackage {
  pname = "odin-compute";
  version = "0.1.1";

  inherit src;

  nativeBuildInputs = [ pkg-config makeWrapper ];
  buildInputs = [ systemd libgpiod libudev-zero basler-pylon ];

  PYLON_ROOT = "${basler-pylon}/opt/pylon";
  buildAndTestSubdir = "machines/pi-5/odin-compute";
  cargoHash = "sha256-N0N3SPfofK4pfurJb60zew731LqvQeflF2XK1fJwIGU=";

  postInstall = ''
    wrapProgram $out/bin/odin-compute \
      --set GENICAM_GENTL64_PATH "${basler-pylon}/opt/pylon/lib/gentlproducer/gtl"
  '';

  doCheck = false;
}