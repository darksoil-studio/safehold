use std::collections::BTreeMap;

use hdk::prelude::*;

pub type ChannelId = Vec<u8>;

pub struct ChannelKeys {
    pub my_pub_key: X25519PubKey,
    pub their_pub_key: X25519PubKey,
}

pub fn query_channel_keys(recipient: AgentPubKey) -> ExternResult<Option<ChannelKeys>> {}

pub fn query_last_channel_participants(
    channel_id: ChannelId,
) -> ExternResult<Option<BTreeMap<AgentPubKey, X25519PubKey>>> {
}
