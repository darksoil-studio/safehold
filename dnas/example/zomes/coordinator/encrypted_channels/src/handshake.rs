use hdk::prelude::*;
use locker_service_trait::StoreMessageInput;

use crate::channels::ChannelId;

// Build the initial message to send to this specific recipient, while we haven't
pub fn initial_handshake_message(
    channel_id: ChannelId,
    recipient: AgentPubKey,
) -> ExternResult<StoreMessageInput> {
}

pub fn query_my_handshake_key() -> ExternResult<X25519PubKey> {}

pub fn create_handshake_key() -> ExternResult<()> {}
