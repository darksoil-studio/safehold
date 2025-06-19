{ inputs, ... }:

{
  imports = (map (m: "${./.}/zomes/coordinator/${m}/zome.nix")
    (builtins.attrNames (builtins.readDir ./zomes/coordinator)))
    ++ (map (m: "${./.}/zomes/integrity/${m}/zome.nix")
      (builtins.attrNames (builtins.readDir ./zomes/integrity)));

  perSystem = { inputs', self', lib, system, ... }: {
    packages.proxy_dna =
      inputs.holochain-nix-builders.outputs.builders.${system}.dna {
        dnaManifest = ./workdir/dna.yaml;
        zomes = {
          # This overrides all the "bundled" properties for the DNA manifest
          proxy_integrity = self'.packages.proxy_integrity;
          proxy = self'.packages.proxy;
        };
      };
  };
}

