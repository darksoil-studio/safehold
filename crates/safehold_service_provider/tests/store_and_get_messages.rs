use std::{collections::BTreeSet, time::Duration};

mod common;
use anyhow::anyhow;
use common::*;
use holochain::core::DnaHash;
use holochain_client::{AdminWebsocket, AgentPubKey, AppWebsocket, ExternIO, ZomeCallTarget};
use safehold_service_client::SafeholdServiceClient;
use safehold_service_trait::MessageOutput;
use safehold_types::{
    DecryptedMessageOutput, EncryptMessageInput, MessageContents, MessageWithProvenance,
};
use serial_test::serial;
use service_providers_utils::make_service_request;
use tempdir::TempDir;

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn store_and_get_messages() {
    let Scenario {
        network_seed,
        progenitor,
        // service_provider,
        alice,
        bob,
        carol,
    } = setup().await;

    let client = SafeholdServiceClient::create(
        TempDir::new("safehold-service-test").unwrap().into_path(),
        network_config(),
        "client-happ".into(),
        client_happ_path(),
        vec![progenitor.clone()],
    )
    .await
    .unwrap();

    std::thread::sleep(Duration::from_secs(10));

    client.create_clone_request(network_seed).await.unwrap();

    with_retries(
        async || {
            let safehold_service_trait_service_id =
                safehold_service_trait::SAFEHOLD_SERVICE_HASH.to_vec();

            let service_providers: Vec<AgentPubKey> = alice
                .0
                .call_zome(
                    ZomeCallTarget::RoleName("service_providers".into()),
                    "service_providers".into(),
                    "get_providers_for_service".into(),
                    ExternIO::encode(safehold_service_trait_service_id.clone())?,
                )
                .await?
                .decode()?;
            if service_providers.is_empty() {
                return Err(anyhow!("No service providers yet"));
            }
            Ok(())
        },
        120,
    )
    .await
    .unwrap();

    let message_content: Vec<u8> = vec![0; 10];
    let messages: Vec<MessageWithProvenance> = send_message(
        &alice.0,
        vec![bob.0.my_pub_key.clone(), carol.0.my_pub_key.clone()],
        message_content.clone(),
    )
    .await
    .unwrap();

    assert_eq!(messages.len(), 2);

    std::thread::sleep(Duration::from_secs(10));

    let decrypted_messages: Vec<DecryptedMessageOutput> = receive_messages(&bob.0).await.unwrap();

    assert_eq!(decrypted_messages.len(), 1);
    assert_eq!(decrypted_messages[0].contents, message_content);

    std::thread::sleep(Duration::from_secs(2));

    let decrypted_messages: Vec<DecryptedMessageOutput> = receive_messages(&bob.0).await.unwrap();

    assert_eq!(decrypted_messages.len(), 0);

    let messages = send_message(
        &bob.0,
        vec![alice.0.my_pub_key.clone(), carol.0.my_pub_key.clone()],
        vec![0, 0, 0],
    )
    .await
    .unwrap();

    assert_eq!(messages.len(), 2);

    let decrypted_messages = receive_messages(&carol.0).await.unwrap();
    assert_eq!(decrypted_messages.len(), 2);

    let messages: Vec<MessageWithProvenance> = send_message(
        &carol.0,
        vec![alice.0.my_pub_key.clone(), bob.0.my_pub_key.clone()],
        vec![0, 0, 0],
    )
    .await
    .unwrap();

    // Now only one message is necessary because players exchanged X25519 keys
    assert_eq!(messages.len(), 1);

    // std::thread::sleep(Duration::from_secs(120));

    // let decrypted_messages = receive_messages(&alice.0).await.unwrap();
    // assert_eq!(decrypted_messages.len(), 2);

    // let messages: Vec<MessageWithProvenance> = send_message(
    //     &alice.0,
    //     vec![bob.0.my_pub_key.clone(), carol.0.my_pub_key.clone()],
    //     message_content.clone(),
    // )
    // .await
    // .unwrap();

    // assert_eq!(messages.len(), 1);
}

