use std::collections::BTreeMap;

use hdi::prelude::*;

pub type MessageContents = Vec<u8>;
pub type AgentSpecificContents = Vec<u8>;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, SerializedBytes)]
pub struct Message {
    pub contents: MessageContents,
    pub recipients: BTreeMap<AgentPubKey, AgentSpecificContents>,
}

#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct MessageWithProvenance {
    pub provenance: AgentPubKey,
    pub signature: Signature,
    pub message: Message,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EncryptMessageInput {
    pub recipients: Vec<AgentPubKey>,
    pub message: MessageContents,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DecryptedMessageOutput {
    pub provenance: AgentPubKey,
    pub contents: MessageContents,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProxiedCall {
    pub zome_name: ZomeName,
    pub fn_name: FunctionName,
    pub payload: ExternIO,
}
