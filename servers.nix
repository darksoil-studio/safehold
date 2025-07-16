{ inputs, ... }:
let
  sshPubKeys = {
    guillem =
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIDTE+RwRfcG3UNTOZwGmQOKd5R+9jN0adH4BIaZvmWjO guillem.cordoba@gmail.com";
  };
  sshModule = {
    users.users.root.openssh.authorizedKeys.keys =
      builtins.attrValues sshPubKeys;
    services.openssh.settings.PermitRootLogin = "without-password";
  };

  safehold_service_provider =
    inputs.self.outputs.packages."x86_64-linux".safehold-service-provider;

  safehold_service_provider_module = {
    systemd.services.safehold_service_provider1 = {
      enable = true;
      path = [ safehold_service_provider ];
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        ExecStart =
          "${safehold_service_provider}/bin/safehold-service-provider --data-dir /root/safehold-service-provider1";
        RuntimeMaxSec = "3600"; # Restart every hour

        Restart = "always";
        RestartSec = 1;
      };
    };
    systemd.services.safehold_service_provider2 = {
      enable = true;
      path = [ safehold_service_provider ];
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        ExecStart =
          "${safehold_service_provider}/bin/safehold-service-provider --data-dir /root/safehold-service-provider2";
        RuntimeMaxSec = "3600"; # Restart every hour

        Restart = "always";
        RestartSec = 1;
      };
    };
    system.stateVersion = "25.05";
    garnix.server.enable = true;
    garnix.server.persistence.enable = true;
  };

in {
  flake = {

    nixosConfigurations = {
      safehold-service-provider1 = inputs.nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        modules = [
          inputs.garnix-lib.nixosModules.garnix
          sshModule
          { garnix.server.persistence.name = "safehold-service-provider1"; }
          safehold_service_provider_module
        ];
      };
      safehold-service-provider2 = inputs.nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        modules = [
          inputs.garnix-lib.nixosModules.garnix
          sshModule
          { garnix.server.persistence.name = "safehold-service-provider2"; }
          safehold_service_provider_module
        ];
      };
    };
  };
}

