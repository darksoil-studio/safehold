use hc_zome_traits::{implement_zome_trait_as_externs, implemented_zome_traits};
use hdk::prelude::*;
use locker_service_trait::{LockerService, MessageOutput};
use locker_types::*;

#[implemented_zome_traits]
pub enum ZomeTraits {
    LockerService(LockerGateway),
}

pub struct LockerGateway;

#[implement_zome_trait_as_externs]
impl LockerService for LockerGateway {
    fn store_messages(messages: Vec<MessageWithProvenance>) -> ExternResult<()> {
        let sender = call_info()?.provenance;

        for message in &messages {
            if message.provenance.ne(&sender) {
                return Err(wasm_error!(
                    "Message provenance is not the caller of store_messages."
                ));
            }
        }

        let response = call(
            CallTargetCell::OtherRole(RoleName::from("locker")),
            ZomeName::from("locker"),
            FunctionName::from("create_messages"),
            None,
            messages,
        )?;
        let ZomeCallResponse::Ok(_) = response else {
            return Err(wasm_error!("Failed to store message: {response:?}"));
        };
        Ok(())
    }

    fn get_messages(_: ()) -> ExternResult<Vec<MessageOutput>> {
        let agent = call_info()?.provenance;
        let response = call(
            CallTargetCell::OtherRole(RoleName::from("locker")),
            ZomeName::from("locker"),
            FunctionName::from("get_messages_for_recipient"),
            None,
            agent,
        )?;
        let ZomeCallResponse::Ok(result) = response else {
            return Err(wasm_error!("Failed to get messages: {:?}"));
        };
        let messages: Vec<MessageOutput> = result.decode().map_err(|err| wasm_error!("{}", err))?;
        Ok(messages)
    }
}
