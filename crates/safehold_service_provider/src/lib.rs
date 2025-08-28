use anyhow::{anyhow, Result};
use clone_manager_types::{CloneRequest, NewCloneRequest};
use clone_manager_utils::reconcile_cloned_cells;
use holochain_client::{AdminWebsocket, AppWebsocket};
use holochain_runtime::*;
use holochain_types::prelude::*;
use safehold_clones::reconcile_safehold_clones;
use setup::setup;
use std::{fs, path::PathBuf, time::Duration};
use utils::with_retries;

mod safehold_clones;
mod setup;
mod utils;

pub const SERVICES_ROLE_NAME: &'static str = "services";

pub async fn run(
    data_dir: PathBuf,
    network_config: NetworkConfig,
    app_id: String,
    safehold_service_provider_happ_path: PathBuf,
    progenitors: Vec<AgentPubKey>,
    mdns_discovery: bool,
) -> anyhow::Result<()> {
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

    log::info!("Starting safehold service provider.");

    let r = runtime.clone();

    let abort_handle = tokio::spawn(async move {
        loop {
            let Ok(app_ws) = runtime
                .app_websocket(app_id.clone(), holochain_client::AllowedOrigins::Any)
                .await
            else {
                log::error!("Failed to connect to the app websocket");
                continue;
            };
            let Ok(admin_ws) = runtime.admin_websocket().await else {
                log::error!("Failed to connect to the admin websocket");
                continue;
            };
            if let Err(err) = reconcile_cloned_cells(
                &admin_ws,
                &app_ws,
                "manager".into(),
                SERVICES_ROLE_NAME.into(),
            )
            .await
            {
                log::error!("Failed to reconcile cloned services: {err}");
            }
            if let Err(err) =
                reconcile_safehold_clones(&admin_ws, &app_ws, progenitors.clone()).await
            {
                log::error!("Failed to reconcile safehold clones: {err}");
            }

            std::thread::sleep(Duration::from_secs(30));
        }
    })
    .abort_handle();

    // wait for a unix signal or ctrl-c instruction to
    // shutdown holochain
    ctrlc::set_handler(move || {
        abort_handle.abort();
        let r = r.clone();
        holochain_util::tokio_helper::block_on(
            async move {
                log::info!("Gracefully shutting down conductor...");
                if let Err(err) = r.shutdown().await {
                    log::error!("Failed to shutdown conductor: {err:?}.");
                }
            },
            Duration::from_secs(10),
        )
        .expect("Timed out shutting down holochain.");
        std::process::exit(0);
    })?;

    // wait for a unix signal or ctrl-c instruction to
    tokio::signal::ctrl_c()
        .await
        .unwrap_or_else(|e| log::error!("Could not handle termination signal: {:?}", e));

    Ok(())
}

pub async fn handle_signal(
    admin_ws: &AdminWebsocket,
    app_ws: &AppWebsocket,
    signal: AppSignal,
) -> anyhow::Result<()> {
    if let Ok(new_clone_request) = signal.into_inner().decode::<NewCloneRequest>() {
        let a = app_ws.clone();
        with_retries(
            async move || {
                let clone_request: Option<CloneRequest> = a
                    .call_zome(
                        holochain_client::ZomeCallTarget::RoleName(String::from("manager")),
                        "clone_manager".into(),
                        "get_clone_request".into(),
                        ExternIO::encode(new_clone_request.clone_request_hash.clone())?,
                    )
                    .await?
                    .decode()?;
                let Some(_) = clone_request else {
                    return Err(anyhow!("CloneRequest not found."));
                };

                Ok(())
            },
            10,
        )
        .await?;

        reconcile_cloned_cells(
            &admin_ws,
            &app_ws,
            "manager".into(),
            SERVICES_ROLE_NAME.into(),
        )
        .await?;
    }
    Ok(())
}

pub async fn read_from_file(happ_bundle_path: &PathBuf) -> Result<AppBundle> {
    let bytes = fs::read(happ_bundle_path)?;
    Ok(AppBundle::decode(bytes.as_slice())?)
}
