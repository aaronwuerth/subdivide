{
  description = "Subdivide faces of a PLY mesh";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      nixpkgs,
      crane,
      rust-overlay,
      ...
    }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
      };

      rust-toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

      craneLib = (crane.mkLib pkgs).overrideToolchain rust-toolchain;

      src = craneLib.cleanCargoSource ./.;

      commonArgs = {
        inherit src;
        strictDeps = true;
      };

      cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      subdividePkg = craneLib.buildPackage (
        commonArgs
        // {
          inherit cargoArtifacts;
        }
      );
    in
    {
      checks.${system} = {
        inherit subdividePkg;

        clippy = craneLib.cargoClippy (
          commonArgs
          // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          }
        );

        fmt = craneLib.cargoFmt {
          inherit src;
        };

        deny = craneLib.cargoDeny {
          inherit src;
        };
      };

      packages.${system} = {
        default = subdividePkg;
      };

      devShells.${system}.default =
        let
          toolchain = rust-toolchain.override (prev: {
            extensions = prev.extensions ++ [ "rust-analyzer" ];
          });
        in
        (craneLib.overrideToolchain toolchain).devShell {
          env.RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library";
        };
    };
}
