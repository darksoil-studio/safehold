use hc_zome_traits::*;
use hdk::prelude::*;
use locker_types::MessageWithProvenance;

#[zome_trait]
pub trait LockerService {
    fn store_message(message: MessageWithProvenance) -> ExternResult<()>;

    fn get_messages(_: ()) -> ExternResult<Vec<MessageWithProvenance>>;
}
