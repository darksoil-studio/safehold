use hdi::prelude::*;

pub type MessageContent = Vec<u8>;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, SerializedBytes)]
pub struct Message {
    pub recipients: Vec<AgentPubKey>,
    pub content: MessageContent,
}

#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct MessageWithProvenance {
    pub provenance: AgentPubKey,
    pub signature: Signature,
    pub message: Message,
}
