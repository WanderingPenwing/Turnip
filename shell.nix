{ pkgs ? import <nixpkgs> { overlays = [ (import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz)) ]; },}:
with pkgs;

mkShell {
  nativeBuildInputs = with xorg; [
    pkg-config
  ] ++ [
    cargo
    rustc
  ];
  buildInputs = [
    latest.rustChannels.stable.rust
  ];
}
