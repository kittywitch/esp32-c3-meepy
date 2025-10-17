arch := "riscv32imc-unknown-none-elf"
pakidg := shell('toml get -r Cargo.toml package.name')
exe := justfile_directory() / "target" / arch / "release" / pakidg

build:
  #!/usr/bin/env bash
  cargo build --release && \
  sudo espflash flash {{exe}}
