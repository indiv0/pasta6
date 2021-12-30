{ pkgs ? import <nixpkgs> {} }:
let
  fenix = import (fetchTarball "https://github.com/nix-community/fenix/archive/main.tar.gz") {};
in
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    (
      with fenix;
      combine (
        with default; [
	  cargo
	  clippy-preview
	  rustc
	  rustfmt-preview

	  # Add rustlang support via rust-analyzer for your IDE of choice.
	  rust-analyzer
	  complete.rust-src
	]
      )
    )

    # Fast compile dependencies.
    # See: https://bevyengine.org/learn/book/getting-started/setup/
    clang
    lld
  ];

  # `LIBCLANG_PATH` must be set for `rust-bindgen` to work. `rust-bindgen` is
  # necessary to generate bindings to `linux/io_uring.h`.
  shellHook = ''export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"'';
}
