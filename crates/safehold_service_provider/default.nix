{ inputs, self, ... }:

{
  perSystem = { inputs', pkgs, self', lib, system, ... }:
    let
      SERVICE_PROVIDER_HAPP =
        self'.packages.safehold_service_provider_happ.meta.debug;
      CLIENT_HAPP = self'.packages.safehold_service_client_happ.meta.debug;

      END_USER_HAPP = (inputs.holochain-utils.outputs.builders.${system}.happ {
        happManifest = builtins.toFile "happ.yaml" ''
          ---
          manifest_version: "1"
          name: test_happ
          description: ~
          roles:   
            - name: services
              provisioning:
                strategy: create
                deferred: false
              dna:
                bundled: ""
                modifiers:
                  network_seed: ~
                  properties: ~
                version: ~
                clone_limit: 100000
            - name: example
              provisioning:
                strategy: create
                deferred: false
              dna:
                bundled: ""
                modifiers:
                  network_seed: ~
                  properties: ~
                version: ~
                clone_limit: 100000
        '';

        dnas = {
          services = inputs'.service-providers.packages.services_dna;
          example = self'.packages.example_dna;
        };
      }).meta.debug;

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
      binary = craneLib.buildPackage (commonArgs // {
        inherit cargoArtifacts;
      });

      checkArgs = commonArgs // {
        doCheck = true;
        CARGO_PROFILE = "test";
        __noChroot = true;
        WASM_LOG = "info";
      };
      check = craneLib.cargoTest (checkArgs // {
        cargoArtifacts = craneLib.buildDepsOnly checkArgs;
        # For the integration test
        inherit END_USER_HAPP CLIENT_HAPP SERVICE_PROVIDER_HAPP;
      });

      binaryWithDebugHapp = pkgs.runCommandLocal "safehold-service-provider" {
        buildInputs = [ pkgs.makeWrapper ];
      } ''
        mkdir $out
        mkdir $out/bin
        DNA_HASHES=test
        makeWrapper ${binary}/bin/safehold-service-provider $out/bin/safehold-service-provider \
          --add-flags "${self'.packages.safehold_service_provider_happ.meta.debug} --app-id $DNA_HASHES"
      '';
      binaryWithHapp = pkgs.runCommandLocal "safehold-service-provider" {
        buildInputs = [ pkgs.makeWrapper ];
        meta.debug = binaryWithDebugHapp;
      } ''
        mkdir $out
        mkdir $out/bin
        DNA_HASHES=$(cat ${self'.packages.safehold_service_provider_happ.dna_hashes})
        makeWrapper ${binary}/bin/safehold-service-provider $out/bin/safehold-service-provider \
          --add-flags "${self'.packages.safehold_service_provider_happ} --app-id $DNA_HASHES"
      '';
    in rec {

      builders.safehold-service-provider = { progenitors }:
        let
          progenitorsArg = builtins.toString
            (builtins.map (p: " --progenitors ${p}") progenitors);

          binaryDebugWithProgenitors =
            pkgs.runCommandLocal "safehold-service-provider" {
              buildInputs = [ pkgs.makeWrapper ];
            } ''
              mkdir $out
              mkdir $out/bin
              DNA_HASHES=test
              makeWrapper ${binaryWithDebugHapp}/bin/safehold-service-provider $out/bin/safehold-service-provider \
                --add-flags "${progenitorsArg}"
            '';
          binaryWithProgenitors =
            pkgs.runCommandLocal "safehold-service-provider" {
              buildInputs = [ pkgs.makeWrapper ];
              meta.debug = binaryDebugWithProgenitors;
              meta.cargoArtifacts = cargoArtifacts;
            } ''
              mkdir $out
              mkdir $out/bin
              DNA_HASHES=$(cat ${self'.packages.safehold_service_provider_happ.dna_hashes})
              makeWrapper ${binaryWithHapp}/bin/safehold-service-provider $out/bin/safehold-service-provider \
                --add-flags "${progenitorsArg}"
            '';
        in binaryWithProgenitors;

      packages.safehold-service-provider = builders.safehold-service-provider {
        progenitors = inputs.service-providers.outputs.progenitors;
      };

      checks.store-and-get-messages-test = check;
    };
}
