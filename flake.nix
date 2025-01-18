{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";

    flake-parts.url = "github:hercules-ci/flake-parts";

    crane.url = "github:ipetkov/crane/v0.20.0";
  };

  outputs = { flake-parts, crane, ... } @ inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" ];

      perSystem = { pkgs, ... }: {
        formatter = pkgs.nixpkgs-fmt;

        imports = [
          ({ _module.args.crane = inputs.crane; })

          ./packages.nix
        ];
      };
    };
}
