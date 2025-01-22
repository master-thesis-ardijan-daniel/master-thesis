{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";

    flake-parts.url = "github:hercules-ci/flake-parts";

    crane.url = "github:ipetkov/crane/v0.20.0";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { flake-parts, ... } @ inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" ];

      perSystem = { pkgs, system, ... }: {
        formatter = pkgs.nixpkgs-fmt;

        imports = [
          {
            _module.args = {
              pkgs = import inputs.nixpkgs {
                inherit system;
                overlays = [ (import inputs.rust-overlay) ];
              };

              crane = inputs.crane;
            };
          }

          ./packages.nix
        ];
      };
    };
}
