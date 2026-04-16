{ pkgs, src, basler-pylon }:

let
  isolatedSrc = pkgs.runCommand "isolated-styx-src" {} ''
    cp -r ${src} $out
    chmod -R +w $out
    
    rm -f $out/Cargo.toml
    rm -f $out/Cargo.lock
  '';
in
pkgs.rustPlatform.buildRustPackage {
  pname = "jupiter-fsw";
  version = "0.1.1";
  
  # lobotomized source
  src = isolatedSrc;

  sourceRoot = "isolated-styx-src/machines/pi-5/jupiter-fsw";

  cargoHash = "sha256-Arm07Nc2+ldNvV8fSbVmycGdxDM8wnL8byQDv8WvoBE="; 

  nativeBuildInputs = [ 
    pkgs.pkg-config 
    pkgs.makeWrapper
  ];

  buildInputs = [ 
    pkgs.systemd 
    pkgs.libgpiod 
    pkgs.libusb1
    pkgs.zlib
    basler-pylon 
  ];

  PYLON_ROOT = "${basler-pylon}/opt/pylon";

  postInstall = ''
    wrapProgram $out/bin/jupiter-fsw \
      --set GENICAM_GENTL64_PATH "${basler-pylon}/opt/pylon/lib/gentlproducer/gtl" \
      --set PYLON_GENTL64_PATH "${basler-pylon}/opt/pylon/lib/gentlproducer/gtl" \
      --set LD_LIBRARY_PATH "${basler-pylon}/opt/pylon/lib:${pkgs.lib.makeLibraryPath [ pkgs.libusb1 pkgs.zlib pkgs.stdenv.cc.cc.lib ]}"
  '';

  buildFeatures = [ "packet_logging" ];

  cargoHash = "sha256-M3vbkixpirKhxSIiEGIhqGe7+VsEFunzREzbD4yHPrk=";

  doCheck = false;
}