const CHUNK_SIZE: usize = 1000;

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn store_and_get_big_messages_in_chunks() {
    let Scenario {
        network_seed,
        progenitor,
        // service_provider,
        alice,
        bob,
        carol,
    } = setup().await;

    let client = SafeholdServiceClient::create(
        TempDir::new("safehold-service-test").unwrap().into_path(),
        network_config(),
        "client-happ".into(),
        client_happ_path(),
        vec![progenitor.clone()],
    )
    .await
    .unwrap();

    std::thread::sleep(Duration::from_secs(10));

    client.create_clone_request(network_seed).await.unwrap();

    with_retries(
        async || {
            let safehold_service_trait_service_id =
                safehold_service_trait::SAFEHOLD_SERVICE_HASH.to_vec();

            let service_providers: Vec<AgentPubKey> = alice
                .0
                .call_zome(
                    ZomeCallTarget::RoleName("service_providers".into()),
                    "service_providers".into(),
                    "get_providers_for_service".into(),
                    ExternIO::encode(safehold_service_trait_service_id.clone())?,
                )
                .await?
                .decode()?;
            if service_providers.is_empty() {
                return Err(anyhow!("No service providers yet"));
            }
            Ok(())
        },
        120,
    )
    .await
    .unwrap();

    let message_content: Vec<u8> = vec![0; CHUNK_SIZE * 2];
    let messages: Vec<MessageWithProvenance> = send_message(
        &alice.0,
        vec![bob.0.my_pub_key.clone(), carol.0.my_pub_key.clone()],
        message_content.clone(),
    )
    .await
    .unwrap();

    assert_eq!(messages.len(), 4);

    std::thread::sleep(Duration::from_secs(10));

    let decrypted_messages: Vec<DecryptedMessageOutput> = receive_messages(&bob.0).await.unwrap();

    assert_eq!(decrypted_messages.len(), 1);
    assert_eq!(decrypted_messages[0].contents, message_content);

    std::thread::sleep(Duration::from_secs(2));

    let decrypted_messages: Vec<DecryptedMessageOutput> = receive_messages(&bob.0).await.unwrap();

    assert_eq!(decrypted_messages.len(), 0);

    let messages = send_message(
        &bob.0,
        vec![alice.0.my_pub_key.clone(), carol.0.my_pub_key.clone()],
        vec![0; CHUNK_SIZE * 2],
    )
    .await
    .unwrap();

    assert_eq!(messages.len(), 4);

    let decrypted_messages = receive_messages(&carol.0).await.unwrap();
    assert_eq!(decrypted_messages.len(), 4);

    let messages: Vec<MessageWithProvenance> = send_message(
        &carol.0,
        vec![alice.0.my_pub_key.clone(), bob.0.my_pub_key.clone()],
        vec![0; CHUNK_SIZE * 2],
    )
    .await
    .unwrap();

    // Now two messages are necessary because players exchanged X25519 keys
    assert_eq!(messages.len(), 2);
}

async fn send_message(
    app_ws: &AppWebsocket,
    recipients: Vec<AgentPubKey>,
    message: MessageContents,
) -> anyhow::Result<Vec<MessageWithProvenance>> {
    let safehold_service_trait_service_id = safehold_service_trait::SAFEHOLD_SERVICE_HASH.to_vec();
    let messages: Vec<MessageWithProvenance> = app_ws
        .call_zome(
            ZomeCallTarget::RoleName("example".into()),
            "encrypted_messages".into(),
            "encrypt_message".into(),
            ExternIO::encode(EncryptMessageInput {
                recipients,
                message,
            })
            .unwrap(),
        )
        .await?
        .decode()?;

    let _response: () = make_service_request(
        &app_ws,
        safehold_service_trait_service_id.clone(),
        "store_messages".into(),
        messages.clone(),
    )
    .await?;

    Ok(messages)
}

async fn receive_messages(app_ws: &AppWebsocket) -> anyhow::Result<Vec<DecryptedMessageOutput>> {
    let safehold_service_trait_service_id = safehold_service_trait::SAFEHOLD_SERVICE_HASH.to_vec();
    let messages_outputs: Vec<MessageOutput> = make_service_request(
        app_ws,
        safehold_service_trait_service_id.clone(),
        "get_messages".into(),
        (),
    )
    .await?;

    let decrypted_messages: Vec<DecryptedMessageOutput> = app_ws
        .call_zome(
            ZomeCallTarget::RoleName("example".into()),
            "encrypted_messages".into(),
            "decrypt_messages".into(),
            ExternIO::encode(messages_outputs).unwrap(),
        )
        .await?
        .decode()?;

    Ok(decrypted_messages)
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
