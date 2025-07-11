use std::collections::BTreeMap;

use chunks::query_pending_chunks;
use encrypted_messages_integrity::{Chunk, EntryTypes, MessageId, PeerKeys, UnitEntryTypes};
use hdk::prelude::*;
use peer_keys::query_peer_keys;
use utils::{create_relaxed, delete_relaxed, from_bytes, to_bytes};

use safehold_service_trait::MessageOutput;
use safehold_types::{
    AgentSpecificContents, DecryptedMessageOutput, EncryptMessageInput, Message, MessageContents,
    MessageWithProvenance,
};

mod chunks;
mod peer_keys;
mod utils;

pub const CHUNK_SIZE: usize = 50_000; // 50KB

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

fn new_message_id() -> ExternResult<Vec<u8>> {
    let bytes = random_bytes(8)?;
    Ok(bytes.to_vec())
}

#[hdk_extern]
pub fn encrypt_message(input: EncryptMessageInput) -> ExternResult<Vec<MessageWithProvenance>> {
    let key_ref = x_salsa20_poly1305_shared_secret_create_random(None)?;

    let mut messages: Vec<MessageWithProvenance> = vec![];

    let agent_info = agent_info()?;

    let message_id = new_message_id()?;

    let chunks: Vec<&[u8]> = input.message.chunks(CHUNK_SIZE).into_iter().collect();

    for (i, chunk_contents) in chunks.iter().enumerate() {
        let mut agent_keys: BTreeMap<AgentPubKey, AgentSpecificContents> = BTreeMap::new();
        let chunk = Chunk {
            provenance: agent_info.agent_initial_pubkey.clone(),
            message_id: message_id.clone(),
            chunk_index: i,
            total_chunk_number: chunks.len(),
            contents: chunk_contents.to_vec(),
        };
        let chunk_serialized_bytes =
            SerializedBytes::try_from(chunk).map_err(|err| wasm_error!(err))?;
        let chunk_bytes = chunk_serialized_bytes.bytes().to_vec();

        for recipient in input.recipients.clone() {
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
                    XSalsa20Poly1305Data::from(chunk_bytes.clone()),
                )?;
                let encrypted_message_bytes = to_bytes(encrypted_data)?;

                let mut recipients: BTreeMap<AgentPubKey, AgentSpecificContents> = BTreeMap::new();

                let message_encryption = MessageEncryption::SigningKey {
                    sender_key: agent_info.agent_initial_pubkey.clone(),
                    sender_encryption_key: new_key,
                    recipient_key: recipient.clone(),
                };
                let message_encryption_bytes = SerializedBytes::try_from(message_encryption)
                    .map_err(|err| wasm_error!(err))?;

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
                XSalsa20Poly1305Data::from(chunk_bytes),
            )?;
            let encrypted_message_bytes = to_bytes(encrypted_message)?;

            let message = Message {
                contents: encrypted_message_bytes,
                recipients: agent_keys,
            };

            messages.push(sign_message(message)?);
        }
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
pub fn decrypt_messages(messages: Vec<MessageOutput>) -> ExternResult<Vec<DecryptedMessageOutput>> {
    let pending_chunks = query_pending_chunks()?;
    let mut new_chunks: BTreeMap<(MessageId, HoloHash<hash_type::Agent>), Vec<Chunk>> =
        BTreeMap::new();

    for message in messages {
        let provenance = message.provenance.clone();
        let result = decrypt_message(message);
        let Ok(chunk) = result else {
            error!("Failed to decrypt message: {:?}", result);
            continue;
        };

        if chunk.provenance.ne(&provenance) {
            error!("Invalid provenance for chunk.");
            continue;
        }

        new_chunks
            .entry((chunk.message_id.clone(), chunk.provenance.clone()))
            .or_insert(Default::default())
            .push(chunk);
    }

    let mut decrypted_messages: Vec<DecryptedMessageOutput> = vec![];

    for ((message_id, provenance), new_chunks) in new_chunks {
        let pending_chunks = pending_chunks
            .get(&(message_id, provenance.clone()))
            .cloned()
            .unwrap_or(vec![]);

        let Some(chunk) = new_chunks.first() else {
            warn!("No chunks found while decrypting a message.");
            continue;
        };

        if chunk.total_chunk_number != new_chunks.len() + pending_chunks.len() {
            for chunk in new_chunks {
                create_relaxed(EntryTypes::Chunk(chunk))?;
            }
            continue;
        }

        let mut chunks: Vec<&Chunk> = new_chunks
            .iter()
            .chain(pending_chunks.iter().map(|c| &c.1))
            .collect();

        chunks.sort_by_key(|c| c.chunk_index);

        let all_chunks_found = chunks
            .iter()
            .enumerate()
            .all(|(i, chunk)| chunk.chunk_index == i);

        if !all_chunks_found {
            for chunk in new_chunks {
                create_relaxed(EntryTypes::Chunk(chunk))?;
            }
            continue;
        }

        for (chunk_hash, _pending_chunk) in &pending_chunks {
            delete_relaxed(chunk_hash.clone())?;
        }

        let all_bytes: Vec<u8> = chunks
            .into_iter()
            .map(|c| c.contents.clone())
            .flatten()
            .collect();
        decrypted_messages.push(DecryptedMessageOutput {
            provenance,
            contents: all_bytes,
        });
    }

    Ok(decrypted_messages)
}

fn decrypt_message(message: MessageOutput) -> ExternResult<Chunk> {
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

    let bytes = SerializedBytes::from(UnsafeBytes::from(decrypted_message));
    let chunk = Chunk::try_from(bytes)
        .map_err(|err| wasm_error!("Failed to deserialize chunk: {:?}", err))?;

    Ok(chunk)
}
