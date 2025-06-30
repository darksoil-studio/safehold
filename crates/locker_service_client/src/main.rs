use anyhow::Result;
use clap::{Parser, Subcommand};
use env_logger::Builder;
use holochain::core::AgentPubKeyB64;
use holochain::prelude::NetworkSeed;
use holochain_runtime::NetworkConfig;
use locker_service_client::LockerServiceClient;
use log::Level;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use tempdir::TempDir;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    locker_service_provider_happ: PathBuf,

    #[arg(long, required = true, num_args = 1)]
    progenitors: Vec<AgentPubKeyB64>,

    #[arg(long)]
    bootstrap_url: String,

    #[arg(long)]
    signal_url: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create a clone request for the service providers DNA
    CreateCloneRequest {
        #[arg(long)]
        network_seed: NetworkSeed,
    },
}

fn network_config(bootstrap_url: String, signal_url: String) -> NetworkConfig {
    let mut network_config = NetworkConfig::default();

    network_config.bootstrap_url = url2::Url2::parse(bootstrap_url);
    network_config.signal_url = url2::Url2::parse(signal_url);
    network_config.webrtc_config = Some(serde_json::json!({
        "ice_servers": {
            "urls": ["stun://stun.l.google.com:19302"]
        },
    }));
    network_config.target_arc_factor = 0;

    network_config
}

fn log_level() -> Level {
    match std::env::var("RUST_LOG") {
        Ok(s) => Level::from_str(s.as_str()).expect("Invalid RUST_LOG level"),
        _ => Level::Info,
    }
}

fn set_wasm_level() {
    match std::env::var("WASM_LOG") {
        Ok(_s) => {}
        _ => {
            std::env::set_var("WASM_LOG", "info");
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    Builder::new()
        .format(|buf, record| writeln!(buf, "[{}] {}", record.level(), record.args()))
        .target(env_logger::Target::Stdout)
        .filter(None, log_level().to_level_filter())
        .filter_module("holochain_sqlite", log::LevelFilter::Off)
        .filter_module("tracing::span", log::LevelFilter::Off)
        .init();
    set_wasm_level();

    let tempdir = TempDir::new("locker-service-client")?;
    let data_dir = tempdir.path().to_path_buf();

    let client = LockerServiceClient::create(
        data_dir.clone(),
        network_config(args.bootstrap_url, args.signal_url),
        String::from("temporary-client-app"),
        args.locker_service_provider_happ,
        args.progenitors.into_iter().map(|p| p.into()).collect(),
    )
    .await?;

    client.wait_for_clone_providers().await?;

    log::info!("Successfully joined peers: executing request...");

    match args.command {
        Commands::CreateCloneRequest { network_seed } => {
            client.create_clone_request(network_seed).await?;
        }
    }
    std::thread::sleep(Duration::from_secs(4));

    Ok(())
}
