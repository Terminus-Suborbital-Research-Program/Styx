{ rustPlatform, fetchFromGitHub, pkg-config, libudev-zero, systemd, libgpiod }:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "jupiter-fsw";
  version = "0.0.3";

  nativeBuildInputs = [ pkg-config libudev-zero ];

  buildInputs = [ systemd libgpiod ];

  src = fetchFromGitHub {
    owner = "Terminus-Suborbital-Research-Program";
    repo = "AMALTHEA";
    rev = "v0.0.3";
    fetchSubmodules = true;
    hash = "";
  };

  cargoHash = "sha256-zOrvDUZ5gIlroafu7IntT6SmoBnpNg+k/vfVQw04TdI=";

  doCheck = false;
})
