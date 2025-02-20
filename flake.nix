{
  description = "A Nix-flake-based Rust development environment";
  nixConfig = {
    extra-substituters = [
      "https://nixcache.vlt81.de"
      "https://cuda-maintainers.cachix.org"
    ];
    extra-trusted-public-keys = [
      "nixcache.vlt81.de:nw0FfUpePtL6P3IMNT9X6oln0Wg9REZINtkkI9SisqQ="
    ];
  };
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable-small";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    flake-parts.url = "github:hercules-ci/flake-parts";
    devshell.url = "github:numtide/devshell";
  };

  outputs =
    { self
    , nixpkgs
    , rust-overlay
    , flake-utils
    , devshell
    , ...
    }:
    flake-utils.lib.eachDefaultSystem
      (system:
      let
        overlays = [
          rust-overlay.overlays.default
          devshell.overlays.default
          (final: prev: {
            customRustToolchain = prev.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          })
          (final: prev: {
            prev.rocmPackages.clr = prev.rocmPackages.clr.overrideDerivation (oldAttrs: {
              passthru = {
                gpuTargets = rocmTargets;

                updateScript = oldAttrs.passthru.updateScript;

                impureTests = oldAttrs.passthru.impureTests;
              };
            });
          })
        ];
        pkgs = import nixpkgs {
          inherit system overlays;
          config = {
            allowUnfree = true;
            rocmSupport = true;
          };
        };
        buildInputs = with pkgs; [
          harfbuzz
          openssl
          pango
          sqlite
          mariadb
          zlib
          clang
          libclang
          gzip
          coreutils
          gdb
          glib
          glibc
          wayland-utils
          waylandpp
          kdePackages.wayland
          libxkbcommon
          webkitgtk_4_1
          libsoup_3
          gtk3
          libGL
          wayland
        ];
        rocmTargets = [
          "gfx1100"
          "gfx1102"
          "gfx1103"
        ];
      in
      {
        apps.devshell = self.outputs.devShells.${system}.default.flakeApp;
        devShells.default = pkgs.mkShell {
          packages = with pkgs;
            [
              customRustToolchain
              bacon
              binaryen
              cacert
              trunk
              cargo-bloat
              cargo-docset
              cargo-machete
              cargo-limit
              cargo-deny
              cargo-edit
              cargo-watch
              cargo-make
              cargo-generate
              cargo-udeps
              wasm-bindgen-cli_0_2_100
              cargo-outdated
              cargo-release
              calc
              # jre8 # needed for xmlls
              dart-sass
              # trunk
              fish
              inotify-tools
              leptosfmt
              mold
              pkg-config
              rustywind
              sccache
              sqlx-cli
              unzip
              rocmPackages.rocminfo
            ]
            ++ buildInputs;

          buildInputs = buildInputs;
          shellHook = ''
            # export NIX_LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath buildInputs}:$NIX_LD_LIBRARY_PATH
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath buildInputs}"
            export MALLOC_CONF=thp:always,metadata_thp:always
          '';
        };
      });
}
