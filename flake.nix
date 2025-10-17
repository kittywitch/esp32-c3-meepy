{
  inputs = {
    esp-rs-nix = {
      url = "github:leighleighleigh/esp-rs-nix";
    };
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };
  outputs = { nixpkgs, flake-utils, esp-rs-nix, ... }@inputs: let
    mkPkgs = system: (import nixpkgs) {
      inherit system;
    };
    eachSystemOutputs = flake-utils.lib.eachDefaultSystem (system: let
      pkgs = mkPkgs system;
      inherit (esp-rs-nix.packages.${system}) esp-rs;
      esp-shell = pkgs.mkShell rec {
          name = "esp-shell";

          buildInputs = [
              esp-rs 
              pkgs.rustup 
              pkgs.espflash
              #pkgs.rust-analyzer
              pkgs.pkg-config 
              pkgs.stdenv.cc 
              #pkgs.bacon 
              #pkgs.systemdMinimal
              #pkgs.lunarvim 
              #pkgs.inotify-tools
              #pkgs.picocom
              #pkgs.vscode-fhs
              pkgs.libusb1
              pkgs.python3
              # Workspace command runners
              pkgs.just
              pkgs.mprocs
              # This is for parameterising the justfile
              pkgs.toml-cli
              pkgs.moreutils
              pkgs.gdb
          ];

          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";

          shellHook = ''
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
