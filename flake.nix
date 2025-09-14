{
  description = "Tailwind-rs CSS extractor and tranform plugin";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    # Rust toolchain management
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # Modern Rust build system for Nix
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, crane }:
    let
      # Systems to build for
      systems = [ "x86_64-linux" "aarch64-linux" ];
      overlays = [
        rust-overlay.overlays.default
        (final: prev: {
	  openssl-static = prev.openssl.override { static = true; };
          # Rust toolchain with LLVM tools for LTO
          rustToolchain =
            let
              # For cross-compilation, we need the build platform's rust with the target platform added
              targets = [
                final.stdenv.hostPlatform.config
              ];
            in
              # use buildPackages rust with target platform to allow
              # cross-compilation
              final.buildPackages.rust-bin.stable.latest.default.override {
                extensions = [ "llvm-tools-preview" ];
                inherit targets;
              };

          # Crane library for regular builds
          craneLib = (crane.mkLib final).overrideToolchain (p: final.rustToolchain);

          # Common build arguments
          commonBuildArgs = rec {
            src = ./.;
            strictDeps = true;

            donStrip = false;

            nativeBuildInputs = with final.buildPackages; [
              pkg-config
              cmake
              python3
              cacert
            ];
            doCheck = true;
          };

          # Build dependencies separately for caching
          cargoArtifacts = final.craneLib.buildDepsOnly final.commonBuildArgs;

          tailwind-extractor-cli = final.craneLib.buildPackage (final.commonBuildArgs // {
            inherit (final) cargoArtifacts;
            pname = "tailwind-extractor-cli";
            doCheck = final.hostPlatform == final.buildPlatform;
            # Build command
            cargoBuildCommand = "cargo build --release --bin tailwind-extractor-cli";
            stripAllList = ["bin"];
          });
        })
      ];

    in flake-utils.lib.eachSystem systems (system:
    let
      pkgs =
        import nixpkgs {
          inherit system overlays;
        };
      staticFrom = crossPkgs: crossPkgs.tailwind-extractor-cli;
    in
    rec {
      legacyPackages = pkgs;

      # Package outputs for each system
      packages = rec {
          default = native;
          native = packages.${system};
          x86_64-linux = staticFrom pkgs.pkgsCross.musl64;
          aarch64-linux = staticFrom pkgs.pkgsCross.aarch64-multiplatform-musl;
        };
      apps.build-static-binaries = flake-utils.lib.mkApp {
        drv = pkgs.writeShellApplication {
          name = "build-static-binaries";
          text =
            pkgs.lib.concatMapStringsSep "\n" (arch: ''
              mkdir -p "./bins/${arch}"
              cp -fv \
                ${builtins.getAttr arch packages}/bin/tailwind-extractor-cli \
                ./bins/${arch}/tailwind-extractor-cli
            '') systems;
        };
      };

      # Development shells
      devShells =
        let
          rustToolchain = pkgs.rust-bin.stable.latest.default.override {
            extensions = [ "rust-src" "rust-analyzer" "llvm-tools-preview" ];
            targets = [
              "${pkgs.stdenv.hostPlatform.config}"
            ];
          };
        in
        {
          default = pkgs.mkShell {
            buildInputs = with pkgs; [
              rustToolchain

              # Build tools
              pkg-config
              cmake
              python3

              # Development tools
              cargo-watch
              cargo-nextest
              valgrind
              hyperfine
              wrk

              # JavaScript ecosystem
              nodejs
              nodePackages.npm
            ];
          };
        };

      checks =
        let
          craneLib = (crane.mkLib pkgs).overrideToolchain (p:
            p.rust-bin.stable.latest.default
          );
        in
        {
          # Format check
          fmt = craneLib.cargoFmt {
            src = ./.;
          };

          # Clippy lints
          clippy = craneLib.cargoClippy {
            src = ./.;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          };
        };
    });
}
