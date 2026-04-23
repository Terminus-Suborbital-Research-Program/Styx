{ pkgs, src, lib,fetchFromGitHub ,basler-pylon }:
let
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
  
  src = fetchFromGitHub {
    owner = "Terminus-Suborbital-Research-Program";
    repo = "Styx";
    rev = "refs/heads/Basler-Nix";
    fetchSubmodules = true;
    hash = "sha256-IFCojXWRR638uhgaj/TYxmAcR/3+rw/MtxhBzwCK1YM=";
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

  cargoBuildFlags = [ "-j" "2" ];

  
  # cargo make --profile release build-host -- --jobs 2 --target ${pkgs.stdenv.hostPlatform.rust.rustcTarget}  

  buildPhase = ''
    runHook preBuild

    sed -i 's/HOST_PACKAGES = "-p jupiter-fsw -p odin-compute -p munin"/HOST_PACKAGES = "-p jupiter-fsw -p odin-compute -p munin --release --jobs 2 --target ${pkgs.stdenv.hostPlatform.rust.rustcTarget}"/g' Makefile.toml
    
    export RUSTFLAGS="-C lto=off -C codegen-units=16 $RUSTFLAGS"
    cargo make build-host
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    
    mkdir -p $out/bin
    cp target/${pkgs.stdenv.hostPlatform.rust.rustcTarget}/release/jupiter-fsw $out/bin/ 2>/dev/null || cp machines/pi-5/jupiter-fsw/target/${pkgs.stdenv.hostPlatform.rust.rustcTarget}/release/jupiter-fsw $out/bin/    
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

  cargoHash = "sha256-QBiPk6Yh8b+jWLLYkvhkO+RoikiwaUHe9MaMCF/ysrY=";
  doCheck = false;
  auditable = false;
}