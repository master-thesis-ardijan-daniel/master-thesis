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

  src =
    let
      root = ./.;
    in
    lib.fileset.toSource {
      inherit root;
      fileset = lib.fileset.unions [
        (craneLib.fileset.commonCargoSources root)
        (lib.fileset.fileFilter
          (file: lib.any file.hasExt [ "html" "scss" "js" "wgsl" ])
          root
        )
      ];
    };

  commonArgs = {
    inherit src;
    strictDeps = true;
    doCheck = false;
  };

  native = rec {
    args = commonArgs;
    cargoArtifacts = craneLib.buildDepsOnly args;
  };

  wasm = rec {
    args = commonArgs // {
      cargoExtraArgs = "--package=frontend --locked";

      CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
    };
    cargoArtifacts = craneLib.buildDepsOnly args;
  };

  crateArgs =
    let
      toplevel = src;
    in
    src: commonArgs // {
      inherit (craneLib.crateNameFromCargoToml { src = toplevel; }) version;
      inherit (craneLib.crateNameFromCargoToml { inherit src; }) pname;
    };
in
{
  config = {
    checks = {
      inherit (config.packages) backend frontend;

      thesis-fmt = craneLib.cargoFmt { inherit src; };

      thesis-clippy = craneLib.cargoClippy (
        native.args // {
          inherit (native) cargoArtifacts;
          cargoClippyExtraArgs = "--all-targets -- --deny warnings";

          ASSETS_DIR = "";
        }
      );

      thesis-nextest = craneLib.cargoNextest (
        native.args // {
          inherit (native) cargoArtifacts;
          cargoNextestExtraArgs = "--no-tests=pass";
          doCheck = true;

          ASSETS_DIR = "";
        }
      );
    };

    packages = {
      default = config.packages.backend;

      backend = craneLib.buildPackage (
        crateArgs ./src/backend // {
          inherit (native) cargoArtifacts;

          ASSETS_DIR = config.packages.frontend;
        }
      );

      frontend = craneLib.buildTrunkPackage (
        crateArgs ./src/frontend // wasm.args // {
          inherit (wasm) cargoArtifacts;

          wasm-bindgen-cli = pkgs.wasm-bindgen-cli_0_2_100;

          preBuild = ''
            cd src/frontend
          '';
        }
      );
    };

    devShells.default = craneLib.devShell {
      inherit (config) checks;

      packages = with pkgs; [
        nixd
        rust-analyzer
        wgsl-analyzer
      ];

      shellHook = ''
        export ASSETS_DIR=$PWD/src/frontend/dist;
      '';
    };
  };
}
