use anyhow::{anyhow, Result};
use clone_manager_types::CloneRequest;
use colored::Colorize;
use holochain_client::ZomeCallTarget;
use holochain_runtime::*;
use holochain_types::prelude::*;
use roles_types::Properties;
use setup::setup;
use std::{fs, path::PathBuf};
use utils::with_retries;

mod setup;
mod utils;

pub const SERVICES_ROLE_NAME: &'static str = "services";

pub struct SafeholdServiceClient {
    pub runtime: HolochainRuntime,
    app_id: String,
    progenitors: Vec<AgentPubKey>,
}

impl SafeholdServiceClient {
    pub async fn create(
        data_dir: PathBuf,
        mut network_config: NetworkConfig,
        app_id: String,
        safehold_service_provider_happ_path: PathBuf,
        progenitors: Vec<AgentPubKey>,
        mdns_discovery: bool,
    ) -> Result<Self> {
        network_config.target_arc_factor = 0;
        let mut config = HolochainRuntimeConfig::new(data_dir.clone(), network_config);
        config.mdns_discovery = mdns_discovery;

        let runtime = HolochainRuntime::launch(vec_to_locked(vec![]), config).await?;
        setup(
            &runtime,
            &app_id,
            &safehold_service_provider_happ_path,
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
        with_retries(
            async || {
                let clone_providers: Vec<AgentPubKey> = app_ws
                    .call_zome(
                        ZomeCallTarget::RoleName("manager".into()),
                        ZomeName::from("clone_manager"),
                        "get_clone_providers".into(),
                        ExternIO::encode(())?,
                    )
                    .await?
                    .decode()?;

                if clone_providers.is_empty() {
                    return Err(anyhow!("No clone providers found."));
                }
                Ok(())
            },
            30,
        )
        .await
    }

    pub async fn create_clone_request(&self, network_seed: String) -> anyhow::Result<()> {
        self.wait_for_clone_providers().await?;

        log::info!("Successfully joined peers: executing request...");

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

        let clone_request_hash: EntryHash = app_ws
            .call_zome(
                ZomeCallTarget::RoleName("manager".into()),
                ZomeName::from("clone_manager"),
                "create_clone_request".into(),
                ExternIO::encode(clone_request.clone())?,
            )
            .await?
            .decode()?;

        with_retries(
            async || {
                let providers: Vec<AgentPubKey> = app_ws
                    .call_zome(
                        ZomeCallTarget::RoleName("manager".into()),
                        ZomeName::from("clone_manager"),
                        "get_clone_providers_for_request".into(),
                        ExternIO::encode(clone_request_hash.clone())?,
                    )
                    .await?
                    .decode()?;

                if providers.is_empty() {
                    return Err(anyhow!("No clone providers for the request."));
                }

                Ok(())
            },
            60,
        )
        .await?;

        println!("");

        println!("{}", "Successfully created clone request.".bold().green());

        println!("");

        Ok(())
    }
}

pub async fn read_from_file(happ_bundle_path: &PathBuf) -> Result<AppBundle> {
    let bytes = fs::read(happ_bundle_path)?;
    Ok(AppBundle::unpack(bytes.as_slice())?)
}
