{ inputs, ... }:

{
  perSystem = { inputs', system, self', ... }: {
    packages.encrypted_messages_integrity =
      inputs.holochain-utils.outputs.builders.${system}.rustZome {
        workspacePath = inputs.self.outPath;
        crateCargoToml = ./Cargo.toml;
      };
  };
}

