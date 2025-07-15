{ inputs, ... }:

{
  perSystem = { inputs', system, self', ... }: {
    packages.safehold =
      inputs.holochain-utils.outputs.builders.${system}.rustZome {
        workspacePath = inputs.self.outPath;
        crateCargoToml = ./Cargo.toml;
      };
  };
}

