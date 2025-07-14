use std::time::Duration;

mod common;
use anyhow::anyhow;
use common::*;
use holochain_client::{AgentPubKey, AppWebsocket, ExternIO, ZomeCallTarget};
use safehold_service_client::SafeholdServiceClient;
use safehold_service_provider::SERVICES_ROLE_NAME;
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

    client.create_clone_request(network_seed).await.unwrap();

    wait_for_providers(&alice.0).await.unwrap();

    let message_content: Vec<u8> = vec![0; 10];
    let messages: Vec<MessageWithProvenance> = send_message(
        &alice.0,
        vec![bob.0.my_pub_key.clone(), carol.0.my_pub_key.clone()],
        message_content.clone(),
    )
    .await
    .unwrap();

    assert_eq!(messages.len(), 2);

    wait_for_providers(&bob.0).await.unwrap();

    std::thread::sleep(Duration::from_secs(2));

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

    std::thread::sleep(Duration::from_secs(2));

    wait_for_providers(&carol.0).await.unwrap();

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

async fn wait_for_providers(app_ws: &AppWebsocket) -> anyhow::Result<()> {
    with_retries(
        async || {
            let safehold_service_trait_service_id =
                safehold_service_trait::SAFEHOLD_SERVICE_HASH.to_vec();

            let service_providers: Vec<AgentPubKey> = app_ws
                .call_zome(
                    ZomeCallTarget::RoleName(SERVICES_ROLE_NAME.into()),
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
        50,
    )
    .await
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

    client.create_clone_request(network_seed).await.unwrap();

    wait_for_providers(&alice.0).await.unwrap();

    let message_content: Vec<u8> = vec![0; CHUNK_SIZE * 2];
    let messages: Vec<MessageWithProvenance> = send_message(
        &alice.0,
        vec![bob.0.my_pub_key.clone(), carol.0.my_pub_key.clone()],
        message_content.clone(),
    )
    .await
    .unwrap();

    assert_eq!(messages.len(), 4);

    std::thread::sleep(Duration::from_secs(4));

    wait_for_providers(&bob.0).await.unwrap();

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

    std::thread::sleep(Duration::from_secs(2));

    wait_for_providers(&carol.0).await.unwrap();

    let decrypted_messages = receive_messages(&carol.0).await.unwrap();
    assert_eq!(decrypted_messages.len(), 2);

    let messages: Vec<MessageWithProvenance> = send_message(
        &carol.0,
        vec![alice.0.my_pub_key.clone(), bob.0.my_pub_key.clone()],
        vec![0; CHUNK_SIZE * 2],
    )
    .await
    .unwrap();

    // Now only two messages are necessary because players exchanged X25519 keys
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
