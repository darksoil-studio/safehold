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
    # To support tests with access to networking
    sandbox = "relaxed";
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        ./workdir/happ.nix
        ./crates/locker_service_provider/default.nix
        ./crates/locker_service_client/default.nix
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

        packages.test-locker-service = pkgs.writeShellApplication {
          name = "test-locker-service";
          runtimeInputs = [
            self'.packages.locker-service-provider.meta.debug
            self'.packages.locker-service-client.meta.debug
          ];
          text = ''
            trap 'killall locker-service-provider' 2 ERR

            export RUST_LOG=error

            rm -rf /tmp/locker-service
            rm -rf /tmp/locker-service2
            locker-service-provider --data-dir /tmp/locker-service --bootstrap-url http://bad --signal-url ws://bad &
            locker-service-provider --data-dir /tmp/locker-service2 --bootstrap-url http://bad --signal-url ws://bad &
            locker-service-client --bootstrap-url http://bad --signal-url ws://bad create-clone-request --network-seed "$1"

            echo "The test locker service is now ready to be used."

            echo ""

            wait
            killall locker-service-provider
          '';
        };

      };
    };
}
