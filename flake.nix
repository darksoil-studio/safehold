{
  description = "Template for Holochain app development";

  inputs = {
    holonix.url = "github:holochain/holonix/main-0.5";
    crane.follows = "holonix/crane";
    nixpkgs.follows = "holonix/nixpkgs";
    flake-parts.follows = "holonix/flake-parts";

    scaffolding.url = "github:darksoil-studio/scaffolding/main-0.5";
    holochain-nix-builders.url =
      "github:darksoil-studio/holochain-nix-builders/main-0.5";
    tauri-plugin-holochain.url =
      "github:darksoil-studio/tauri-plugin-holochain/main-0.5";
    playground.url = "github:darksoil-studio/holochain-playground/main-0.5";

    service-providers.url = "github:darksoil-studio/service-providers/main-0.5";
    clone-manager.url = "github:darksoil-studio/clone-manager-zome/main-0.5";
  };

  nixConfig = {
    extra-substituters = [
      "https://holochain-ci.cachix.org"
      "https://darksoil-studio.cachix.org"
    ];
    extra-trusted-public-keys = [
      "holochain-ci.cachix.org-1:5IUSkZc0aoRS53rfkvH9Kid40NpyjwCMCzwRTXy+QN8="
      "darksoil-studio.cachix.org-1:UEi+aujy44s41XL/pscLw37KEVpTEIn8N/kn7jO8rkc="
    ];
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        ./workdir/happ.nix
        ./crates/safehold_service_provider/default.nix
        ./crates/safehold_service_client/default.nix
        inputs.holochain-nix-builders.outputs.flakeModules.builders
      ];

      systems = builtins.attrNames inputs.holonix.devShells;
      perSystem = { inputs', config, pkgs, system, self', ... }: {
        devShells.default = pkgs.mkShell {
          inputsFrom = [
            inputs'.holochain-nix-builders.devShells.holochainDev
            inputs'.holonix.devShells.default
          ];

          packages = [ inputs'.scaffolding.packages.hc-scaffold-zome ];
        };

        packages.test-safehold-service = pkgs.writeShellApplication {
          name = "test-safehold-service";
          runtimeInputs = [
            self'.packages.safehold-service-provider.meta.debug
            self'.packages.safehold-service-client.meta.debug
          ];
          text = ''

            export RUST_LOG=''${RUST_LOG:=error}

            DIR1="$(mktemp -d)"
            DIR2="$(mktemp -d)"
            safehold-service-provider --bootstrap-url https://bad.bad --data-dir "$DIR1" &
            safehold-service-provider --bootstrap-url https://bad.bad --data-dir "$DIR2" &
            safehold-service-client --bootstrap-url https://bad.bad create-clone-request --network-seed "$1"

            echo "The test safehold service is now ready to be used."

            echo ""

            function cleanup() {
              killall safehold-service-provider
              rm -rf "$DIR1"
              rm -rf "$DIR2"
            }

            trap cleanup 2 ERR

            wait
            killall safehold-service-provider
          '';
        };

      };
    };
}
