{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        mkMoldShell = pkgs.mkShell.override{
          stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.clangStdenv;
        };
      in
      with pkgs;
      {
        devShells.default = mkMoldShell {
          buildInputs = [
            (rust-bin.stable.latest.default.override { extensions = [ "rust-src" "rust-analyzer" ]; })
            cmake
            bacon
            pkg-config
            glib
            fontconfig
            atk
            gtk3
            alsa-lib
            systemd
            openssl
            just
          ];

          LD_LIBRARY_PATH = lib.makeLibraryPath [ 
            vulkan-loader 
            libGL
            libxkbcommon
            wayland
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
          ];

          NIX_LD_LIBRARY_PATH = lib.makeLibraryPath [
          ];
        };
      }
    );
}
