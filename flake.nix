{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { nixpkgs, rust-overlay, ... }:
    let
      system = "x86_64-linux";
      overlays = [ (import rust-overlay) ];
      pkgs = import nixpkgs { inherit system overlays; };
      mkMoldShell = pkgs.mkShell.override{
        stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.clangStdenv;
      };
    in
    with pkgs;
    {
      # unfortunately upx can't pack bins made with mold linker, so we have to switch back sometmes
      # devShells."${system}".default = mkShell {
      devShells."${system}".default = mkMoldShell {
        packages = [
          (rust-bin.stable.latest.default.override {
            extensions = [ "rust-src" "rust-analyzer" ];
          })
          just
          clang
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
          renderdoc
          gdb
          lldb
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
    };
}
