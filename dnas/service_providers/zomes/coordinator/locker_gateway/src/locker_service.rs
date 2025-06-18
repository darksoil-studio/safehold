use hc_zome_traits::{implement_zome_trait_as_externs, implemented_zome_traits};
use hdk::prelude::*;
use locker_service_trait::{LockerService, MessageOutput, StoreMessageInput};
use locker_types::*;

#[implemented_zome_traits]
pub enum ZomeTraits {
    LockerService(LockerGateway),
}

pub struct LockerGateway;

#[implement_zome_trait_as_externs]
impl LockerService for LockerGateway {
    fn store_messages(inputs: Vec<StoreMessageInput>) -> ExternResult<()> {
        let sender = call_info()?.provenance;

        let input: Vec<CreateMessageInput> = inputs
            .into_iter()
            .map(|input| CreateMessageInput {
                message: Message {
                    sender: sender.clone(),
                    signature: input.signature,
                    contents: input.contents,
                },
                recipients: input.recipients,
            })
            .collect();

        let response = call(
            CallTargetCell::OtherRole(RoleName::from("locker")),
            ZomeName::from("locker"),
            FunctionName::from("create_messages"),
            None,
            input,
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
