use hc_zome_traits::*;
use hdk::prelude::*;
use locker_types::{AgentSpecificContents, MessageContents, MessageWithProvenance};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, SerializedBytes)]
pub struct MessageOutput {
    pub provenance: AgentPubKey,
    pub message_contents: MessageContents,
    pub agent_specific_contents: AgentSpecificContents,
}

#[zome_trait]
pub trait LockerService {
    fn store_messages(message: Vec<MessageWithProvenance>) -> ExternResult<()>;

    fn get_messages(_: ()) -> ExternResult<Vec<MessageOutput>>;
}
