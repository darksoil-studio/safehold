use std::collections::BTreeMap;

use hc_zome_traits::*;
use hdk::prelude::*;
use locker_types::{AgentSpecificContents, Message, MessageContents};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, SerializedBytes)]
pub struct StoreMessageInput {
    pub signature: Signature,
    pub contents: MessageContents,
    pub recipients: BTreeMap<AgentPubKey, Option<Vec<u8>>>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, SerializedBytes)]
pub struct MessageOutput {
    pub message: Message,
    pub agent_specific_contents: AgentSpecificContents,
}

#[zome_trait]
pub trait LockerService {
    fn store_message(message: StoreMessageInput) -> ExternResult<()>;

    fn get_messages(_: ()) -> ExternResult<Vec<MessageOutput>>;
}
