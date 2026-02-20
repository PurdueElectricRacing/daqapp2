{
  description = "Dev shell for daqapp2";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      system = "x86_64-linux";
      overlays = [ (import rust-overlay) ];
      pkgs = import nixpkgs {
        inherit system overlays;
      };
    in {
      devShells.${system}.default = pkgs.mkShell {
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
    };
}