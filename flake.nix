{
  description = "memrs — an in-memory data structure store";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
  };

  outputs = { self, nixpkgs, flake-utils, naersk }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        naersk' = pkgs.callPackage naersk { };
      in
      {
        packages = {
          memrs = naersk'.buildPackage {
            name = "memrs";
            src = ./memrs-core;
            cargoBuildOptions = x: x ++ [ "--package" "memrs" ];
          };

          memrs-cli = naersk'.buildPackage {
            name = "memrs-cli";
            src = ./memrs-cli;
          };

          default = self.packages.${system}.memrs;
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [ cargo rustc rustfmt clippy ];
        };
      });
}
