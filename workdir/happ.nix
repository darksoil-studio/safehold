{ inputs, ... }:

{
  # Import all `../dnas/*/dna.nix` files
  imports = (map (m: "${./..}/dnas/${m}/dna.nix") (builtins.attrNames
    (if builtins.pathExists ../dnas then builtins.readDir ../dnas else { })));

  perSystem = { inputs', lib, self', system, ... }: {
    packages.safehold_service_provider_happ =
      inputs.holochain-utils.outputs.builders.${system}.happ {
        happManifest = ./happ.yaml;

        dnas = {
          manager = self'.packages.manager_dna;
          safehold = self'.packages.safehold_dna;
          proxy = self'.packages.proxy_dna;
          services = self'.packages.services_dna;
        };
      };

    packages.safehold_service_client_happ =
      inputs.holochain-utils.outputs.builders.${system}.happ {
        happManifest = builtins.toFile "happ.yaml" ''
          ---
          manifest_version: "1"
          name: safehold-service-client
          description: ~
          roles:   
            - name: manager
              provisioning:
                strategy: create
                deferred: false
              dna:
                bundled: ""
                modifiers:
                  network_seed: ~
                  properties: ~
                version: ~
                clone_limit: 0
        '';

        dnas = { manager = self'.packages.manager_client_dna; };
      };

  };
}
