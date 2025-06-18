use std::collections::BTreeMap;

use encrypted_messages_integrity::{EntryTypes, PeerKeys};
use hdk::prelude::*;
use peer_keys::query_peer_keys;
use utils::{create_relaxed, from_bytes, to_bytes};

use locker_service_trait::MessageOutput;
use locker_types::{AgentSpecificContents, Message, MessageContents, MessageWithProvenance};

mod peer_keys;
mod utils;

#[derive(Serialize, Deserialize, Debug)]
pub struct EncryptMessageInput {
    pub recipients: Vec<AgentPubKey>,
    pub message: MessageContents,
}

#[derive(Serialize, Deserialize, Debug, SerializedBytes)]
pub enum MessageEncryption {
    Secret {
        encrypted_secret: XSalsa20Poly1305EncryptedData,
        sender_encryption_key: X25519PubKey,
        recipient_encryption_key: X25519PubKey,
    },
    SigningKey {
        sender_key: AgentPubKey,
        recipient_key: AgentPubKey,
        sender_encryption_key: X25519PubKey,
    },
}

#[hdk_extern]
pub fn encrypt_message(input: EncryptMessageInput) -> ExternResult<Vec<MessageWithProvenance>> {
    let hash = hash_blake2b(input.message.clone(), 255)?;
    let key_ref = x_salsa20_poly1305_shared_secret_create_random(Some(hash.into()))?;

    let mut messages: Vec<MessageWithProvenance> = vec![];

    let mut agent_keys: BTreeMap<AgentPubKey, AgentSpecificContents> = BTreeMap::new();

    let agent_info = agent_info()?;

    for recipient in input.recipients {
        let new_key = create_x25519_keypair()?;

        let their_current_key = query_peer_keys(&recipient)?
            .map(|p| p.their_current_key)
            .flatten();

        let new_peer_keys = PeerKeys {
            peer: recipient.clone(),
            my_current_key: Some(new_key),
            their_current_key: their_current_key.clone(),
        };
        create_relaxed(EntryTypes::PeerKeys(new_peer_keys))?;

        if let Some(their_current_key) = their_current_key {
            let encrypted_secret = x_salsa20_poly1305_shared_secret_export(
                new_key,
                their_current_key,
                key_ref.clone(),
            )?;

            let encrypted_message_secret = MessageEncryption::Secret {
                sender_encryption_key: new_key,
                recipient_encryption_key: their_current_key,
                encrypted_secret,
            };
            let message_encryption_bytes = SerializedBytes::try_from(encrypted_message_secret)
                .map_err(|err| wasm_error!(err))?;

            let encrypted_data = ed_25519_x_salsa20_poly1305_encrypt(
                agent_info.agent_initial_pubkey.clone(),
                recipient.clone(),
                XSalsa20Poly1305Data::from(message_encryption_bytes.bytes().clone()),
            )?;

            agent_keys.insert(recipient, to_bytes(encrypted_data)?);
        } else {
            let encrypted_data = ed_25519_x_salsa20_poly1305_encrypt(
                agent_info.agent_initial_pubkey.clone(),
                recipient.clone(),
                XSalsa20Poly1305Data::from(input.message.clone()),
            )?;
            let encrypted_message_bytes = to_bytes(encrypted_data)?;

            let mut recipients: BTreeMap<AgentPubKey, AgentSpecificContents> = BTreeMap::new();

            let message_encryption = MessageEncryption::SigningKey {
                sender_key: agent_info.agent_initial_pubkey.clone(),
                sender_encryption_key: new_key,
                recipient_key: recipient.clone(),
            };
            let message_encryption_bytes =
                SerializedBytes::try_from(message_encryption).map_err(|err| wasm_error!(err))?;

            let encrypted_data = ed_25519_x_salsa20_poly1305_encrypt(
                agent_info.agent_initial_pubkey.clone(),
                recipient.clone(),
                XSalsa20Poly1305Data::from(message_encryption_bytes.bytes().clone()),
            )?;
            recipients.insert(recipient.clone(), to_bytes(encrypted_data)?);

            let encrypted_message = Message {
                contents: encrypted_message_bytes,
                recipients,
            };

            messages.push(sign_message(encrypted_message)?);
        }
    }

    if agent_keys.len() > 0 {
        let encrypted_message = x_salsa20_poly1305_encrypt(
            key_ref.clone(),
            XSalsa20Poly1305Data::from(input.message.clone()),
        )?;
        let encrypted_message_bytes = to_bytes(encrypted_message)?;

        let message = Message {
            contents: encrypted_message_bytes,
            recipients: agent_keys,
        };

        messages.push(sign_message(message)?);
    }

    Ok(messages)
}

