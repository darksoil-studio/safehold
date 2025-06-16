use anyhow::Result;
use clone_manager_types::NewCloneRequest;
use clone_manager_utils::{clone_cell, reconcile_cloned_cells};
use holochain_client::{AdminWebsocket, AppWebsocket};
use holochain_runtime::*;
use holochain_types::prelude::*;
use setup::setup;
use std::{fs, path::PathBuf, time::Duration};

mod setup;

pub const SERVICE_PROVIDERS_ROLE_NAME: &'static str = "service_providers";

pub async fn run(
    data_dir: PathBuf,
    network_config: NetworkConfig,
    app_id: String,
    locker_service_provider_happ_path: PathBuf,
    progenitors: Vec<AgentPubKey>,
) -> anyhow::Result<()> {
    let config = HolochainRuntimeConfig::new(data_dir.clone(), network_config);

    let runtime = HolochainRuntime::launch(vec_to_locked(vec![]), config).await?;
    setup(
        &runtime,
        &app_id,
        &locker_service_provider_happ_path,
        progenitors,
    )
    .await?;

    let app_ws = runtime
        .app_websocket(app_id.clone(), holochain_client::AllowedOrigins::Any)
        .await?;
    let app_clone = app_ws.clone();
    let admin_ws = runtime.admin_websocket().await?;

    app_ws
        .on_signal(move |signal| {
            let Signal::App { signal, .. } = signal else {
                return ();
            };

            let app_ws = &app_clone;
            let admin_ws = &admin_ws;

            holochain_util::tokio_helper::run_on(async move {
                if let Err(err) = handle_signal(admin_ws, app_ws, signal).await {
                    log::error!("Failed to handle signal: {err:?}");
                }
            });
        })
        .await;

    log::info!("Starting push notifications service provider.");

    loop {
        let app_ws = runtime
            .app_websocket(app_id.clone(), holochain_client::AllowedOrigins::Any)
            .await?;
        let admin_ws = runtime.admin_websocket().await?;
        if let Err(err) = reconcile_cloned_cells(
            &admin_ws,
            &app_ws,
            "manager".into(),
            SERVICE_PROVIDERS_ROLE_NAME.into(),
        )
        .await
        {
            log::error!("Failed to reconcile cloned services: {err}");
        }

        std::thread::sleep(Duration::from_secs(30));
    }
}

pub async fn handle_signal(
    admin_ws: &AdminWebsocket,
    app_ws: &AppWebsocket,
    signal: AppSignal,
) -> anyhow::Result<()> {
    if let Ok(new_clone_request) = signal.into_inner().decode::<NewCloneRequest>() {
        clone_cell(
            &admin_ws,
            &app_ws,
            SERVICE_PROVIDERS_ROLE_NAME.into(),
            new_clone_request.clone_request,
        )
        .await?;
    }
    Ok(())
}
pub async fn read_from_file(happ_bundle_path: &PathBuf) -> Result<AppBundle> {
    let bytes = fs::read(happ_bundle_path)?;
    Ok(AppBundle::decode(bytes.as_slice())?)
}
