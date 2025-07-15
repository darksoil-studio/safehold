{ inputs, ... }:

{
  perSystem = { inputs', system, self', ... }: {
    packages.proxy_integrity =
      inputs.holochain-utils.outputs.builders.${system}.rustZome {
        workspacePath = inputs.self.outPath;
        crateCargoToml = ./Cargo.toml;
      };
  };
}

