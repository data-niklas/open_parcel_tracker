{
  description = "A Nix-flake-based Rust development environment";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1.*.tar.gz";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
  }: let
    overlays = [
      rust-overlay.overlays.default
      (final: prev: {
        rustToolchain = prev.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      })
    ];
    supportedSystems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];
    forEachSupportedSystem = f:
      nixpkgs.lib.genAttrs supportedSystems (system:
        f {
          pkgs = import nixpkgs {inherit overlays system;};
        });
  in {
    # defaultPackage = forEachSupportedSystem ({pkgs}:
    #   pkgs.rustPlatform.buildRustPackage rec {
    #     pname = "parcel_tracker";
    #     version = "0.1.0";
    #     src = ./.;
    #     cargoLock = {
    #       lockFile = ./Cargo.lock;
    #       outputHashes = {
    #       };
    #     };
    #     env = {
    #     };
    #   });
    devShells = forEachSupportedSystem ({pkgs}: {
      default = pkgs.mkShell rec {
        packages = with pkgs; [
          rustToolchain
          openssl
          pkg-config
          rust-analyzer
          bacon
          stdenv.cc.cc.lib
        ];
        LD_LIBRARY_PATH = nixpkgs.lib.makeLibraryPath packages;
      };
    });
  };
}
