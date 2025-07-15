{ inputs, ... }:

{
  perSystem = { inputs', system, self', ... }: {
    packages.safehold_gateway =
      inputs.holochain-utils.outputs.builders.${system}.rustZome {
        workspacePath = inputs.self.outPath;
        crateCargoToml = ./Cargo.toml;
      };
  };
}

