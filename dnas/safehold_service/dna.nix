{ inputs, ... }:

{
  imports = (map (m: "${./.}/zomes/coordinator/${m}/zome.nix")
    (builtins.attrNames (builtins.readDir ./zomes/coordinator)))
    ++ (map (m: "${./.}/zomes/integrity/${m}/zome.nix")
      (builtins.attrNames (builtins.readDir ./zomes/integrity)));

  perSystem = { inputs', self', lib, system, ... }: {
    packages.safehold_dna =
      inputs.holochain-utils.outputs.builders.${system}.dna {
        dnaManifest = ./workdir/dna.yaml;
        zomes = {
          # This overrides all the "bundled" properties for the DNA manifest
          safehold_integrity = self'.packages.safehold_integrity;
          safehold = self'.packages.safehold;
        };
      };
  };
}

