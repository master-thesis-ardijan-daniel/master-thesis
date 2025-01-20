{ config
, pkgs
, lib
, crane
, ...
}:

let
  craneLib = crane.mkLib pkgs;

  fileSetForCrate =
    crate:
    lib.fileset.toSource {
      root = ./.;
      fileset = lib.fileset.unions [
        ./Cargo.toml
        ./Cargo.lock
        (craneLib.fileset.commonCargoSources crate)
      ];
    };

  src = craneLib.cleanCargoSource ./.;

  cargoArtifacts = craneLib.buildDepsOnly {
    inherit src;
  };

  commonArgs = {
    inherit src;
    strictDeps = true;
  };

  crateArgs =
    let
      toplevel = src;
    in
    src: {
      inherit cargoArtifacts;
      inherit (craneLib.crateNameFromCargoToml { src = toplevel; }) version;
      inherit (craneLib.crateNameFromCargoToml { inherit src; }) pname;

      src = fileSetForCrate src;
    };
in
{
  config = {
    checks = {
      inherit (config.packages) backend frontend;

      workspace-fmt = craneLib.cargoFmt { inherit src; };

      workspace-clippy = craneLib.cargoClippy (
        commonArgs
        // {
          inherit cargoArtifacts;
          cargoClippyExtraArgs = "--all-targets -- --deny warnings";
        }
      );
    };

    packages = {
      backend = craneLib.buildPackage (crateArgs ./src/backend);
      frontend = craneLib.buildPackage (crateArgs ./src/frontend);
    };

    devShells.default = craneLib.devShell {
      inherit (config) checks;
      packages = with pkgs; [
        rust-analyzer
      ];
    };
  };
}
