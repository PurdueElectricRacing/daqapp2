{
  description = "Dev shell for PurdueElectricRacing/daqapp2";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            pkg-config
            systemd
            udev
            wayland
            wayland-protocols
            libxkbcommon
            mesa
            libGL

            (rust-bin.stable.latest.default.override {
              extensions = [ "rust-src" "clippy" "rustfmt" ];
            })
          ];
        };
      });
}