{ inputs, ... }:
let
  sshPubKeys = {
    guillem =
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIDTE+RwRfcG3UNTOZwGmQOKd5R+9jN0adH4BIaZvmWjO guillem.cordoba@gmail.com";
    guillemslaptop = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIO8DVpvRgQ90MyMyiuNdvyMNAio9n2o/+57MyhZS2A5A guillem.cordoba@gmail.com";
  };
  sshModule = {
    users.users.root.openssh.authorizedKeys.keys =
      builtins.attrValues sshPubKeys;
    services.openssh.settings.PermitRootLogin = "without-password";
  };
  bootstrapServerUrl = "http://157.180.93.55:8888";

  safehold-service-provider =
    inputs.self.outputs.packages."x86_64-linux".safehold-service-provider;

  safehold-service-provider-module = {
    systemd.services.safehold-service-provider = {
      enable = true;
      path = [ safehold-service-provider ];
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      serviceConfig = {
        ExecStart =
          "${safehold-service-provider}/bin/safehold-service-provider --data-dir /root/safehold-service-provider  --bootstrap-url ${bootstrapServerUrl}";
        RuntimeMaxSec = "3600"; # Restart every hour

        Restart = "always";
      };
    };
  };

in {

  flake = {
    nixosConfigurations = {
      safehold-service-provider1 = inputs.nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        modules = [
          inputs.garnix-lib.nixosModules.garnix
          sshModule
          safehold-service-provider-module
          {
            garnix.server.persistence.name =
              "safehold-service-provider-v0-5-x-3";
            system.stateVersion = "25.05";
            garnix.server.enable = true;
            garnix.server.persistence.enable = true;
          }
        ];
      };
      safehold-service-provider2 = inputs.nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        modules = [
          inputs.garnix-lib.nixosModules.garnix
          sshModule
          safehold-service-provider-module
          {
            garnix.server.persistence.name =
              "safehold-service-provider-v0-5-x-4";
            system.stateVersion = "25.05";
            garnix.server.enable = true;
            garnix.server.persistence.enable = true;
          }
        ];
      };
    };
  };
}

