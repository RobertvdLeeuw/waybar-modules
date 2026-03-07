{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        craneLib = crane.mkLib pkgs;

        workspaces = craneLib.buildPackage {
          src = craneLib.cleanCargoSource ./workspaces;
          strictDeps = true;
        };

        resources = craneLib.buildPackage {
          src = craneLib.cleanCargoSource ./resources;
          strictDeps = true;

          nativeBuildInputs = with pkgs; [ makeWrapper ];
          buildInputs = with pkgs; [
            rocmPackages.rocm-smi
            lm_sensors
          ];

          postInstall =
            let
              binPath = pkgs.lib.makeBinPath [
                pkgs.rocmPackages.rocm-smi
                pkgs.lm_sensors
              ];
            in
            ''
              wrapProgram $out/bin/resources \
                --prefix PATH : "${binPath}"
            '';
        };
      in
      {
        packages = {
          inherit workspaces;
          inherit resources;

          default = workspaces; # or combine them somehow
        };

        apps = {
          workspaces = {
            type = "app";
            program = "${workspaces}/bin/workspaces";
          };
          resources = {
            type = "app";
            program = "${resources}/bin/resources";
          };
        };
      }
    );
}
