use std::{path::PathBuf, time::Duration};

use anyhow::anyhow;
use holochain::prelude::{DnaModifiersOpt, RoleSettings, RoleSettingsMap, YamlProperties};
use holochain_client::{AgentPubKey, AppWebsocket};
use holochain_runtime::HolochainRuntime;
use roles_types::Properties;

use crate::read_from_file;

pub async fn setup(
    runtime: &HolochainRuntime,
    app_id: &String,
    locker_service_provider_happ_path: &PathBuf,
    progenitors: Vec<AgentPubKey>,
) -> anyhow::Result<()> {
    let admin_ws = runtime.admin_websocket().await?;
    let installed_apps = admin_ws.list_apps(None).await?;
    let happ_bundle = read_from_file(locker_service_provider_happ_path).await?;
    let roles_properties = Properties {
        progenitors: progenitors.into_iter().map(|p| p.into()).collect(),
    };
    let value = serde_yaml::to_value(roles_properties).unwrap();
    let properties_bytes = YamlProperties::new(value);

    if installed_apps
        .iter()
        .find(|app| app.installed_app_id.eq(app_id))
        .is_none()
    {
        let mut roles_settings = RoleSettingsMap::new();
        roles_settings.insert(
            String::from("manager"),
            RoleSettings::Provisioned {
                membrane_proof: None,
                modifiers: Some(DnaModifiersOpt {
                    properties: Some(properties_bytes.clone()),
                    ..Default::default()
                }),
            },
        );

        let app_info = runtime
            .install_app(
                app_id.clone(),
                happ_bundle,
                Some(roles_settings),
                None,
                None,
            )
            .await?;

        log::info!("Installed app {app_info:?}");
    }
    let app_ws = runtime
        .app_websocket(app_id.clone(), holochain_client::AllowedOrigins::Any)
        .await?;

    // Wait for network to be ready
    wait_until_connected_to_peers(app_ws).await?;

    Ok(())
}

async fn wait_until_connected_to_peers(app_ws: AppWebsocket) -> crate::Result<()> {
    let mut retry_count = 0;
    loop {
        let network_stats = app_ws.dump_network_stats().await?;
        if network_stats.connections.len() > 0 {
            log::info!("Found connected peers: {:?}.", network_stats.connections);
            return Ok(());
        }
        log::warn!("Not connected to peers yet: retrying in 1s.");
        std::thread::sleep(Duration::from_secs(1));

        retry_count += 1;
        if retry_count == 200 {
            return Err(anyhow!("Can't connect to any peers.".to_string(),));
        }
    }
}
