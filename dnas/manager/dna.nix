{ inputs, ... }:

{
  # imports = (map (m: "${./.}/zomes/coordinator/${m}/zome.nix")
  #   (builtins.attrNames (builtins.readDir ./zomes/coordinator)))
  #   ++ (map (m: "${./.}/zomes/integrity/${m}/zome.nix")
  #     (builtins.attrNames (builtins.readDir ./zomes/integrity)));

  perSystem = { inputs', self', lib, system, ... }: {
    packages.manager_dna =
      inputs.holochain-nix-builders.outputs.builders.${system}.dna {
        dnaManifest = ./workdir/dna.yaml;
        zomes = {
          # This overrides all the "bundled" properties for the DNA manifest
          clone_manager_integrity =
            inputs'.clone-manager.packages.clone_manager_integrity;
          clone_manager = inputs'.clone-manager.packages.clone_manager_provider;
        };
      };
    packages.manager_client_dna =
      inputs.holochain-nix-builders.outputs.builders.${system}.dna {
        dnaManifest = ./workdir/dna.yaml;
        zomes = {
          # This overrides all the "bundled" properties for the DNA manifest
          clone_manager_integrity =
            inputs'.clone-manager.packages.clone_manager_integrity;
          clone_manager = inputs'.clone-manager.packages.clone_manager;
        };
      };
  };
}

