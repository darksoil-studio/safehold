use std::{collections::BTreeSet, time::Duration};

mod common;
use anyhow::anyhow;
use common::*;
use holo_hash::DnaHash;
use holochain_client::{AdminWebsocket, AgentPubKey, ExternIO, SerializedBytes, ZomeCallTarget};
use locker_service_client::LockerServiceClient;
use locker_types::{Message, MessageWithProvenance};
use service_providers_utils::make_service_request;
use tempdir::TempDir;

#[tokio::test(flavor = "multi_thread")]
async fn store_and_get_messages() {
    let Scenario {
        network_seed,
        progenitor,
        // service_provider,
        happ_developer,
        sender,
        recipient,
    } = setup().await;

    let client = LockerServiceClient::create(
        TempDir::new("locker-service-test").unwrap().into_path(),
        network_config(),
        "client-happ".into(),
        client_happ_path(),
        vec![progenitor.clone()],
    )
    .await
    .unwrap();

    client.create_clone_request(network_seed).await.unwrap();

    std::thread::sleep(Duration::from_secs(25));

    let locker_service_trait_service_id = locker_service_trait::LOCKER_SERVICE_HASH.to_vec();

    let service_providers: Vec<AgentPubKey> = happ_developer
        .0
        .call_zome(
            ZomeCallTarget::RoleName("service_providers".into()),
            "service_providers".into(),
            "get_providers_for_service".into(),
            ExternIO::encode(locker_service_trait_service_id.clone()).unwrap(),
        )
        .await
        .unwrap()
        .decode()
        .unwrap();

    assert_eq!(service_providers.len(), 2);

    let message_content: Vec<u8> = vec![0, 1, 2];

    let message = Message {
        recipients: vec![recipient.0.my_pub_key.clone()],
        content: message_content,
    };
    let signature = sender
        .1
        .conductor_handle
        .keystore()
        .sign(
            sender.0.my_pub_key.clone(),
            SerializedBytes::try_from(message.clone())
                .unwrap()
                .bytes()
                .as_slice()
                .into(),
        )
        .await
        .unwrap();

    let message_with_provenance = MessageWithProvenance {
        provenance: sender.0.my_pub_key.clone(),
        signature,
        message,
    };

    let _response: () = make_service_request(
        &sender.0,
        locker_service_trait_service_id.clone(),
        "store_message".into(),
        message_with_provenance.clone(),
    )
    .await
    .unwrap();

    consistency(vec![
        recipient.1.admin_websocket().await.unwrap(),
        sender.1.admin_websocket().await.unwrap(),
    ])
    .await
    .unwrap();

    let messages: Vec<MessageWithProvenance> = make_service_request(
        &recipient.0,
        locker_service_trait_service_id.clone(),
        "get_messages".into(),
        (),
    )
    .await
    .unwrap();

    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0], message_with_provenance);

    std::thread::sleep(Duration::from_millis(100));

    let messages: Vec<MessageWithProvenance> = make_service_request(
        &recipient.0,
        locker_service_trait_service_id.clone(),
        "get_messages".into(),
        (),
    )
    .await
    .unwrap();

    assert_eq!(messages.len(), 0);
}

async fn consistency(admins_wss: Vec<AdminWebsocket>) -> anyhow::Result<()> {
    let mut retry_count = 0;
    loop {
        let dna_hashes: BTreeSet<DnaHash> =
            futures::future::try_join_all(admins_wss.iter().map(|admin| admin.list_dnas()))
                .await
                .unwrap()
                .into_iter()
                .flatten()
                .collect();

        let consistencied = futures::future::try_join_all(
            dna_hashes
                .into_iter()
                .map(|dna| are_conductors_consistencied(&admins_wss, dna)),
        )
        .await?
        .iter()
        .all(|c| c.clone());

        if consistencied {
            return Ok(());
        }

        retry_count += 1;

        if retry_count > 200 {
            return Err(anyhow!("Timeout"));
        }

        std::thread::sleep(Duration::from_millis(500));
    }
}

async fn are_conductors_consistencied(
    admins_wss: &Vec<AdminWebsocket>,
    dna_hash: DnaHash,
) -> anyhow::Result<bool> {
    let states = futures::future::try_join_all(admins_wss.iter().map(|admin_ws| async {
        let cells = admin_ws.list_cell_ids().await?;
        let Some(cell_id) = cells.into_iter().find(|cell| cell.dna_hash().eq(&dna_hash)) else {
            return Err(anyhow!("Cell not found for dna: {dna_hash}."));
        };
        let dump = admin_ws.dump_full_state(cell_id, None).await?;
        Ok(dump)
    }))
    .await?;

    if states.iter().any(|s| {
        s.integration_dump.validation_limbo.len() > 0
            || s.integration_dump.integration_limbo.len() > 0
    }) {
        return Ok(false);
    }

    if !states
        .windows(2)
        .all(|w| w[0].integration_dump.integrated.len() == w[1].integration_dump.integrated.len())
    {
        return Ok(false);
    }

    Ok(true)
}