fn sign_message(message: Message) -> ExternResult<MessageWithProvenance> {
    let my_pub_key = agent_info()?.agent_initial_pubkey;
    let signature = sign(my_pub_key.clone(), &message)?;

    Ok(MessageWithProvenance {
        provenance: my_pub_key,
        signature,
        message,
    })
}

#[hdk_extern]
pub fn decrypt_messages(messages: Vec<MessageOutput>) -> ExternResult<Vec<MessageContents>> {
    let mut decrypted_messages: Vec<MessageContents> = vec![];

    for message in messages {
        let result = decrypt_message(message);
        let Ok(decrypted_message) = result else {
            error!("Failed to decrypt message: {:?}", result);
            continue;
        };
        decrypted_messages.push(decrypted_message);
    }

    Ok(decrypted_messages)
}

fn decrypt_message(message: MessageOutput) -> ExternResult<MessageContents> {
    let agent_info = agent_info()?;

    let decrypted_data = ed_25519_x_salsa20_poly1305_decrypt(
        agent_info.agent_initial_pubkey.clone(),
        message.provenance.clone(),
        from_bytes(message.agent_specific_contents)?,
    )?;

    let bytes = SerializedBytes::from(UnsafeBytes::from(decrypted_data.as_ref().to_vec()));

    let encryption = MessageEncryption::try_from(bytes).map_err(|err| wasm_error!(err))?;

    let their_new_key = match &encryption {
        MessageEncryption::Secret {
            sender_encryption_key: sender_key,
            ..
        } => sender_key.clone(),
        MessageEncryption::SigningKey {
            sender_encryption_key: new_sender_key,
            ..
        } => new_sender_key.clone(),
    };

    let decrypted_message = match encryption {
        MessageEncryption::Secret {
            encrypted_secret,
            sender_encryption_key: sender_key,
            recipient_encryption_key: recipient_key,
        } => {
            let key_ref = x_salsa20_poly1305_shared_secret_ingest(
                recipient_key,
                sender_key,
                encrypted_secret,
                None,
            )?;
            let Some(decrypted_message) =
                x_salsa20_poly1305_decrypt(key_ref, from_bytes(message.message_contents)?)?
            else {
                return Err(wasm_error!("Failed to decrypt the message."));
            };
            decrypted_message.as_ref().to_vec()
        }
        MessageEncryption::SigningKey {
            sender_key,
            recipient_key,
            ..
        } => {
            let decrypted_message = ed_25519_x_salsa20_poly1305_decrypt(
                recipient_key,
                sender_key,
                from_bytes(message.message_contents)?,
            )?;

            decrypted_message.as_ref().to_vec()
        }
    };

    let their_current_key = query_peer_keys(&message.provenance)?
        .map(|p| p.their_current_key)
        .flatten();

    let changed_key = match their_current_key {
        Some(key) => key.ne(&their_new_key),
        None => true,
    };

    if changed_key {
        let new_peer_keys = PeerKeys {
            peer: message.provenance.clone(),
            my_current_key: None,
            their_current_key: Some(their_new_key.clone()),
        };
        create_relaxed(EntryTypes::PeerKeys(new_peer_keys))?;
    }

    Ok(decrypted_message)
}
