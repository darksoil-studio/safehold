use std::collections::BTreeMap;

use channels::{query_channel_keys, query_last_channel_participants, ChannelId};
use handshake::initial_handshake_message;
use hdk::prelude::*;
use locker_service_trait::StoreMessageInput;
use locker_types::{AgentSpecificContents, MessageContents};
use send_messages::send_messages;
use utils::to_bytes;

mod channels;
mod handshake;
mod send_messages;
mod utils;

#[derive(Serialize, Deserialize, Debug)]
pub struct SendEncryptedMessageInput {
    pub channel_id: ChannelId,
    pub recipients: Vec<AgentPubKey>,
    pub message: MessageContents,
}

#[derive(PartialEq, Eq, Serialize, Deserialize, SerializedBytes, Debug, Clone)]
pub struct EncryptedMessageSecret {
    pub encrypted_secret: XSalsa20Poly1305EncryptedData,
    pub used_sender_key: X25519PubKey,
    pub new_sender_key: X25519PubKey,
    pub recipient_key: X25519PubKey,
}

#[hdk_extern]
pub fn send_encrypted_message(input: SendEncryptedMessageInput) -> ExternResult<()> {
    let hash = hash_blake2b(input.message.clone(), 256)?;
    let key_ref = x_salsa20_poly1305_shared_secret_create_random(Some(hash.into()))?;

    let mut messages: Vec<StoreMessageInput> = vec![];

    let mut agent_keys: BTreeMap<AgentPubKey, AgentSpecificContents> = BTreeMap::new();

    let agent_info = agent_info()?;

    for recipient in input.recipients {
        let Some(channel_keys) = query_channel_keys(recipient.clone())? else {
            let handshake_message =
                initial_handshake_message(input.channel_id.clone(), recipient.clone())?;
            messages.push(handshake_message);

            let encrypted_data = ed_25519_x_salsa20_poly1305_encrypt(
                agent_info.agent_initial_pubkey.clone(),
                recipient.clone(),
                XSalsa20Poly1305Data::from(input.message.clone()),
            )?;
            let encrypted_message_bytes = to_bytes(encrypted_data)?;

            let signature = sign(
                agent_info.agent_initial_pubkey.clone(),
                &encrypted_message_bytes,
            )?;

            let mut recipients: BTreeMap<AgentPubKey, AgentSpecificContents> = BTreeMap::new();

            recipients.insert(recipient, vec![]);

            let store_encrypted_message_input = StoreMessageInput {
                signature,
                contents: encrypted_message_bytes,
                recipients,
            };

            messages.push(store_encrypted_message_input);

            continue;
        };

        let encrypted_secret = x_salsa20_poly1305_shared_secret_export(
            channel_keys.my_pub_key,
            channel_keys.their_pub_key,
            key_ref.clone(),
        )?;

        let new_key = create_x25519_keypair()?;

        // TODO: Store new keypair

        let encrypted_message_secret = EncryptedMessageSecret {
            used_sender_key: channel_keys.my_pub_key,
            new_sender_key: new_key,
            recipient_key: channel_keys.their_pub_key,
            encrypted_secret,
        };

        let bytes =
            SerializedBytes::try_from(encrypted_message_secret).map_err(|err| wasm_error!(err))?;

        agent_keys.insert(recipient, bytes.bytes().clone());
    }

    if agent_keys.len() > 0 {
        let encrypted_message = x_salsa20_poly1305_encrypt(
            key_ref.clone(),
            XSalsa20Poly1305Data::from(input.message.clone()),
        )?;
        let encrypted_message_bytes = to_bytes(encrypted_message)?;

        let signature = sign(agent_info.agent_initial_pubkey, &encrypted_message_bytes)?;

        let store_encrypted_message_input = StoreMessageInput {
            signature,
            contents: encrypted_message_bytes,
            recipients: agent_keys,
        };

        messages.push(store_encrypted_message_input);
    }

    send_messages(messages)?;

    Ok(())
}

#[hdk_extern]
pub fn get_messages(_: ()) -> ExternResult<Vec<MessageContents>> {}
