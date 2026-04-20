{ pkgs, src, lib,fetchFromGitHub ,basler-pylon }:

let
  # isolatedSrc = pkgs.runCommand "isolated-styx-src" {} ''
  #   cp -r ${src} $out
  #   chmod -R +w $out
  #   ls $out
  #   rm -f $out/Cargo.toml
  #   rm -f $out/Cargo.lock
  # '';
  soapyextra = pkgs.soapysdr.override {
    extraPackages = [ 
      pkgs.soapyairspy 
      pkgs.soapyrtlsdr 
    ];
  };
in
pkgs.rustPlatform.buildRustPackage {
  pname = "jupiter-fsw";
  version = "0.1.1";
  
  # lobotomized source
  # src = isolatedSrc;
  # inherit src;


  # sourceRoot = "isolated-styx-src/machines/pi-5/jupiter-fsw";
  # src = lib.cleanSource src;
  # src = pkgs.runCommand "styx-raw-src" {} ''
  #   cp -r ${src} $out
  #   chmod -R +w $out
  # '';
  src = fetchFromGitHub {
    owner = "Terminus-Suborbital-Research-Program";
    repo = "Styx";
    rev = "refs/heads/Basler-Nix";
    fetchSubmodules = true;
    hash = "sha256-bEQs09P5DFCsKCir93sd3Bl847Tlzz0DrfxHR3DLQno=";
  };

  nativeBuildInputs = [ 
    pkgs.pkg-config 
    pkgs.makeWrapper
    pkgs.cargo-make
    pkgs.rustPlatform.bindgenHook
  ];

  buildInputs = [ 
    pkgs.systemd 
    pkgs.libgpiod 
    pkgs.libusb1
    pkgs.zlib
    pkgs.clang              
    pkgs.llvmPackages.libclang 
    pkgs.linuxHeaders
    basler-pylon 
    soapyextra        
    pkgs.airspy
  ];

  LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
  BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.linuxHeaders}/include -I${pkgs.glibc.dev}/include";
  SOAPY_SDR_PLUGIN_PATH = "${soapyextra}/lib/SoapySDR/modules0.8";

  CARGO_PROFILE_RELEASE_LTO = "false";
  CARGO_PROFILE_RELEASE_CODEGEN_UNITS = "16";

  buildPhase = ''
    runHook preBuild
    cargo make --profile release build-host
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    
    mkdir -p $out/bin
    cp target/release/jupiter-fsw $out/bin/ 2>/dev/null || cp machines/pi-5/jupiter-fsw/target/release/jupiter-fsw $out/bin/
    
    runHook postInstall
  '';

  PYLON_ROOT = "${basler-pylon}/opt/pylon";

  postInstall = ''
    wrapProgram $out/bin/jupiter-fsw \
      --set GENICAM_GENTL64_PATH "${basler-pylon}/opt/pylon/lib/gentlproducer/gtl" \
      --set PYLON_GENTL64_PATH "${basler-pylon}/opt/pylon/lib/gentlproducer/gtl" \
      --set LD_LIBRARY_PATH "${basler-pylon}/opt/pylon/lib:${pkgs.lib.makeLibraryPath [ pkgs.libusb1 pkgs.zlib pkgs.stdenv.cc.cc.lib ]}"
  '';

  buildFeatures = [ "packet_logging" ];
  # cargoLock = {
    # lockFile = ./Cargo.lock;
  # };

  cargoHash = "sha256-PpX3aP6p/8MBoOJHsih2sYbCYVV5m13PwMFect838AM=";
  doCheck = false;
  auditable = false;
}