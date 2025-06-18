use hdk::prelude::*;

use locker_service_trait::StoreMessageInput;


pub fn send_messages(store_message_inputs: Vec<StoreMessageInput>) -> ExternResult<()> {
    
}

fn call_store_messages(store_message_inputs: Vec<StoreMessageInput>) -> ExternResult<()> {
    
    let response = call(CallTargetCell::OtherRole(get_locker_role()))?;

    let providers

    Ok(())
}

fn get_locker_role() -> String {
    match std::option_env!("SERVICE_PROVIDERS") {
        Some(locker_role) => locker_role.to_string(),
        None => String::from("service_providers"),
    }
}
