use std::{collections::BTreeSet, time::Duration};

mod common;
use anyhow::anyhow;
use common::*;
use holochain::core::DnaHash;
use holochain_client::{AdminWebsocket, AgentPubKey, ExternIO, ZomeCallTarget};
use locker_service_client::LockerServiceClient;
use locker_service_trait::MessageOutput;
use locker_types::{DecryptedMessageOutput, EncryptMessageInput, MessageWithProvenance};
use service_providers_utils::make_service_request;
use tempdir::TempDir;

#[tokio::test(flavor = "multi_thread")]
async fn store_and_get_messages() {
    let Scenario {
        network_seed,
        progenitor,
        // service_provider,
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

    std::thread::sleep(Duration::from_secs(5));

    consistency(vec![
        recipient.1.admin_websocket().await.unwrap(),
        sender.1.admin_websocket().await.unwrap(),
    ])
    .await
    .unwrap();

    let locker_service_trait_service_id = locker_service_trait::LOCKER_SERVICE_HASH.to_vec();

    let service_providers: Vec<AgentPubKey> = sender
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

    let message = EncryptMessageInput {
        message: message_content.clone(),
        recipients: vec![recipient.0.my_pub_key.clone()],
    };

    let messages: Vec<MessageWithProvenance> = sender
        .0
        .call_zome(
            ZomeCallTarget::RoleName("example".into()),
            "encrypted_messages".into(),
            "encrypt_message".into(),
            ExternIO::encode(message).unwrap(),
        )
        .await
        .unwrap()
        .decode()
        .unwrap();

    let _response: () = make_service_request(
        &sender.0,
        locker_service_trait_service_id.clone(),
        "store_messages".into(),
        messages.clone(),
    )
    .await
    .unwrap();

    consistency(vec![
        recipient.1.admin_websocket().await.unwrap(),
        sender.1.admin_websocket().await.unwrap(),
    ])
    .await
    .unwrap();

    let messages_outputs: Vec<MessageOutput> = make_service_request(
        &recipient.0,
        locker_service_trait_service_id.clone(),
        "get_messages".into(),
        (),
    )
    .await
    .unwrap();

    assert_eq!(messages_outputs.len(), 1);

    std::thread::sleep(Duration::from_millis(500));

    let messages: Vec<MessageOutput> = make_service_request(
        &recipient.0,
        locker_service_trait_service_id.clone(),
        "get_messages".into(),
        (),
    )
    .await
    .unwrap();

    assert_eq!(messages_outputs.len(), 0);

    let decrypted_messages: Vec<DecryptedMessageOutput> = recipient
        .0
        .call_zome(
            ZomeCallTarget::RoleName("example".into()),
            "encrypted_messages".into(),
            "decrypt_messages".into(),
            ExternIO::encode(messages).unwrap(),
        )
        .await
        .unwrap()
        .decode()
        .unwrap();

    assert_eq!(decrypted_messages.len(), 0);
    assert_eq!(decrypted_messages[0].contents, message_content);
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

    println!("{:?}", states);

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
