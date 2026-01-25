{
  description = "Manse - Scrolling Window Manager for Terminals";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default;

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Filter source to include Cargo files and local dependencies
        src = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = path: type:
            # Include local crate dependencies
            (pkgs.lib.hasInfix "/egui_term" path) ||
            (pkgs.lib.hasInfix "/patches" path) ||
            (pkgs.lib.hasInfix "/assets" path) ||
            # Include .git for build.rs git hash extraction
            (pkgs.lib.hasInfix "/.git" path) ||
            (baseNameOf path == ".git") ||
            # Include standard Cargo files
            (craneLib.filterCargoSources path type);
        };

        # Common arguments for crane builds
        commonArgs = {
          inherit src;

          strictDeps = true;

          nativeBuildInputs = with pkgs; [
            pkg-config
            git # needed by build.rs
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            # New darwin SDK pattern - provides all frameworks
            pkgs.apple-sdk_15
            pkgs.libiconv
          ];

          buildInputs = with pkgs; [
            openssl
          ] ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
            # Linux GUI dependencies for eframe/wgpu
            libxkbcommon
            libGL
            wayland
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
            vulkan-loader
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            # New darwin SDK pattern - provides all frameworks
            pkgs.apple-sdk_15
          ];

          # Set dummy values for build.rs when git isn't available in sandbox
          BUILD_GIT_HASH = self.shortRev or self.dirtyShortRev or "unknown";
          BUILD_TIME = "nix-build";
        };

        # Build just the cargo dependencies for caching
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual crate
        manse = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;

          # Runtime library path for Linux
          postFixup = pkgs.lib.optionalString pkgs.stdenv.isLinux ''
            patchelf --set-rpath "${pkgs.lib.makeLibraryPath commonArgs.buildInputs}" $out/bin/manse
          '';

          meta = with pkgs.lib; {
            description = "Scrolling window manager for terminals";
            homepage = "https://github.com/yourusername/manse";
            license = licenses.mit;
            mainProgram = "manse";
          };
        });
      in
      {
        checks = {
          # Build the crate as part of checks
          inherit manse;

          # Run clippy
          manse-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          # Check formatting
          manse-fmt = craneLib.cargoFmt {
            inherit src;
          };

          # Run tests
          manse-test = craneLib.cargoTest (commonArgs // {
            inherit cargoArtifacts;
          });
        };

        packages = {
          default = manse;
          manse = manse;
        };

        apps.default = flake-utils.lib.mkApp {
          drv = manse;
        };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};

          packages = with pkgs; [
            rust-analyzer
            cargo-watch
          ];

          # Set library path for running locally on Linux
          LD_LIBRARY_PATH = pkgs.lib.optionalString pkgs.stdenv.isLinux
            (pkgs.lib.makeLibraryPath commonArgs.buildInputs);
        };
      });
}
