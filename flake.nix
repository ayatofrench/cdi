{
  description = "";

  nixConfig = {
    builders-use-substitutes = true;
    flake-registry = "";
    show-trace = true;

    experimental-features = [
      "flakes"
      "nix-command"
      "pipe-operators"
    ];

    extra-substituters = [
      # "https://cache.garnix.io/"
      "https://nix-community.cachix.org/"
    ];

    extra-trusted-public-keys = [
      # "cache.garnix.io:CTFPyKSLcx5RMJKfLo5EEPUObbA78b0YQ2DTCJXqr9g="
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
    ];
  };

  inputs = {
    systems.url = "github:nix-systems/default";
    # flake-parts.url = "github:hercules-ci/flake-parts";

    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    # RUST
    crane.url = "github:ipetkov/crane";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ {
    systems,
    nixpkgs,
    fenix,
    ...
  }: let
    inherit (nixpkgs) lib;
    forAllSystems = lib.genAttrs (import systems);
    nixpkgsFor = forAllSystems (system: nixpkgs.legacyPackages.${system});
  in {
    devShells = forAllSystems (
      system: let
        pkgs = nixpkgsFor.${system};
        f = fenix.packages.${system};
      in {
        default = pkgs.mkShell {
          packages = [
            f.complete.toolchain
          ];
        };
      }
    );
  };
}
