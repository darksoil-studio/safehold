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
            inputs'.scaffolding.devShells.synchronized-pnpm
            inputs'.holochain-nix-builders.devShells.holochainDev
            inputs'.holonix.devShells.default
          ];

          packages = [
            inputs'.holochain-nix-builders.packages.holochain
            inputs'.scaffolding.packages.hc-scaffold-zome
            inputs'.tauri-plugin-holochain.packages.hc-pilot
            inputs'.playground.packages.hc-playground
          ];
        };
        devShells.npm-ci = inputs'.scaffolding.devShells.synchronized-pnpm;

        packages.test-locker-service = pkgs.writeShellApplication {
          name = "test-locker-service";
          runtimeInputs = [
            self'.packages.locker-service-provider.meta.debug
            self'.packages.locker-service-client.meta.debug
          ];
          text = ''
            trap 'killall locker-service-provider' 2 ERR

            export RUST_LOG=error

            rm -rf /tmp/pnsp
            rm -rf /tmp/pnsp2
            locker-service-provider --data-dir /tmp/pnsp --bootstrap-url http://bad --signal-url ws://bad &
            locker-service-provider --data-dir /tmp/pnsp2 --bootstrap-url http://bad --signal-url ws://bad &
            locker-service-client --bootstrap-url http://bad --signal-url ws://bad create-clone-request --network-seed "$2"

            echo "The test locker service is now ready to be used."

            echo ""

            wait
            killall locker-service-provider
          '';
        };

        packages.scaffold = pkgs.symlinkJoin {
          name = "scaffold-remote-zome";
          paths = [ inputs'.scaffolding.packages.scaffold-remote-zome ];
          buildInputs = [ pkgs.makeWrapper ];
          postBuild = ''
            wrapProgram $out/bin/scaffold-remote-zome \
              --add-flags "locker-service-provider-zome \
                --integrity-zome-name locker_service_provider_integrity \
                --coordinator-zome-name locker_service_provider \
                --remote-zome-git-url github:darksoil-studio/locker-service-provider-zome \
                --remote-npm-package-name @darksoil-studio/locker-service-provider-zome \
                --remote-zome-git-branch main-0.5 \
                --context-element locker-service-provider-context \
                --context-element-import @darksoil-studio/locker-service-provider-zome/dist/elements/locker-service-provider-context.js" 
          '';
        };
      };
    };
}
