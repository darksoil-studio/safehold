use anyhow::{anyhow, Result};
use clone_manager_types::CloneRequest;
use colored::Colorize;
use holochain_client::ZomeCallTarget;
use holochain_runtime::*;
use holochain_types::prelude::*;
use roles_types::Properties;
use setup::setup;
use std::{collections::BTreeMap, fs, path::PathBuf, time::Duration};

mod setup;

pub const SERVICE_PROVIDERS_ROLE_NAME: &'static str = "service_providers";

pub struct LockerServiceClient {
    runtime: HolochainRuntime,
    app_id: String,
    progenitors: Vec<AgentPubKey>,
}

impl LockerServiceClient {
    pub async fn create(
        data_dir: PathBuf,
        mut network_config: NetworkConfig,
        app_id: String,
        locker_service_provider_happ_path: PathBuf,
        progenitors: Vec<AgentPubKey>,
    ) -> Result<Self> {
        network_config.target_arc_factor = 0;
        let config = HolochainRuntimeConfig::new(data_dir.clone(), network_config);

        let runtime = HolochainRuntime::launch(vec_to_locked(vec![]), config).await?;
        setup(
            &runtime,
            &app_id,
            &locker_service_provider_happ_path,
            progenitors.clone(),
        )
        .await?;
        Ok(Self {
            app_id,
            runtime,
            progenitors,
        })
    }

    pub async fn wait_for_clone_providers(&self) -> anyhow::Result<()> {
        let app_ws = self
            .runtime
            .app_websocket(self.app_id.clone(), holochain_client::AllowedOrigins::Any)
            .await?;

        let mut retry_count = 0;
        loop {
            let clone_providers: Vec<AgentPubKey> = app_ws
                .call_zome(
                    ZomeCallTarget::RoleName("manager".into()),
                    ZomeName::from("clone_manager"),
                    "get_clone_providers".into(),
                    ExternIO::encode(())?,
                )
                .await?
                .decode()?;

            if clone_providers.len() > 0 {
                return Ok(());
            }
            log::warn!("No clone providers found yet: retrying in 1s.");
            std::thread::sleep(Duration::from_secs(1));

            retry_count += 1;
            if retry_count == 60 {
                return Err(anyhow!("No clone providers found.".to_string(),));
            }
        }
    }

    pub async fn create_clone_request(&self, network_seed: String) -> anyhow::Result<()> {
        let app_ws = self
            .runtime
            .app_websocket(self.app_id.clone(), holochain_client::AllowedOrigins::Any)
            .await?;

        let roles_properties = Properties {
            progenitors: self
                .progenitors
                .clone()
                .into_iter()
                .map(|p| p.into())
                .collect(),
        };
        let properties = SerializedBytes::try_from(roles_properties)?;

        let clone_request = CloneRequest {
            dna_modifiers: DnaModifiers {
                network_seed,
                properties,
            },
        };

        log::info!("Creating clone request...");

        app_ws
            .call_zome(
                ZomeCallTarget::RoleName("manager".into()),
                ZomeName::from("clone_manager"),
                "create_clone_request".into(),
                ExternIO::encode(clone_request.clone())?,
            )
            .await?;

        std::thread::sleep(Duration::from_secs(4));

        let result = app_ws
            .call_zome(
                ZomeCallTarget::RoleName("manager".into()),
                ZomeName::from("clone_manager"),
                "get_all_clone_requests".into(),
                ExternIO::encode(())?,
            )
            .await?;

        let all_clone_requests: BTreeMap<EntryHashB64, CloneRequest> = result.decode()?;

        if !all_clone_requests
            .into_values()
            .any(|created_clone_request| created_clone_request.eq(&clone_request))
        {
            return Err(anyhow!("Failed to create clone request."));
        }

        println!("");

        println!("{}", "Successfully created clone request.".bold().green());

        println!("");

        Ok(())
    }
}

pub async fn read_from_file(happ_bundle_path: &PathBuf) -> Result<AppBundle> {
    let bytes = fs::read(happ_bundle_path)?;
    Ok(AppBundle::decode(bytes.as_slice())?)
}
