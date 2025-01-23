{ config
, pkgs
, lib
, crane
, ...
}:

let
  craneLib = (crane.mkLib pkgs).overrideToolchain (pkgs:
    pkgs.rust-bin.stable.latest.default.override {
      targets = [ "wasm32-unknown-unknown" ];
    });

  filesetForCrate = crate: lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      (lib.fileset.fileFilter
        (file: lib.any file.hasExt [ "html" "css" "js" ])
        crate
      )
      (craneLib.fileset.commonCargoSources crate)
    ];
  };

  src = craneLib.cleanCargoSource ./.;

  cargoArtifacts = craneLib.buildDepsOnly {
    inherit src;
  };

  cargoArtifactsWasm = craneLib.buildDepsOnly {
    inherit src;

    CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
    cargoExtraArgs = "--package=frontend";
  };

  commonArgs = {
    inherit src;
    strictDeps = true;
  };

  crateArgs =
    let
      toplevel = src;
    in
    src: commonArgs // {
      inherit cargoArtifacts;
      inherit (craneLib.crateNameFromCargoToml { src = toplevel; }) version;
      inherit (craneLib.crateNameFromCargoToml { inherit src; }) pname;

      src = filesetForCrate src;
    };
in
{
  config = {
    checks = {
      inherit (config.packages) backend frontend;

      workspace-fmt = craneLib.cargoFmt { inherit src; };

      workspace-clippy = craneLib.cargoClippy (
        commonArgs // {
          inherit cargoArtifacts;

          cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          ASSETS_DIR = "";
        }
      );

      unitTests = craneLib.cargoNextest (
        commonArgs // {
          inherit cargoArtifacts;
          cargoNextestExtraArgs = "--no-tests=pass";
          ASSETS_DIR = "";
        }
      );
    };

    packages = {
      default = config.packages.backend;

      backend = craneLib.buildPackage (crateArgs ./src/backend // {
        inherit cargoArtifacts;

        cargoExtraArgs = "--package=backend";

        ASSETS_DIR = config.packages.frontend;

      });


      frontend = craneLib.buildTrunkPackage (
        crateArgs ./src/frontend //
        {
          cargoArtifacts = cargoArtifactsWasm;

          wasm-bindgen-cli = pkgs.wasm-bindgen-cli.override {
            version = "0.2.100";
            hash = "sha256-3RJzK7mkYFrs7C/WkhW9Rr4LdP5ofb2FdYGz1P7Uxog=";
            cargoHash = "sha256-tD0OY2PounRqsRiFh8Js5nyknQ809ZcHMvCOLrvYHRE=";
          };

          preBuild = ''
            cd src/frontend
          '';

          cargoExtraArgs = "--package=frontend";

          CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
        }
      );
    };

    devShells.default = craneLib.devShell {
      inherit (config) checks;

      packages = with pkgs; [
        nixd
        rust-analyzer
      ];

      shellHook = ''
        export ASSETS_DIR=$PWD/src/frontend/dist;
      '';
    };
  };
}
