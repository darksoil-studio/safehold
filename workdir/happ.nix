{ inputs, ... }:

{
  # Import all `../dnas/*/dna.nix` files
  imports = (map (m: "${./..}/dnas/${m}/dna.nix") (builtins.attrNames
    (if builtins.pathExists ../dnas then builtins.readDir ../dnas else { })));

  perSystem = { inputs', lib, self', system, ... }: {
    packages.locker_service_provider_happ =
      inputs.holochain-nix-builders.outputs.builders.${system}.happ {
        happManifest = ./happ.yaml;

        dnas = {
          locker = self'.packages.locker_dna;
          manager = self'.packages.manager_dna;
          service_providers = self'.packages.service_providers_dna;
        };
      };

    packages.locker_service_client_happ =
      inputs.holochain-nix-builders.outputs.builders.${system}.happ {
        happManifest = ./happ.yaml;

        dnas = {
          manager = self'.packages.manager_client_dna;
          locker = self'.packages.locker_dna;
          service_providers = self'.packages.service_providers_dna;
        };
      };

  };
}
