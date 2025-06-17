use std::collections::BTreeMap;

use hdi::prelude::*;

pub type MessageContents = Vec<u8>;
pub type AgentSpecificContents = Vec<u8>;

#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct Message {
    pub sender: AgentPubKey,
    pub signature: Signature,
    pub contents: MessageContents,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, SerializedBytes)]
pub struct CreateMessageInput {
    pub message: Message,
    pub recipients: BTreeMap<AgentPubKey, AgentSpecificContents>,
}
