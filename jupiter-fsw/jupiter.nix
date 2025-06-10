{ rustPlatform, fetchFromGitHub, pkg-config, libudev-zero, systemd, libgpiod }:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "jupiter-fsw";
  version = "0.1.1";

  nativeBuildInputs = [ pkg-config libudev-zero ];

  buildInputs = [ systemd libgpiod ];

  src = fetchFromGitHub {
    owner = "Terminus-Suborbital-Research-Program";
    repo = "AMALTHEA";
    rev = "v0.1.1";
    fetchSubmodules = true;
    hash = "sha256-bFDU4HTSjYYyJQoNjb5eZUbjTeCN0T9FixVGzXHISLE=";
  };

  buildFeatures = [ "packet_logging" ];

  cargoHash = "sha256-N0N3SPfofK4pfurJb60zew731LqvQeflF2XK1fJwIGU=";

  doCheck = false;
})
