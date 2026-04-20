{ pkgs, src, basler-pylon }:

# let
  # isolatedSrc = pkgs.runCommand "isolated-styx-src" {} ''
  #   cp -r ${src} $out
  #   chmod -R +w $out
  #   ls $out
  #   rm -f $out/Cargo.toml
  #   rm -f $out/Cargo.lock
  # '';
# in
pkgs.rustPlatform.buildRustPackage {
  pname = "jupiter-fsw";
  version = "0.1.1";
  
  # lobotomized source
  # src = isolatedSrc;
  inherit src;

  sourceRoot = "isolated-styx-src/machines/pi-5/jupiter-fsw";


  nativeBuildInputs = [ 
    pkgs.pkg-config 
    pkgs.makeWrapper
    pkgs.cargo-make
  ];

  buildInputs = [ 
    pkgs.systemd 
    pkgs.libgpiod 
    pkgs.libusb1
    pkgs.zlib
    basler-pylon 
  ];

  # preBuild = ''
  #   cargo make my-custom-setup-task
  # '';

  buildPhase = ''
    runHook preBuild
    cargo make --profile release build-host
    runHook postBuild
  '';

  PYLON_ROOT = "${basler-pylon}/opt/pylon";

  postInstall = ''
    wrapProgram $out/bin/jupiter-fsw \
      --set GENICAM_GENTL64_PATH "${basler-pylon}/opt/pylon/lib/gentlproducer/gtl" \
      --set PYLON_GENTL64_PATH "${basler-pylon}/opt/pylon/lib/gentlproducer/gtl" \
      --set LD_LIBRARY_PATH "${basler-pylon}/opt/pylon/lib:${pkgs.lib.makeLibraryPath [ pkgs.libusb1 pkgs.zlib pkgs.stdenv.cc.cc.lib ]}"
  '';

  buildFeatures = [ "packet_logging" ];
  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  # cargoHash = "sha256-M3vbkixpirKhxSIiEGIhqGe7+VsEFunzREzbD4yHPrk=";

  doCheck = false;
}