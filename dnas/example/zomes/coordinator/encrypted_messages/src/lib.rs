use std::collections::BTreeMap;

use hdk::prelude::*;
use locker_service_trait::MessageOutput;
use locker_types::MessageContents;
use utils::to_bytes;

mod utils;

#[derive(Serialize, Deserialize, Debug)]
pub struct EncryptMessageInput {
    pub recipients: Vec<AgentPubKey>,
    pub message: MessageContents,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EncryptedMessage {
    pub contents: MessageContents,
    pub recipients: BTreeMap<AgentPubKey, XSalsa20Poly1305EncryptedData>,
}

#[derive(Serialize, Deserialize, Debug, SerializedBytes)]
pub enum MessageEncryption {
    Secret {
        encrypted_secret: XSalsa20Poly1305EncryptedData,
        used_sender_key: X25519PubKey,
        new_sender_key: X25519PubKey,
        recipient_key: X25519PubKey,
    },
    SigningKey {
        sender_key: AgentPubKey,
        recipient_key: AgentPubKey,
        new_sender_key: X25519PubKey,
    },
}

pub struct PeerKeys {
    my_current_key: X25519PubKey,
    their_current_key: X25519PubKey,
}

fn query_peer_keys(recipient: AgentPubKey) -> ExternResult<Option<PeerKeys>> {}

#[hdk_extern]
pub fn encrypt_message(input: EncryptMessageInput) -> ExternResult<Vec<EncryptedMessage>> {
    let hash = hash_blake2b(input.message.clone(), 256)?;
    let key_ref = x_salsa20_poly1305_shared_secret_create_random(Some(hash.into()))?;

    let mut messages: Vec<EncryptedMessage> = vec![];

    let mut agent_keys: BTreeMap<AgentPubKey, XSalsa20Poly1305EncryptedData> = BTreeMap::new();

    let agent_info = agent_info()?;

    for recipient in input.recipients {
        let new_key = create_x25519_keypair()?;

        // TODO: Store new keypair

        if let Some(channel_keys) = query_peer_keys(recipient.clone())? {
            let encrypted_secret = x_salsa20_poly1305_shared_secret_export(
                channel_keys.my_current_key,
                channel_keys.their_current_key,
                key_ref.clone(),
            )?;

            let encrypted_message_secret = MessageEncryption::Secret {
                used_sender_key: channel_keys.my_current_key,
                new_sender_key: new_key,
                recipient_key: channel_keys.their_current_key,
                encrypted_secret,
            };
            let message_encryption_bytes = SerializedBytes::try_from(encrypted_message_secret)
                .map_err(|err| wasm_error!(err))?;

            let encrypted_data = ed_25519_x_salsa20_poly1305_encrypt(
                agent_info.agent_initial_pubkey.clone(),
                recipient.clone(),
                XSalsa20Poly1305Data::from(message_encryption_bytes.bytes().clone()),
            )?;

            agent_keys.insert(recipient, encrypted_data);
        } else {
            let encrypted_data = ed_25519_x_salsa20_poly1305_encrypt(
                agent_info.agent_initial_pubkey.clone(),
                recipient.clone(),
                XSalsa20Poly1305Data::from(input.message.clone()),
            )?;
            let encrypted_message_bytes = to_bytes(encrypted_data)?;

            let mut recipients: BTreeMap<AgentPubKey, XSalsa20Poly1305EncryptedData> =
                BTreeMap::new();

            let message_encryption = MessageEncryption::SigningKey {
                sender_key: agent_info.agent_initial_pubkey.clone(),
                new_sender_key: new_key,
                recipient_key: recipient.clone(),
            };
            let message_encryption_bytes =
                SerializedBytes::try_from(message_encryption).map_err(|err| wasm_error!(err))?;

            let encrypted_data = ed_25519_x_salsa20_poly1305_encrypt(
                agent_info.agent_initial_pubkey.clone(),
                recipient.clone(),
                XSalsa20Poly1305Data::from(message_encryption_bytes.bytes().clone()),
            )?;
            recipients.insert(recipient.clone(), encrypted_data);

            let encrypted_message = EncryptedMessage {
                contents: encrypted_message_bytes,
                recipients,
            };

            messages.push(encrypted_message);
        }
    }

    if agent_keys.len() > 0 {
        let encrypted_message = x_salsa20_poly1305_encrypt(
            key_ref.clone(),
            XSalsa20Poly1305Data::from(input.message.clone()),
        )?;
        let encrypted_message_bytes = to_bytes(encrypted_message)?;

        let store_encrypted_message_input = EncryptedMessage {
            contents: encrypted_message_bytes,
            recipients: agent_keys,
        };

        messages.push(store_encrypted_message_input);
    }

    Ok(messages)
}

#[hdk_extern]
pub fn receive_encrypted_messages(
    messages: Vec<MessageOutput>,
) -> ExternResult<Vec<MessageContents>> {
}
