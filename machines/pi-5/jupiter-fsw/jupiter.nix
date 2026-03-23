{ rustPlatform, fetchFromGitHub, pkg-config, libudev-zero, systemd, libgpiod }:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "jupiter-fsw";
  version = "0.1.1";

  nativeBuildInputs = [ pkg-config libudev-zero ];

  buildInputs = [ systemd libgpiod ];

  inherit src;

  PYLON_ROOT = "${basler-pylon}/opt/pylon";
  buildAndTestSubdir = "machines/pi-5/jupiter-fsw";

  postInstall = ''
    wrapProgram $out/bin/odin-compute \
      --set GENICAM_GENTL64_PATH "${basler-pylon}/opt/pylon/lib/gentlproducer/gtl"
  '';

  buildFeatures = [ "packet_logging" ];

  cargoHash = "sha256-N0N3SPfofK4pfurJb60zew731LqvQeflF2XK1fJwIGU=";

  doCheck = false;
})
