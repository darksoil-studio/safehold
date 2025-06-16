use std::time::Duration;

mod common;
use common::*;
use holochain_client::{AgentPubKey, ExternIO, SerializedBytes, ZomeCallTarget};
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

    std::thread::sleep(Duration::from_secs(2));

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

    let messages: Vec<MessageWithProvenance> = make_service_request(
        &recipient.0,
        locker_service_trait_service_id,
        "get_messages".into(),
        (),
    )
    .await
    .unwrap();

    assert_eq!(messages.len(), 0);
}
