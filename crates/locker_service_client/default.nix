{ inputs, self, ... }:

{
  perSystem = { inputs', pkgs, self', lib, system, ... }:
    let

      craneLib = inputs.crane.mkLib pkgs;
      src = craneLib.cleanCargoSource (craneLib.path self.outPath);

      cratePath = ./.;

      cargoToml =
        builtins.fromTOML (builtins.readFile "${cratePath}/Cargo.toml");
      crate = cargoToml.package.name;
      pname = crate;
      version = cargoToml.package.version;

      commonArgs = {
        inherit src version pname;
        doCheck = false;
        buildInputs =
          inputs.holochain-nix-builders.outputs.dependencies.${system}.holochain.buildInputs;
        LIBCLANG_PATH = "${pkgs.llvmPackages_18.libclang.lib}/lib";
      };
      cargoArtifacts = craneLib.buildDepsOnly commonArgs;
      binary =
        craneLib.buildPackage (commonArgs // { inherit cargoArtifacts; });

      binaryWithDebugHapp =
        pkgs.runCommandLocal "locker-service-client" {
          buildInputs = [ pkgs.makeWrapper ];
        } ''
          mkdir $out
          mkdir $out/bin
          makeWrapper ${binary}/bin/locker-service-client $out/bin/locker-service-client \
            --add-flags "${self'.packages.locker_service_client_happ.meta.debug}"
        '';
      binaryWithHapp =
        pkgs.runCommandLocal "locker-service-client" {
          buildInputs = [ pkgs.makeWrapper ];
          meta.debug = binaryWithDebugHapp;
        } ''
          mkdir $out
          mkdir $out/bin
          makeWrapper ${binary}/bin/locker-service-client $out/bin/locker-service-client \
            --add-flags "${self'.packages.locker_service_client_happ}"
        '';
    in rec {

      builders.locker-service-client = { progenitors }:
        let
          progenitorsArg = builtins.toString
            (builtins.map (p: " --progenitors ${p}") progenitors);

          debugBinaryWithProgenitors =
            pkgs.runCommandLocal "locker-service-client" {
              buildInputs = [ pkgs.makeWrapper ];
            } ''
              mkdir $out
              mkdir $out/bin
              makeWrapper ${binaryWithDebugHapp}/bin/locker-service-client $out/bin/locker-service-client \
                --add-flags "${progenitorsArg}"
            '';
          binaryWithProgenitors =
            pkgs.runCommandLocal "locker-service-client" {
              buildInputs = [ pkgs.makeWrapper ];
              meta.debug = debugBinaryWithProgenitors;
            } ''
              mkdir $out
              mkdir $out/bin
              makeWrapper ${binaryWithHapp}/bin/locker-service-client $out/bin/locker-service-client \
                --add-flags "${progenitorsArg}"
            '';
        in binaryWithProgenitors;

      packages.locker-service-client =
        builders.locker-service-client {
          progenitors = inputs.service-providers.outputs.progenitors;
        };
    };
}
