{ rustPlatform, fetchFromGitHub, pkg-config, libudev-zero, systemd, libgpiod }:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "jupiter-fsw";
  version = "0.0.1";

  nativeBuildInputs = [ pkg-config libudev-zero ];

  buildInputs = [ systemd libgpiod ];

  src = fetchFromGitHub {
    owner = "Terminus-Suborbital-Research-Program";
    repo = "AMALTHEA";
    rev = "v0.0.2";
    fetchSubmodules = true;
    hash = "sha256-o64hjQwn7knPU/icH9dbhJGQXQVHaKnSSF0PiYpbkMU=";
  };

  cargoHash = "sha256-zOrvDUZ5gIlroafu7IntT6SmoBnpNg+k/vfVQw04TdI=";

  doCheck = false;
})
