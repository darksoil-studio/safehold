{ inputs, ... }:

{
  perSystem = { inputs', system, self', ... }: {
    packages.encrypted_messages_integrity =
      inputs.holochain-nix-builders.outputs.builders.${system}.rustZome {
        workspacePath = inputs.self.outPath;
        crateCargoToml = ./Cargo.toml;
      };
  };
}

