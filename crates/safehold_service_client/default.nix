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
          inputs.holochain-utils.outputs.dependencies.${system}.holochain.buildInputs;
        LIBCLANG_PATH = "${pkgs.llvmPackages_18.libclang.lib}/lib";
      };
      cargoArtifacts = craneLib.buildDepsOnly commonArgs;
      binary =
        craneLib.buildPackage (commonArgs // { inherit cargoArtifacts; });

      binaryWithDebugHapp = pkgs.runCommandLocal "safehold-service-client" {
        buildInputs = [ pkgs.makeWrapper ];
      } ''
        mkdir $out
        mkdir $out/bin
        makeWrapper ${binary}/bin/safehold-service-client $out/bin/safehold-service-client \
          --add-flags "${self'.packages.safehold_service_client_happ.meta.debug}"
      '';
      binaryWithHapp = pkgs.runCommandLocal "safehold-service-client" {
        buildInputs = [ pkgs.makeWrapper ];
        meta.debug = binaryWithDebugHapp;
      } ''
        mkdir $out
        mkdir $out/bin
        makeWrapper ${binary}/bin/safehold-service-client $out/bin/safehold-service-client \
          --add-flags "${self'.packages.safehold_service_client_happ}"
      '';
    in rec {

      builders.safehold-service-client = { progenitors }:
        let
          progenitorsArg = builtins.toString
            (builtins.map (p: " --progenitors ${p}") progenitors);

          debugBinaryWithProgenitors =
            pkgs.runCommandLocal "safehold-service-client" {
              buildInputs = [ pkgs.makeWrapper ];
            } ''
              mkdir $out
              mkdir $out/bin
              makeWrapper ${binaryWithDebugHapp}/bin/safehold-service-client $out/bin/safehold-service-client \
                --add-flags "${progenitorsArg}"
            '';
          binaryWithProgenitors =
            pkgs.runCommandLocal "safehold-service-client" {
              buildInputs = [ pkgs.makeWrapper ];
              meta.debug = debugBinaryWithProgenitors;
              meta.cargoArtifacts = cargoArtifacts;
            } ''
              mkdir $out
              mkdir $out/bin
              makeWrapper ${binaryWithHapp}/bin/safehold-service-client $out/bin/safehold-service-client \
                --add-flags "${progenitorsArg}"
            '';
        in binaryWithProgenitors;

      packages.safehold-service-client = builders.safehold-service-client {
        progenitors = inputs.service-providers.outputs.progenitors;
      };
    };
}
