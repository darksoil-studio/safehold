{
  description = "Template for Holochain app development";

  inputs = {
    holochain-utils.url = "github:darksoil-studio/holochain-utils/main-0.6";
    nixpkgs.follows = "holochain-utils/nixpkgs";
    crane.follows = "holochain-utils/crane";

    service-providers.url = "github:darksoil-studio/service-providers/main-0.6";
    service-providers.inputs.holochain-utils.follows = "holochain-utils";

    clone-manager.url = "github:darksoil-studio/clone-manager-zome/main-0.6";
    clone-manager.inputs.holochain-utils.follows = "holochain-utils";

    garnix-lib = {
      url = "github:garnix-io/garnix-lib";
      inputs.nixpkgs.follows = "nixpkgs";
    };

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
    inputs.holochain-utils.inputs.holonix.inputs.flake-parts.lib.mkFlake {
      inherit inputs;
    } {
      imports = [
        ./servers.nix
        ./workdir/happ.nix
        ./crates/safehold_service_provider/default.nix
        ./crates/safehold_service_client/default.nix
        inputs.holochain-utils.outputs.flakeModules.builders
      ];

      systems =
        builtins.attrNames inputs.holochain-utils.inputs.holonix.devShells;
      perSystem = { inputs', config, pkgs, system, self', ... }: {
        devShells.default = pkgs.mkShell {
          inputsFrom = [
            inputs'.holochain-utils.devShells.holochainDev
            inputs'.holochain-utils.devShells.default
          ];

          packages = [ inputs'.holochain-utils.packages.hc-scaffold-zome ];
        };

        packages.test-safehold-service = pkgs.writeShellApplication {
          name = "test-safehold-service";
          runtimeInputs = [
            self'.packages.safehold-service-provider.meta.debug
            self'.packages.safehold-service-client.meta.debug
          ];
          text = ''
            export RUST_LOG=''${RUST_LOG:=error}
            export BOOTSTRAP_URL=''${BOOTSTRAP_URL:=http://bad.url.only.mdns.discovery}

            DIR1="$(mktemp -d)"
            DIR2="$(mktemp -d)"
            safehold-service-provider --bootstrap-url "$BOOTSTRAP_URL" --mdns-discovery --data-dir "$DIR1" &
            safehold-service-provider --bootstrap-url "$BOOTSTRAP_URL" --mdns-discovery --data-dir "$DIR2" &
            safehold-service-client --bootstrap-url "$BOOTSTRAP_URL" --mdns-discovery create-clone-request --network-seed "$1"

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
