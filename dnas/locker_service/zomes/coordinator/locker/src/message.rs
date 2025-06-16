use hdk::prelude::*;
use locker_integrity::*;

#[hdk_extern]
pub fn create_message(message: Message) -> ExternResult<Record> {
    let message_hash = create_entry(&EntryTypes::Message(message.clone()))?;
    for base in message.recipients.clone() {
        create_link(
            base,
            message_hash.clone(),
            LinkTypes::RecipientToMessages,
            (),
        )?;
    }
    let record = get(message_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find the newly created Message".to_string())
    ))?;
    Ok(record)
}

#[hdk_extern]
pub fn get_message(message_hash: ActionHash) -> ExternResult<Option<Record>> {
    let Some(details) = get_details(message_hash, GetOptions::default())? else {
        return Ok(None);
    };
    match details {
        Details::Record(details) => Ok(Some(details.record)),
        _ => Err(wasm_error!(WasmErrorInner::Guest(
            "Malformed get details response".to_string()
        ))),
    }
}

#[hdk_extern]
pub fn get_messages_for_recipient(recipient: AgentPubKey) -> ExternResult<Vec<Link>> {
    get_links(GetLinksInputBuilder::try_new(recipient, LinkTypes::RecipientToMessages)?.build())
}
