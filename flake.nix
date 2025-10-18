{
  inputs = {
    esp-rs-nix = {
      url = "github:leighleighleigh/esp-rs-nix";
    };
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };
  outputs = { nixpkgs, flake-utils, esp-rs-nix, ... }@inputs: let
    arch = "riscv32imc-unknown-none-elf";
    mkPkgs = system: (import nixpkgs) {
      inherit system;
      rust.rustcTarget = arch;
    };
    eachSystemOutputs = flake-utils.lib.eachDefaultSystem (system: let
      pkgs = mkPkgs system;
      lib = pkgs.lib;
      inherit (esp-rs-nix.packages.${system}) esp-rs;
      inherit (lib.meta) getExe;
      buildy = pkgs.writeShellScriptBin "build" ''
        set -euox pipefail
        PACKAGE_NAME="$(${getExe pkgs.toml-cli} get -r Cargo.toml package.name)"
        EXE="''${PROJECT_DIR}/target/${arch}/release/''${PACKAGE_NAME}"
        cargo build --release &&
        sudo espflash flash ''${EXE}
      '';
      esp-shell = pkgs.mkShell rec {
          name = "esp-shell";

          buildInputs = [
              esp-rs
              buildy
              pkgs.rustup
              pkgs.espflash
              pkgs.clippy
              pkgs.pkg-config
              pkgs.stdenv.cc
              pkgs.libusb1
              pkgs.python3
              pkgs.mprocs
              pkgs.toml-cli
              pkgs.moreutils
              pkgs.gdb
          ];

          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";

          shellHook = ''
          export PATH="${lib.makeBinPath [pkgs.rust-analyzer]}:$PATH"
          export PROJECT_DIR="$(pwd)";
          # custom bashrc stuff
          export PS1_PREFIX="(esp-rs)"
          . ~/.bashrc

          export LD_LIBRARY_PATH="''${LD_LIBRARY_PATH}:${LD_LIBRARY_PATH}"
          # this is important - it tells rustup where to find the esp toolchain,
          # without needing to copy it into your local ~/.rustup/ folder.
          export RUSTUP_TOOLCHAIN=${esp-rs}

          # Load shell completions for espflash
          if (which espflash >/dev/null 2>&1); then
          . <(espflash completions $(basename $SHELL))
          fi
          '';
      };
    in {
      devShells = {
        inherit esp-shell;
        default = esp-shell;
      };
    });
    in eachSystemOutputs;
}
